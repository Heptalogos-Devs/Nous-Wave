// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

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
  type Client,
  type ServiceImpl,
} from "@connectrpc/connect";
import { create, type DescService } from "@bufbuild/protobuf";
import { timestampDate } from "@bufbuild/protobuf/wkt";
import { timingSafeEqual } from "node:crypto";
import { Readable } from "node:stream";
import {
  SubjectService,
  CognitionService,
  RuntimeService,
  MemoryService,
  MaterialService,
} from "@nous-wave/protocol/nous/wave/v1alpha1/services_pb.js";
import {
  IdentityService,
  ResolveIdentityResponseSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/identity_pb.js";
import {
  ResourceService,
  ResourceRegistryService,
  ConceptService,
  SystemService,
} from "@nous-wave/protocol/nous/wave/v1alpha1/management_pb.js";
import { compileNousQL } from "./nousql/compiler.js";
import {
  ProjectionRequestSchema,
  ProjectionSchema,
  ManagedContextResponseSchema,
  QueryRequestSchema,
  type QueryRequest,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import type { KernelClient } from "./kernel-client.js";
import type { ConsumerPolicy } from "./domain.js";
import { ProjectionPlanner } from "./cognition/projection.js";
import { ContextCompiler } from "./cognition/context.js";
import { ModelRuntime } from "./model/runtime.js";
import { ModelService } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import { modelOperations } from "./model/operations.js";
import { configurationOperations } from "./configuration-service.js";

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
// Canonical owner RPCs preserve the incoming cancellation/deadline at the Core hop.
function forward<S extends DescService>(
  service: S,
  client: Client<S>,
): ServiceImpl<S> {
  return Object.fromEntries(
    service.methods.map((method) => {
      const invoke = client[method.localName] as (
        input: unknown,
        callOptions: ReturnType<typeof options>,
      ) => unknown;
      return [
        method.localName,
        (input: unknown, context: HandlerContext) =>
          invoke(input, options(context)),
      ];
    }),
  ) as ServiceImpl<S>;
}
export async function createCore(settings: CoreOptions) {
  if (settings.token.length < 32)
    throw new Error("Core credential must contain at least 32 characters");
  const app = Fastify({
    logger: false,
    bodyLimit: (settings.execution ?? settings.kernel.execution)
      .http_body_limit_bytes,
  });
  const kernel = settings.kernel;
  const materialLimits = await kernel.material.getLimits({});
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
    input: Parameters<typeof kernel.projection.buildContributionBatch>[0],
    context: HandlerContext,
  ) {
    const policy = planner.policy(input.consumerId ?? "");
    const request = create(ProjectionRequestSchema, input);
    const snapshot = await kernel.runtime.getSession(
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
    const batch = await kernel.projection.buildContributionBatch(
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
            basisRole: e.basisRole,
            reference: e.reference
              ? { kind: e.reference.kind, value: e.reference.value }
              : undefined,
          })),
        })),
      },
      request,
      context.signal,
    );
    const after = await kernel.runtime.getSession(
      { subjectId: request.subjectId, id: request.sessionId },
      { signal: context.signal },
    );
    if (after.runtimeRevision !== snapshot.runtimeRevision)
      throw new ConnectError("Session changed during projection", Code.Aborted);
    return projection;
  }
  async function bindQueryInput(r: QueryRequest, c: HandlerContext) {
    if (r.nousql !== undefined) {
      if (r.expression)
        throw new ConnectError(
          "Supply either NousQL or typed expression",
          Code.InvalidArgument,
        );
      const cognitiveTime = await kernel.queryWorkflow.getCognitiveTime(
        { subjectId: r.subjectId },
        options(c),
      );
      const compiled = await compileNousQL(
        r.nousql,
        async (kind, locator, asOf) => {
          const result = await kernel.identity.resolveIdentity(
            {
              subjectId: r.subjectId,
              kind,
              asOf: asOf
                ? {
                    seconds: BigInt(Math.floor(asOf.getTime() / 1000)),
                    nanos: (((asOf.getTime() % 1000) + 1000) % 1000) * 1000000,
                  }
                : undefined,
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
    }
    return r;
  }
  const cognition: ServiceImpl<typeof CognitionService> = {
    grantMaintenance: (r, c) =>
      grantMaintenance(kernel, modelRuntime, r, options(c)),
    prepareQuery: async (r, c) =>
      kernel.queryWorkflow.prepareQuery(
        { query: await bindQueryInput(r, c), reserveExecution: false },
        options(c),
      ),
    query: async (r, c) => {
      await bindQueryInput(r, c);
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
          const binding = await kernel.identity.bindIdentity(
            { subjectId: r.subjectId, canonical: hit.reference },
            options(c),
          );
          hit.lexicalRef = binding.lexicalRef;
        }
      }
      return result;
    },
    buildProjection: async (r, c) =>
      create(ProjectionSchema, await project(r, c)),
    buildManagedContext: async (r, c) => {
      if (!r.projection)
        throw new ConnectError(
          "Projection request required",
          Code.InvalidArgument,
        );
      const projection = await project(r.projection, c);
      const session = await kernel.runtime.getSession(
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
  const resources: ServiceImpl<typeof ResourceService> = {
    materializeResource: (r, c) =>
      materializeResource(kernel, resourceRegistry, r, options(c)),
  };
  const system: ServiceImpl<typeof SystemService> = {
    getStatus: async (r, c) => {
      try {
        return await kernel.system.getStatus(r, options(c));
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
      const status = await kernel.system.getStatus(r, options(c));
      return {
        components: [
          ...status.components,
          ...(settings.models ?? new ModelRuntime()).invocations.capabilities,
        ],
      };
    },
    getProjectionStatus: (r, c) =>
      kernel.system.getProjectionStatus(r, options(c)),
  };
  await app.register(fastifyConnectPlugin, {
    routes: (router) => {
      router.service(SubjectService, forward(SubjectService, kernel.subjects));
      router.service(CognitionService, cognition);
      router.service(MemoryService, forward(MemoryService, kernel.memory));
      router.service(
        MaterialService,
        forward(MaterialService, kernel.material),
      );
      router.service(
        IdentityService,
        forward(IdentityService, kernel.identity),
      );
      router.service(ResourceService, resources);
      router.service(ConceptService, forward(ConceptService, kernel.concepts));
      router.service(SystemService, system);
      router.service(
        ConfigurationService,
        configurationOperations(
          forward(ConfigurationService, kernel.configuration),
        ),
      );
      router.service(RuntimeService, forward(RuntimeService, kernel.runtime));
      router.service(
        ResourceRegistryService,
        forward(ResourceRegistryService, kernel.resourceRegistry),
      );
      router.service(ModelService, modelOperations(kernel, modelRuntime));
    },
    grpc: false,
    grpcWeb: false,
    readMaxBytes: kernel.execution.http_body_limit_bytes,
    writeMaxBytes: kernel.execution.public_rpc_response_max_bytes,
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
      const meta = await kernel.material.getArtifact({
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
