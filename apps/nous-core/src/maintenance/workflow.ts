import { consolidationRequest } from "./consolidation.js";
import { consolidationSchema } from "../model/schemas/consolidation.js";
import { CommitLongitudinalConsolidationRequestSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/consolidation_pb.js";
import { createHash } from "node:crypto";
import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import { create, fromJson, toJson, type JsonValue } from "@bufbuild/protobuf";
import { z } from "zod";
import {
  MaintenancePlanSchema,
  ApplyEpisodePartitionRequestSchema,
  CommitJournalRequestSchema,
  type MaintenanceNeed,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import {
  ProducerSignatureSchema,
  type ProducerSignature,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { GenerationFailure } from "../model/invocations.js";
import type { KernelClient } from "../kernel-client.js";
import type { ModelRuntime } from "../model/runtime.js";
import type {
  ModelRoleSnapshot,
  ModelProducerMetadata,
} from "../model/invocations.js";
import {
  partitionIndices,
  journalSupportKeys,
  episodePartitionSchema,
  journalSynthesisSchema,
} from "../model/schemas/longitudinal.js";
import { canonicalDigest } from "../digest.js";

const outcomeSchema = z.strictObject({
  status: z.enum(["committed", "no_change", "obsolete", "rejected_invalid"]),
  problemCode: z.string().optional(),
});
function readOutcome(text: string) {
  const value: unknown = JSON.parse(text);
  if (z.strictObject({ purged: z.literal(true) }).safeParse(value).success)
    return { status: "obsolete" as const };
  return outcomeSchema.parse(value);
}
const snapshotSchema = z.strictObject({
  plan: z.unknown(),
  model: z.unknown(),
  cognitive_formed_at: z.string(),
});
const proposalSchema = z.discriminatedUnion("action", [
  z.strictObject({ action: z.literal("partition"), request: z.unknown() }),
  z.strictObject({ action: z.literal("journal"), request: z.unknown() }),
  z.strictObject({ action: z.literal("consolidation"), request: z.unknown() }),
  z.strictObject({
    action: z.literal("withdraw"),
    journalId: z.string().uuid(),
    epoch: z.string(),
  }),
  z.strictObject({ action: z.literal("no_change") }),
]);

export function maintenanceOperationId(need: MaintenanceNeed) {
  const bytes = createHash("sha1")
    .update(Buffer.from("6ba7b8129dad11d180b400c04fd430c8", "hex"))
    .update(`${need.needId}:${need.triggerAuthoritySeq}`)
    .digest()
    .subarray(0, 16);
  bytes[6] = (bytes[6]! & 15) | 80;
  bytes[8] = (bytes[8]! & 63) | 128;
  const value = bytes.toString("hex");
  return `${value.slice(0, 8)}-${value.slice(8, 12)}-${value.slice(12, 16)}-${value.slice(16, 20)}-${value.slice(20)}`;
}
function producer(
  metadata: ModelProducerMetadata,
  operation: string,
): Omit<ProducerSignature, "$typeName"> {
  return {
    signatureHash: "",
    providerClass: metadata.protocol,
    operation,
    implementation: metadata.implementation,
    modelIdentity: metadata.model,
    modelRevision: metadata.modelRevision,
    outputSchemaDigest: metadata.outputSchemaDigest,
    preprocessingIdentity: metadata.promptId!,
    preprocessingRevision: metadata.promptDigest!,
    configDigest: metadata.configDigest,
  };
}

export async function runModelMaintenance(
  kernel: KernelClient,
  models: ModelRuntime,
  need: MaintenanceNeed,
  options: CallOptions,
  reserveModelCall: () => void,
) {
  const operationId = maintenanceOperationId(need);
  const identity = {
    subjectId: need.subjectId,
    owner: "memory",
    operationKey: operationId,
    semanticDigest: canonicalDigest({
      needId: need.needId,
      trigger: need.triggerAuthoritySeq.toString(),
      kind: need.kind,
      scopeKind: need.scopeKind,
      scopeRef: need.scopeRef,
    }),
  };
  const previous = await kernel.modelMaterial.findWorkflow(identity, options);
  if (previous.outcomeJson) return readOutcome(previous.outcomeJson);
  let snapshotJson = previous.snapshotJson;
  if (!snapshotJson) {
    const plan = await kernel.authority.planMaintenance(
      { claimed: need },
      options,
    );
    if (plan.status === "obsolete") return { status: "obsolete" as const };
    if (plan.status === "deferred")
      return {
        status: "blocked_dependency" as const,
        problemCode: plan.problemCode ?? "source_not_settled",
        nextDue: plan.nextDue,
      };
    const role =
      need.kind === "episode_resegment"
        ? "episode_segmentation"
        : need.kind === "memory_consolidate"
          ? "memory_consolidation"
          : "journal_synthesis";
    const model =
      plan.status === "withdraw" ? null : models.invocations.snapshot(role);
    snapshotJson = JSON.stringify({
      plan: toJson(MaintenancePlanSchema, plan),
      model,
    });
  }
  const reservation = await kernel.modelMaterial.reserveWorkflow(
    { ...identity, snapshotJson },
    options,
  );
  if (reservation.outcomeJson) return readOutcome(reservation.outcomeJson);
  if (reservation.busy || !reservation.leaseToken)
    return {
      status: "blocked_dependency" as const,
      problemCode: "workflow_busy",
    };
  const lease = {
    subjectId: need.subjectId,
    owner: "memory",
    operationKey: operationId,
    leaseToken: reservation.leaseToken,
  };
  try {
    const snapshot = snapshotSchema.parse(JSON.parse(reservation.snapshotJson));
    const plan = fromJson(MaintenancePlanSchema, snapshot.plan as JsonValue);
    let proposed = reservation.proposalJson
      ? proposalSchema.parse(JSON.parse(reservation.proposalJson))
      : undefined;
    if (!proposed) {
      if (plan.status === "withdraw") {
        if (!plan.target)
          throw new ConnectError(
            "Journal withdrawal requires a bound target",
            Code.InvalidArgument,
          );
        proposed = {
          action: "withdraw",
          journalId: plan.target.journalId,
          epoch: plan.target.expectedEpoch.toString(),
        };
      } else {
        reserveModelCall();
        if (need.kind === "episode_resegment") {
          const result = await models.segmentEpisode(
            JSON.stringify(snapshot.plan),
            options.signal ?? undefined,
            snapshot.model as ModelRoleSnapshot,
          );
          let segments;
          try {
            segments = partitionIndices(
              episodePartitionSchema.parse(result.value),
              plan.members.map((member) => member.key),
            );
          } catch {
            throw new ConnectError(
              "Episode partition proposal violates the member catalog",
              Code.InvalidArgument,
            );
          }
          if (!segments) {
            const existing = new Set(
              plan.episodes.flatMap(
                (episode) =>
                  episode.currentRevision?.members.map(
                    (member) => member.reference?.value,
                  ) ?? [],
              ),
            );
            if (
              plan.members.some((member) => !existing.has(member.occurrenceId))
            )
              throw new ConnectError(
                "No-change proposal omitted unorganized experience",
                Code.InvalidArgument,
              );
          }
          proposed = segments
            ? {
                action: "partition",
                request: toJson(ApplyEpisodePartitionRequestSchema, {
                  $typeName:
                    "nous.wave.kernel.v1alpha1.ApplyEpisodePartitionRequest",
                  operationId,
                  subjectId: need.subjectId,
                  expectedAuthoritySeq: plan.authoritySeq,
                  sources: plan.sources,
                  orderedOccurrences: plan.members.map(
                    (member) => member.occurrenceId,
                  ),
                  segments: segments.map((segment) => ({
                    $typeName:
                      "nous.wave.kernel.v1alpha1.EpisodePartitionSegment" as const,
                    ...segment,
                  })),
                }),
              }
            : { action: "no_change" };
          if (proposed.action === "partition") {
            const request = fromJson(
              ApplyEpisodePartitionRequestSchema,
              proposed.request as JsonValue,
            );
            request.producer = create(
              ProducerSignatureSchema,
              producer(result.producerMetadata, "episode_segmentation_text"),
            );
            proposed.request = toJson(
              ApplyEpisodePartitionRequestSchema,
              request,
            );
          }
        } else if (need.kind === "memory_consolidate") {
          const result = await models.consolidate(
            JSON.stringify(snapshot.plan),
            options.signal ?? undefined,
            snapshot.model as ModelRoleSnapshot,
          );
          const request = consolidationRequest(
            plan,
            consolidationSchema.parse(result.value),
            operationId,
            create(
              ProducerSignatureSchema,
              producer(result.producerMetadata, "memory_consolidation_text"),
            ),
          );
          proposed = {
            action: "consolidation",
            request: toJson(
              CommitLongitudinalConsolidationRequestSchema,
              request,
            ),
          };
        } else {
          const result = await models.synthesizeJournal(
            JSON.stringify(snapshot.plan),
            options.signal ?? undefined,
            snapshot.model as ModelRoleSnapshot,
          );
          const proposal = journalSynthesisSchema.parse(result.value);
          try {
            journalSupportKeys(
              proposal,
              new Set(plan.supports.map((entry) => entry.key)),
            );
          } catch {
            throw new ConnectError(
              "Journal proposal violates the support catalog",
              Code.InvalidArgument,
            );
          }
          if (proposal.action === "no_change" && plan.target)
            throw new ConnectError(
              "Journal revalidation requires a supported commit",
              Code.InvalidArgument,
            );
          if (proposal.action === "no_change")
            proposed = { action: "no_change" };
          else if (proposal.action === "withdraw") {
            if (!plan.target)
              throw new ConnectError(
                "Journal withdrawal requires an existing scope",
                Code.InvalidArgument,
              );
            proposed = {
              action: "withdraw",
              journalId: plan.target.journalId,
              epoch: plan.target.expectedEpoch.toString(),
            };
          } else {
            const supports = new Map(
              plan.supports.map((entry) => [entry.key, entry.support!]),
            );
            proposed = {
              action: "journal",
              request: toJson(CommitJournalRequestSchema, {
                $typeName: "nous.wave.kernel.v1alpha1.CommitJournalRequest",
                operationId,
                subjectId: need.subjectId,
                expectedAuthoritySeq: plan.authoritySeq,
                target: plan.target,
                sources: plan.sources,
                title: proposal.title ?? undefined,
                narrative: proposal.narrative,
                points: proposal.points.map((point, ordinal) => ({
                  $typeName: "nous.wave.v1alpha1.JournalPoint" as const,
                  ordinal,
                  role: point.role,
                  text: point.text,
                  supports: point.supportKeys.map((key) => supports.get(key)!),
                })),
                producer: {
                  $typeName: "nous.wave.v1alpha1.ProducerSignature",
                  ...producer(
                    result.producerMetadata,
                    "journal_synthesis_text",
                  ),
                },
              }),
            };
          }
        }
      }
      await kernel.modelMaterial.saveWorkflow(
        { ...lease, proposalJson: JSON.stringify(proposed) },
        options,
      );
    }
    if (proposed.action === "partition")
      await kernel.authority.applyEpisodePartition(
        fromJson(
          ApplyEpisodePartitionRequestSchema,
          proposed.request as JsonValue,
        ),
        options,
      );
    else if (proposed.action === "journal")
      await kernel.authority.commitJournal(
        fromJson(CommitJournalRequestSchema, proposed.request as JsonValue),
        options,
      );
    else if (proposed.action === "consolidation") {
      const result = await kernel.authority.commitLongitudinalConsolidation(
        fromJson(
          CommitLongitudinalConsolidationRequestSchema,
          proposed.request as JsonValue,
        ),
        options,
      );
      const outcome = {
        status:
          result.status === "no_change"
            ? ("no_change" as const)
            : ("committed" as const),
      };
      await kernel.modelMaterial.saveWorkflow(
        { ...lease, outcomeJson: JSON.stringify(outcome) },
        options,
      );
      return outcome;
    } else if (proposed.action === "withdraw")
      await kernel.authority.withdrawJournal(
        {
          operationId,
          subjectId: need.subjectId,
          journalId: proposed.journalId,
          expectedObjectEpoch: BigInt(proposed.epoch),
        },
        options,
      );
    if (
      (proposed.action === "partition" && need.scopeKind === "track") ||
      (proposed.action === "journal" && need.kind === "journal_review")
    )
      await kernel.authority.refreshMaintenance({ claimed: need }, options);
    const outcome = {
      status:
        proposed.action === "no_change"
          ? ("no_change" as const)
          : ("committed" as const),
    };
    await kernel.modelMaterial.saveWorkflow(
      { ...lease, outcomeJson: JSON.stringify(outcome) },
      options,
    );
    return outcome;
  } catch (error) {
    if (
      (error instanceof ConnectError && error.code === Code.InvalidArgument) ||
      error instanceof z.ZodError ||
      (error instanceof GenerationFailure &&
        /output_schema_invalid|output_json_invalid/.test(error.message))
    ) {
      const outcome = {
        status: "rejected_invalid" as const,
        problemCode: "proposal_invalid",
      };
      await kernel.modelMaterial.saveWorkflow(
        { ...lease, outcomeJson: JSON.stringify(outcome) },
        options,
      );
      return outcome;
    }
    if (
      error instanceof ConnectError &&
      [Code.Aborted, Code.NotFound].includes(error.code)
    ) {
      const outcome = { status: "obsolete" as const };
      await kernel.modelMaterial.saveWorkflow(
        { ...lease, outcomeJson: JSON.stringify(outcome) },
        options,
      );
      await kernel.authority.refreshMaintenance({ claimed: need }, options);
      return outcome;
    }
    throw error;
  } finally {
    await kernel.modelMaterial
      .releaseWorkflow(lease, { timeoutMs: 5000 })
      .catch(() => {});
  }
}
