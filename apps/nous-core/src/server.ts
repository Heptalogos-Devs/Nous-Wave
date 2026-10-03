import { type CoreExecutionPolicy } from "./configuration-catalog.js";
import { ConfigurationService } from "@nous-wave/protocol/nous/wave/v1alpha1/configuration_pb.js";
import Fastify from "fastify";
import { grantMaintenance } from "./maintenance/grants.js";
import { ResourceRegistry } from "./resources/registry.js";
import { QueryOrchestrator } from "./query/orchestrator.js";
import { materializeResource } from "./resources/materialize.js";
import multipart from "@fastify/multipart";
import { fastifyConnectPlugin } from "@connectrpc/connect-fastify";
import {
  Code,
  ConnectError,
  type HandlerContext,
  type ServiceImpl,
} from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import { timestampDate } from "@bufbuild/protobuf/wkt";
import { timingSafeEqual } from "node:crypto";
import { Readable } from "node:stream";
import {
  SubjectService,
  CognitionService,
  MemoryService,
  MaterialService,
} from "@nous-wave/protocol/nous/wave/v1alpha1/services_pb.js";
import {
  IdentityService,
  ResolveIdentityResponseSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/identity_pb.js";
import {
  ResourceService,
  TopologyService,
  SystemService,
} from "@nous-wave/protocol/nous/wave/v1alpha1/management_pb.js";
import { compileNousQL } from "./nousql/compiler.js";
import {
  ProjectionRequestSchema,
  ProjectionSchema,
  ManagedContextResponseSchema,
  QueryRequestSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import type { KernelClient } from "./kernel-client.js";
import type { ConsumerPolicy } from "./domain.js";
import { ProjectionPlanner } from "./cognition/projection.js";
import { ContextCompiler } from "./cognition/context.js";
import { ModelRuntime } from "./model/runtime.js";
import { ModelService } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import { modelOperations } from "./model/operations.js";

export interface CoreOptions {
  kernel: KernelClient;
  token: string;
  consumers: ConsumerPolicy[];
  models?: ModelRuntime;
  resources?: ResourceRegistry;
  execution?: CoreExecutionPolicy;
}
const options = (context: HandlerContext) => ({
  signal: context.signal,
  timeoutMs: context.timeoutMs(),
});
export async function createCore(settings: CoreOptions) {
  if (settings.token.length < 32)
    throw new Error("Core credential must contain at least 32 characters");
  const app = Fastify({
    logger: false,
    bodyLimit: (settings.execution ?? settings.kernel.execution)
      .http_body_limit_bytes,
  });
  const kernel = settings.kernel;
  const materialLimits = await kernel.authority.getMaterialLimits({});
  const maxUploadBytes = Number(materialLimits.maxUploadBytes);
  if (!Number.isSafeInteger(maxUploadBytes) || maxUploadBytes < 1)
    throw new Error(
      "Kernel upload limit cannot be represented by the HTTP host",
    );
  const planner = new ProjectionPlanner(
    new Map(settings.consumers.map((p) => [p.consumerId, p])),
    settings.models,
  );
  const contexts = new ContextCompiler(kernel.execution.context_track_limit);
  const modelRuntime = settings.models ?? new ModelRuntime();
  const resourceRegistry = settings.resources ?? new ResourceRegistry({});
  const queries = new QueryOrchestrator(kernel, modelRuntime, resourceRegistry);
  app.addHook("onRequest", async (request, reply) => {
    const origin = request.headers.origin;
    if (origin && origin !== `http://${request.headers.host}`)
      return reply
        .code(403)
        .send({ code: "permission_denied", message: "Origin denied" });
    const supplied = Buffer.from(request.headers.authorization ?? "");
    const expected = Buffer.from(`Bearer ${settings.token}`);
    if (
      supplied.length !== expected.length ||
      !timingSafeEqual(supplied, expected)
    )
      return reply
        .code(401)
        .send({ code: "unauthenticated", message: "Core credential required" });
  });
  async function project(
    input: Parameters<typeof kernel.authority.buildContributionBatch>[0],
    context: HandlerContext,
  ) {
    const policy = planner.policy(input.consumerId ?? "");
    const request = create(ProjectionRequestSchema, input);
    const snapshot = await kernel.authority.getSession(
      { subjectId: request.subjectId, id: request.sessionId },
      { signal: context.signal },
    );
    if (
      request.activeWorkContextId &&
      request.activeWorkContextId !== snapshot.activeWorkContextId
    )
      throw new ConnectError(
        "Projection WorkContext is not active",
        Code.FailedPrecondition,
      );
    request.activeWorkContextId ??= snapshot.activeWorkContextId;
    request.maxItems = Math.min(
      request.maxItems || policy.maxItems,
      policy.maxItems,
    );
    request.maxTextBytes = Math.min(
      request.maxTextBytes || policy.maxTextBytes,
      policy.maxTextBytes,
    );
    const queryDegradation = [];
    if (request.query) {
      const query = await queries.execute(
        create(QueryRequestSchema, {
          subjectId: request.subjectId,
          sessionId: request.sessionId,
          expression: request.query,
        }),
        options(context),
      );
      request.situationRefs.push(
        ...query.hits.flatMap((h) => (h.reference ? [h.reference] : [])),
      );
      queryDegradation.push(...query.degradation);
      request.query = undefined;
    }
    const batch = await kernel.authority.buildContributionBatch(
      request,
      options(context),
    );
    batch.degradation.push(...queryDegradation);
    const projection = await planner.build(
      {
        projectionId: batch.projectionId,
        consumerId: batch.consumerId,
        sourceRuntimeRevision: batch.sourceRuntimeRevision,
        degradation: batch.degradation.map((d) => ({
          code: d.code,
          detail: d.detail,
        })),
        segments: batch.segments.map((s) => ({
          segmentId: s.segmentId,
          text: s.text,
          semanticRole: s.semanticRole,
          authority: s.authority,
          stability: s.stability,
          sourceRevision: s.sourceRevision,
          sourceRefs: s.sourceRefs.map((r) => ({
            kind: r.kind,
            value: r.value,
          })),
          evidence: s.evidence.map((e) => ({
            supportRole: e.supportRole,
            reference: e.reference
              ? { kind: e.reference.kind, value: e.reference.value }
              : undefined,
          })),
        })),
      },
      request,
      context.signal,
    );
    const after = await kernel.authority.getSession(
      { subjectId: request.subjectId, id: request.sessionId },
      { signal: context.signal },
    );
    if (after.runtimeRevision !== snapshot.runtimeRevision)
      throw new ConnectError("Session changed during projection", Code.Aborted);
    return projection;
  }
  const cognition: ServiceImpl<typeof CognitionService> = {
    grantMaintenance: (r, c) =>
      grantMaintenance(kernel, modelRuntime, r, options(c)),
    openSession: (r, c) => kernel.authority.openSession(r, options(c)),
    getSession: (r, c) => kernel.authority.getSession(r, options(c)),
    listSessions: (r, c) => kernel.authority.listSessions(r, options(c)),
    closeSession: (r, c) => kernel.authority.closeSession(r, options(c)),
    recordObservation: (r, c) =>
      kernel.authority.recordObservation(r, options(c)),
    query: async (r, c) => {
      let boundQuery: string | undefined;
      if (r.nousql !== undefined) {
        if (r.expression)
          throw new ConnectError(
            "Supply either NousQL or typed expression",
            Code.InvalidArgument,
          );
        const cognitiveTime = await kernel.authority.getCognitiveTime(
          { subjectId: r.subjectId },
          options(c),
        );
        const compiled = await compileNousQL(
          r.nousql,
          async (kind, locator) => {
            const result = await kernel.authority.resolveIdentity(
              {
                subjectId: r.subjectId,
                kind,
                locator: {
                  case: locator.kind === "name" ? "name" : "lexicalRef",
                  value: locator.value,
                },
              },
              options(c),
            );
            if (result.status !== "BOUND" || !result.candidates[0]?.canonical)
              throw new ConnectError(
                result.status,
                Code.InvalidArgument,
                undefined,
                [{ desc: ResolveIdentityResponseSchema, value: result }],
              );
            return {
              canonical: result.candidates[0].canonical,
              lexicalRef: result.candidates[0].lexicalRef,
            };
          },
          timestampDate(cognitiveTime),
        );
        r.expression = compiled.expression;
        r.nousql = undefined;
        boundQuery = compiled.boundCanonical;
      }
      const result = await queries.execute(r, options(c));
      for (const hit of result.hits) {
        if (
          hit.reference &&
          [
            "memory",
            "memory_revision",
            "cognitive_schema",
            "cognitive_schema_revision",
            "tag",
            "resource",
          ].includes(hit.reference.kind)
        ) {
          const binding = await kernel.authority.bindIdentity(
            { subjectId: r.subjectId, canonical: hit.reference },
            options(c),
          );
          hit.lexicalRef = binding.lexicalRef;
        }
      }
      result.boundQuery = boundQuery;
      return result;
    },
    reportUse: (r, c) => kernel.authority.reportUse(r, options(c)),
    createWorkContext: (r, c) =>
      kernel.authority.createWorkContext(r, options(c)),
    getWorkContext: (r, c) => kernel.authority.getWorkContext(r, options(c)),
    listWorkContexts: (r, c) =>
      kernel.authority.listWorkContexts(r, options(c)),
    updateWorkContext: (r, c) =>
      kernel.authority.updateWorkContext(r, options(c)),
    pauseWorkContext: (r, c) =>
      kernel.authority.pauseWorkContext(r, options(c)),
    resumeWorkContext: (r, c) =>
      kernel.authority.resumeWorkContext(r, options(c)),
    endWorkContext: (r, c) => kernel.authority.endWorkContext(r, options(c)),
    setActiveWorkContext: (r, c) =>
      kernel.authority.setActiveWorkContext(r, options(c)),
    buildProjection: async (r, c) =>
      create(ProjectionSchema, await project(r, c)),
    buildManagedContext: async (r, c) => {
      if (!r.projection)
        throw new ConnectError(
          "Projection request required",
          Code.InvalidArgument,
        );
      const projection = await project(r.projection, c);
      const session = await kernel.authority.getSession(
        { subjectId: r.projection.subjectId, id: r.projection.sessionId },
        options(c),
      );
      if (session.runtimeRevision !== projection.sourceRuntimeRevision)
        throw new ConnectError(
          "Session changed before context synchronization",
          Code.Aborted,
        );
      const policy = planner.policy(r.projection.consumerId);
      return create(
        ManagedContextResponseSchema,
        contexts.compile(
          {
            subjectId: r.projection.subjectId,
            sessionId: r.projection.sessionId,
            consumerId: r.projection.consumerId,
            workContextId:
              r.projection.activeWorkContextId ?? session.activeWorkContextId,
          },
          `${policy.revision}:${r.projection.maxItems}:${r.projection.maxTextBytes}`,
          projection,
          r.knownCursor,
        ),
      );
    },
  };
  const subjects: ServiceImpl<typeof SubjectService> = {
    createSubject: (r, c) => kernel.authority.createSubject(r, options(c)),
    getSubject: (r, c) => kernel.authority.getSubject(r, options(c)),
    listSubjects: (r, c) => kernel.authority.listSubjects(r, options(c)),
    getCognitiveSeed: (r, c) =>
      kernel.authority.getCognitiveSeed(r, options(c)),
    adoptCognitiveSeed: (r, c) =>
      kernel.authority.adoptCognitiveSeed(r, options(c)),
  };
  const memories: ServiceImpl<typeof MemoryService> = {
    setAccessibility: (r, c) =>
      kernel.authority.setAccessibility(r, options(c)),
    linkRevisions: (r, c) => kernel.authority.linkRevisions(r, options(c)),
    consolidateMemory: (r, c) =>
      kernel.authority.consolidateMemory(r, options(c)),
    getMemory: (r, c) => kernel.authority.getMemory(r, options(c)),
    listMemories: (r, c) => kernel.authority.listMemories(r, options(c)),
    getMemoryRevision: (r, c) =>
      kernel.authority.getMemoryRevision(r, options(c)),
    listMemoryRevisions: (r, c) =>
      kernel.authority.listMemoryRevisions(r, options(c)),
    formMemory: (r, c) => kernel.authority.formMemory(r, options(c)),
    reviseMemory: (r, c) => kernel.authority.reviseMemory(r, options(c)),
    suppressMemory: (r, c) => kernel.authority.suppressMemory(r, options(c)),
    restoreMemory: (r, c) => kernel.authority.restoreMemory(r, options(c)),
    withdrawMemory: (r, c) => kernel.authority.withdrawMemory(r, options(c)),
    reacceptMemory: (r, c) => kernel.authority.reacceptMemory(r, options(c)),
    purgeMemory: (r, c) => kernel.authority.purgeMemory(r, options(c)),
    createEpisode: (r, c) => kernel.authority.createEpisode(r, options(c)),
    getEpisode: (r, c) => kernel.authority.getEpisode(r, options(c)),
    getEpisodeRevision: (r, c) =>
      kernel.authority.getEpisodeRevision(r, options(c)),
    listEpisodes: (r, c) => kernel.authority.listEpisodes(r, options(c)),
    listEpisodeRevisions: (r, c) =>
      kernel.authority.listEpisodeRevisions(r, options(c)),
    reviseEpisode: (r, c) => kernel.authority.reviseEpisode(r, options(c)),
    linkEpisodeRevisions: (r, c) =>
      kernel.authority.linkEpisodeRevisions(r, options(c)),
    suppressEpisode: (r, c) => kernel.authority.suppressEpisode(r, options(c)),
    restoreEpisode: (r, c) => kernel.authority.restoreEpisode(r, options(c)),
    withdrawEpisode: (r, c) => kernel.authority.withdrawEpisode(r, options(c)),
    reacceptEpisode: (r, c) => kernel.authority.reacceptEpisode(r, options(c)),
    purgeEpisode: (r, c) => kernel.authority.purgeEpisode(r, options(c)),
    getJournal: (r, c) => kernel.authority.getJournal(r, options(c)),
    getJournalRevision: (r, c) =>
      kernel.authority.getJournalRevision(r, options(c)),
    listJournals: (r, c) => kernel.authority.listJournals(r, options(c)),
    listJournalRevisions: (r, c) =>
      kernel.authority.listJournalRevisions(r, options(c)),
    suppressJournal: (r, c) => kernel.authority.suppressJournal(r, options(c)),
    restoreJournal: (r, c) => kernel.authority.restoreJournal(r, options(c)),
    withdrawJournal: (r, c) => kernel.authority.withdrawJournal(r, options(c)),
    reacceptJournal: (r, c) => kernel.authority.reacceptJournal(r, options(c)),
    purgeJournal: (r, c) => kernel.authority.purgeJournal(r, options(c)),
  };
  const material: ServiceImpl<typeof MaterialService> = {
    getDerivedRegion: (r, c) =>
      kernel.authority.getDerivedRegion(r, options(c)),
    getProducer: (r, c) => kernel.authority.getProducer(r, options(c)),
    listDerivedRepresentations: (r, c) =>
      kernel.authority.listDerivedRepresentations(r, options(c)),
    getLimits: (r, c) => kernel.authority.getMaterialLimits(r, options(c)),
    getOccurrence: (r, c) => kernel.authority.getOccurrence(r, options(c)),
    getSourceRegion: (r, c) => kernel.authority.getSourceRegion(r, options(c)),
    getDerivedRepresentation: (r, c) =>
      kernel.authority.getDerivedRepresentation(r, options(c)),
    getArtifact: (r, c) => kernel.authority.getArtifact(r, options(c)),
    listArtifacts: (r, c) => kernel.authority.listArtifacts(r, options(c)),
    materializeEvidence: (r, c) =>
      kernel.authority.materializeEvidence(r, options(c)),
  };
  const identities: ServiceImpl<typeof IdentityService> = {
    bindIdentity: (r, c) => kernel.authority.bindIdentity(r, options(c)),
    resolveIdentity: (r, c) => kernel.authority.resolveIdentity(r, options(c)),
  };
  const resources: ServiceImpl<typeof ResourceService> = {
    materializeResource: (r, c) =>
      materializeResource(kernel, resourceRegistry, r, options(c)),
    putResource: (r, c) => kernel.authority.putResource(r, options(c)),
    getResource: (r, c) => kernel.authority.getResource(r, options(c)),
    listResources: (r, c) => kernel.authority.listResources(r, options(c)),
    removeResource: (r, c) => kernel.authority.removeResource(r, options(c)),
  };
  const topology: ServiceImpl<typeof TopologyService> = {
    createTag: (r, c) => kernel.authority.createTag(r, options(c)),
    getTag: (r, c) => kernel.authority.getTag(r, options(c)),
    listTags: (r, c) => kernel.authority.listTags(r, options(c)),
    createAssociation: (r, c) =>
      kernel.authority.createAssociation(r, options(c)),
    revokeAssociation: (r, c) =>
      kernel.authority.revokeAssociation(r, options(c)),
    getNeighborhood: (r, c) => kernel.authority.getNeighborhood(r, options(c)),
    rebindEntity: (r, c) => kernel.authority.rebindEntity(r, options(c)),
    createCognitiveSchema: (r, c) =>
      kernel.authority.createCognitiveSchema(r, options(c)),
    getCognitiveSchema: (r, c) =>
      kernel.authority.getCognitiveSchema(r, options(c)),
    addSchemaEvidence: (r, c) =>
      kernel.authority.addSchemaEvidence(r, options(c)),
    reviseCognitiveSchema: (r, c) =>
      kernel.authority.reviseCognitiveSchema(r, options(c)),
    splitCognitiveSchema: (r, c) =>
      kernel.authority.splitCognitiveSchema(r, options(c)),
    mergeCognitiveSchemas: (r, c) =>
      kernel.authority.mergeCognitiveSchemas(r, options(c)),
  };
  const system: ServiceImpl<typeof SystemService> = {
    getStatus: async (r, c) => {
      try {
        return await kernel.authority.getStatus(r, options(c));
      } catch (error) {
        if (c.signal.aborted) throw error;
        return {
          components: [
            {
              name: "kernel",
              state: "UNAVAILABLE",
              detail: "Kernel unavailable",
            },
          ],
        };
      }
    },
    getCapabilities: async (r, c) => {
      const status = await kernel.authority.getStatus(r, options(c));
      return {
        components: [
          ...status.components,
          ...(settings.models ?? new ModelRuntime()).invocations.capabilities,
        ],
      };
    },
    getProjectionStatus: (r, c) =>
      kernel.authority.getProjectionStatus(r, options(c)),
  };
  await app.register(fastifyConnectPlugin, {
    routes: (router) => {
      router.service(SubjectService, subjects);
      router.service(CognitionService, cognition);
      router.service(MemoryService, memories);
      router.service(MaterialService, material);
      router.service(IdentityService, identities);
      router.service(ResourceService, resources);
      router.service(TopologyService, topology);
      router.service(SystemService, system);
      router.service(ConfigurationService, {
        listConfigDescriptors: (r, c) =>
          kernel.configuration.listConfigDescriptors(r, options(c)),
        getConfigDescriptor: (r, c) =>
          kernel.configuration.getConfigDescriptor(r, options(c)),
        getConfiguration: (r, c) =>
          kernel.configuration.getConfiguration(r, options(c)),
        setSystemOverride: (r, c) =>
          kernel.configuration.setSystemOverride(r, options(c)),
        clearSystemOverride: (r, c) =>
          kernel.configuration.clearSystemOverride(r, options(c)),
        setSubjectOverride: (r, c) =>
          kernel.configuration.setSubjectOverride(r, options(c)),
        clearSubjectOverride: (r, c) =>
          kernel.configuration.clearSubjectOverride(r, options(c)),
      });
      router.service(ModelService, modelOperations(kernel, modelRuntime));
    },
    grpc: false,
    grpcWeb: false,
    readMaxBytes: 2 * 1024 * 1024,
    writeMaxBytes: 4 * 1024 * 1024,
  });
  await app.register(multipart, {
    limits: {
      files: 1,
      fields: 0,
      fileSize: maxUploadBytes,
    },
  });
  app.post<{ Params: { subjectId: string } }>(
    "/artifacts/:subjectId",
    async (request, reply) => {
      const file = await request.file();
      if (!file)
        return reply
          .code(400)
          .send({ code: "invalid_argument", message: "One file required" });
      async function* chunks() {
        yield {
          part: {
            case: "header" as const,
            value: {
              subjectId: request.params.subjectId,
              mediaType: file!.mimetype,
            },
          },
        };
        for await (const chunk of file!.file) {
          const bytes = Buffer.from(chunk);
          for (let offset = 0; offset < bytes.length; offset += 262144)
            yield {
              part: {
                case: "content" as const,
                value: bytes.subarray(offset, offset + 262144),
              },
            };
        }
        if (file!.file.truncated)
          throw new ConnectError(
            "Upload limit exceeded",
            Code.ResourceExhausted,
          );
      }
      const value = await kernel.artifacts.uploadArtifact(chunks(), {
        timeoutMs: 0,
      });
      return {
        artifactId: value.artifactId,
        subjectId: value.subjectId,
        contentHash: value.contentHash,
        byteLength: value.byteLength.toString(),
        mediaType: value.mediaType,
      };
    },
  );
  app.get<{ Params: { subjectId: string; artifactId: string } }>(
    "/artifacts/:subjectId/:artifactId",
    async (request, reply) => {
      const meta = await kernel.authority.getArtifact({
        subjectId: request.params.subjectId,
        id: request.params.artifactId,
      });
      const length = Number(meta.byteLength);
      let start = 0;
      let end = length;
      if (request.headers.range) {
        const match = /^bytes=(\d*)-(\d*)$/.exec(request.headers.range);
        if (!match || (!match[1] && !match[2]))
          return reply
            .code(416)
            .header("content-range", `bytes */${length}`)
            .send();
        if (!match[1]) start = Math.max(0, length - Number(match[2]));
        else {
          start = Number(match[1]);
          if (match[2]) end = Math.min(length, Number(match[2]) + 1);
        }
        if (
          !Number.isSafeInteger(start) ||
          !Number.isSafeInteger(end) ||
          start >= end
        )
          return reply
            .code(416)
            .header("content-range", `bytes */${length}`)
            .send();
        reply
          .code(206)
          .header("content-range", `bytes ${start}-${end - 1}/${length}`);
      }
      const stream = kernel.artifacts.downloadArtifact(
        {
          subjectId: request.params.subjectId,
          artifactId: request.params.artifactId,
          start: BigInt(start),
          end: BigInt(end),
        },
        { timeoutMs: 0 },
      );
      async function* bytes() {
        for await (const chunk of stream) yield Buffer.from(chunk.content);
      }
      return reply
        .header("accept-ranges", "bytes")
        .header("content-type", meta.mediaType)
        .header("content-length", end - start)
        .send(Readable.from(bytes()));
    },
  );
  return app;
}
