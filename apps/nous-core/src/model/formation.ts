import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import { create, fromJson, toJson, type JsonValue } from "@bufbuild/protobuf";
import { FormMemoryRequestSchema } from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { type FormationRequest } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import type { ModelRuntime } from "./runtime.js";
import type { ModelRoleSnapshot } from "./invocations.js";
import { canonicalDigest } from "../digest.js";
import { z } from "zod";

const snapshotSchema = z.strictObject({
  cognitive_formed_at: z.string(),
  model: z.unknown(),
  representationId: z.string().optional(),
  candidates: z
    .array(
      z.strictObject({
        key: z.string(),
        surface: z.string(),
        entityRef: z.string(),
      }),
    )
    .max(128),
});
const proposalSchema = z.strictObject({
  request: z.unknown(),
});
const outcomeSchema = z.union([
  z.strictObject({ purged: z.literal(true) }),
  z.strictObject({
    revisionId: z.string(),
  }),
]);

export async function formObservation(
  kernel: KernelClient,
  models: ModelRuntime,
  r: FormationRequest,
  options: CallOptions,
) {
  if (
    !z.string().uuid().safeParse(r.operationId).success ||
    r.operationId === "00000000-0000-0000-0000-000000000000"
  )
    throw new ConnectError(
      "Caller-stable operation_id is required",
      Code.InvalidArgument,
    );
  r.operationId = r.operationId.toLowerCase();
  r.subjectId = r.subjectId.toLowerCase();
  r.occurrenceId = r.occurrenceId.toLowerCase();
  if (r.representationId) r.representationId = r.representationId.toLowerCase();
  const mode = r.aboutnessMode || "select_from_resolved_mentions";
  if (
    !["explicit", "select_from_resolved_mentions", "none"].includes(mode) ||
    (mode !== "explicit" && r.explicitAboutness.length)
  )
    throw new ConnectError("Invalid aboutness policy", Code.InvalidArgument);
  if (
    new Set(r.explicitAboutness).size !== r.explicitAboutness.length ||
    r.explicitAboutness.length > 128
  )
    throw new ConnectError(
      "Duplicate/oversized aboutness",
      Code.InvalidArgument,
    );
  const identity = {
    subjectId: r.subjectId,
    owner: "memory",
    operationKey: r.operationId,
    semanticDigest: canonicalDigest({
      subject: r.subjectId,
      occurrence: r.occurrenceId,
      representation: r.representationId,
      mode,
      aboutness: [...r.explicitAboutness].sort(),
    }),
  };
  const replay = async (text: string) => {
    const outcome = outcomeSchema.parse(JSON.parse(text));
    if ("purged" in outcome)
      throw new ConnectError("Formation outcome was purged", Code.NotFound);
    const memory = await kernel.authority.getMemoryRevision(
      { subjectId: r.subjectId, id: outcome.revisionId },
      options,
    );
    return {
      memory,
      degradation: [],
    };
  };
  const previous = await kernel.modelMaterial.findWorkflow(identity, options);
  if (previous.outcomeJson) return replay(previous.outcomeJson);
  let snapshotText = previous.snapshotJson;
  if (!snapshotText) {
    const model = models.invocations.snapshot("memory_formation");
    const occurrence = await kernel.authority.getOccurrence(
      { subjectId: r.subjectId, id: r.occurrenceId },
      options,
    );
    let representationId = r.representationId;
    if (!representationId && occurrence.artifactId) {
      const artifact = await kernel.authority.getArtifact(
        { subjectId: r.subjectId, id: occurrence.artifactId },
        options,
      );
      const mime = artifact.mediaType.split(";")[0]!.trim().toLowerCase();
      const kind = mime.startsWith("image/")
        ? "image_description"
        : mime.startsWith("audio/")
          ? models.audio.input_mode === "direct"
            ? "audio_description"
            : "transcript"
          : mime.startsWith("video/")
            ? "scene_description"
            : "extracted_text";
      const available = await kernel.authority.listDerivedRepresentations(
        {
          subjectId: r.subjectId,
          artifactId: occurrence.artifactId,
          kind,
          limit: 1,
        },
        options,
      );
      representationId = available.items[0]?.representationId;
    }
    const candidates =
      mode === "select_from_resolved_mentions"
        ? (
            await kernel.modelMaterial.getResolvedMentions(
              { subjectId: r.subjectId, occurrenceId: r.occurrenceId },
              options,
            )
          ).candidates.map((candidate) => ({
            key: candidate.key,
            surface: candidate.surface,
            entityRef: candidate.entityRef,
          }))
        : [];
    snapshotText = JSON.stringify({ model, representationId, candidates });
  }
  const reservation = await kernel.modelMaterial.reserveWorkflow(
    { ...identity, snapshotJson: snapshotText },
    options,
  );
  if (reservation.outcomeJson) return replay(reservation.outcomeJson);
  if (reservation.busy || !reservation.leaseToken)
    throw new ConnectError(
      "Formation workflow is busy; retry the same operation ID",
      Code.Aborted,
    );
  const lease = {
    subjectId: r.subjectId,
    owner: "memory",
    operationKey: r.operationId,
    leaseToken: reservation.leaseToken,
  };
  try {
    let proposed = reservation.proposalJson
      ? proposalSchema.parse(JSON.parse(reservation.proposalJson))
      : undefined;
    if (!proposed) {
      const snapshot = snapshotSchema.parse(
        JSON.parse(reservation.snapshotJson),
      );
      const source = await kernel.authority.materializeEvidence(
        {
          subjectId: r.subjectId,
          reference: snapshot.representationId
            ? {
                kind: "derived_representation",
                value: snapshot.representationId,
              }
            : { kind: "occurrence", value: r.occurrenceId },
          maxBytes: 32768n,
        },
        options,
      );
      if (source.partial)
        throw new ConnectError(
          "Formation source is too large; select a bounded representation",
          Code.ResourceExhausted,
        );
      const mime = source.mediaType.split(";")[0]!.toLowerCase();
      if (!mime.startsWith("text/") && mime !== "application/json")
        throw new ConnectError(
          "material_representation_required",
          Code.FailedPrecondition,
        );
      const text = new TextDecoder("utf-8", { fatal: true }).decode(
        source.content,
      );
      const result = await models.form(
        JSON.stringify({
          producerMetadata: text,
          resolvedEntityCandidates: snapshot.candidates,
          aboutnessMode: mode,
        }),
        options.signal ?? undefined,
        snapshot.model as ModelRoleSnapshot,
      );
      if (
        new Set(result.selectedEntityKeys).size !==
          result.selectedEntityKeys.length ||
        result.selectedEntityKeys.some(
          (key) =>
            !snapshot.candidates.some((candidate) => candidate.key === key),
        )
      )
        throw new ConnectError(
          "Model selected an invalid entity candidate",
          Code.FailedPrecondition,
        );
      const aboutness =
        mode === "explicit"
          ? r.explicitAboutness
          : mode === "none"
            ? []
            : [
                ...new Set(
                  snapshot.candidates
                    .filter((candidate) =>
                      result.selectedEntityKeys.includes(candidate.key),
                    )
                    .map((candidate) => candidate.entityRef),
                ),
              ];
      const request = create(FormMemoryRequestSchema, {
        operationId: r.operationId,
        subjectId: r.subjectId,
        input: {
          cognitiveRole: "declarative",
          formationMode: "grounded",
          groundingOccurrenceId: r.occurrenceId,
          semanticRole: result.semanticRole,
          text: result.text,
          title: result.title,
          epistemicClass: "derived",
          aboutness,
          producer: {
            providerClass: result.producerMetadata.protocol,
            operation: "memory_formation_text",
            implementation: "ai-sdk@7.0.102/openai@4.0.67",
            modelIdentity: result.producerMetadata.model,
            modelRevision: result.producerMetadata.modelRevision,
            outputSchemaDigest: result.producerMetadata.outputSchemaDigest,
            preprocessingIdentity: result.producerMetadata.promptId!,
            preprocessingRevision: result.producerMetadata.promptDigest!,
            configDigest: result.producerMetadata.configDigest,
          },
          supports: [
            {
              support: {
                case: "evidence",
                value: {
                  occurrenceId: r.occurrenceId,
                  supportRole: "interpretation",
                  locator: snapshot.representationId
                    ? {
                        case: "derivedRepresentationId",
                        value: snapshot.representationId,
                      }
                    : { case: "wholeOccurrence", value: true },
                },
              },
            },
          ],
        },
      });
      proposed = {
        request: toJson(FormMemoryRequestSchema, request),
      };
      await kernel.modelMaterial.saveWorkflow(
        { ...lease, proposalJson: JSON.stringify(proposed) },
        options,
      );
    }
    const request = fromJson(
      FormMemoryRequestSchema,
      proposed.request as JsonValue,
    );
    const memory = await kernel.authority.formMemory(request, options);
    await kernel.modelMaterial.saveWorkflow(
      {
        ...lease,
        outcomeJson: JSON.stringify({
          revisionId: memory.revisionId,
        }),
      },
      options,
    );
    return {
      memory,
      degradation: [],
    };
  } finally {
    await kernel.modelMaterial
      .releaseWorkflow(lease, {
        timeoutMs: kernel.execution.workflow_ack_timeout_ms,
      })
      .catch(() => {});
  }
}
