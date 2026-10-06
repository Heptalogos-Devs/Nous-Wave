import { topologyMaintenanceSchema } from "../model/schemas/topology.js";
import { CommitTopologyRequestSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import {
  executeConsolidation,
  consolidationResultSchema,
} from "./consolidation.js";
import { consolidationSchema } from "../model/schemas/consolidation.js";
import { maintenanceOperationId } from "./identity.js";
export { maintenanceOperationId } from "./identity.js";
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
  status: z.enum([
    "committed",
    "partial",
    "stale",
    "no_change",
    "obsolete",
    "rejected_invalid",
  ]),
  problemCode: z.string().optional(),
  actions: z.array(consolidationResultSchema).optional(),
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
  maintenance_claim: z.unknown().optional(),
});
const proposalSchema = z.discriminatedUnion("action", [
  z.strictObject({ action: z.literal("partition"), request: z.unknown() }),
  z.strictObject({ action: z.literal("journal"), request: z.unknown() }),
  z.strictObject({
    action: z.literal("consolidation"),
    proposal: z.unknown(),
    producer: z.unknown(),
    progress: z.array(consolidationResultSchema),
  }),
  z.strictObject({ action: z.literal("topology"), request: z.unknown() }),
  z.strictObject({
    action: z.literal("withdraw"),
    journalId: z.string().uuid(),
    epoch: z.string(),
  }),
  z.strictObject({ action: z.literal("no_change") }),
]);

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
      triggerRevision: need.triggerRevision.toString(),
      kind: need.kind,
      scopeKind: need.scopeKind,
      scopeRef: need.scopeRef,
    }),
  };
  const previous = await kernel.modelWorkflow.findWorkflow(identity, options);
  if (previous.outcomeJson) return readOutcome(previous.outcomeJson);
  let snapshotJson = previous.snapshotJson;
  if (!snapshotJson) {
    const plan = await kernel.maintenance.planMaintenance(
      { claimed: need },
      options,
    );
    if (plan.status === "obsolete") return { status: "obsolete" as const };
    if (plan.status === "blocked" || plan.status === "deferred")
      return {
        status:
          plan.status === "blocked"
            ? ("blocked_dependency" as const)
            : ("deferred" as const),
        problemCode: plan.problemCode ?? "source_not_settled",
        nextDue: plan.nextDue,
      };
    const role =
      need.kind === "episode_resegment"
        ? "episode_segmentation"
        : need.kind === "topology_maintenance"
          ? "topology_maintenance"
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
  const reservation = await kernel.modelWorkflow.reserveWorkflow(
    {
      ...identity,
      snapshotJson,
      maintenanceNeedId: need.needId,
      maintenanceLeaseToken: need.leaseToken,
      maintenanceTriggerAuthoritySeq: need.triggerAuthoritySeq,
      maintenanceTriggerRevision: need.triggerRevision,
    },
    options,
  );
  if (reservation.outcomeJson) return readOutcome(reservation.outcomeJson);
  if (reservation.busy || !reservation.leaseToken)
    return {
      status: "retry" as const,
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
        } else if (need.kind === "topology_maintenance") {
          if (!plan.topologyPlanJson || !plan.topologyModelInputJson)
            throw new ConnectError(
              "Missing topology catalog",
              Code.InvalidArgument,
            );
          const result = await models.maintainTopology(
            plan.topologyModelInputJson,
            options.signal ?? undefined,
            snapshot.model as ModelRoleSnapshot,
          );
          const proposal = topologyMaintenanceSchema.parse(result.value);
          proposed = {
            action: "topology",
            request: toJson(
              CommitTopologyRequestSchema,
              create(CommitTopologyRequestSchema, {
                operationId,
                claimed: need,
                planJson: plan.topologyPlanJson,
                proposalJson: JSON.stringify(proposal),
                producer: create(
                  ProducerSignatureSchema,
                  producer(
                    result.producerMetadata,
                    "topology_maintenance_text",
                  ),
                ),
              }),
            ),
          };
        } else if (need.kind === "memory_consolidate") {
          const result = await models.consolidate(
            JSON.stringify(snapshot.plan),
            options.signal ?? undefined,
            snapshot.model as ModelRoleSnapshot,
          );
          proposed = {
            action: "consolidation",
            proposal: consolidationSchema.parse(result.value),
            producer: toJson(
              ProducerSignatureSchema,
              create(
                ProducerSignatureSchema,
                producer(result.producerMetadata, "memory_consolidation_text"),
              ),
            ),
            progress: [],
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
      await kernel.modelWorkflow.saveWorkflow(
        { ...lease, proposalJson: JSON.stringify(proposed) },
        options,
      );
    }
    if (proposed.action === "partition")
      await kernel.maintenance.applyEpisodePartition(
        fromJson(
          ApplyEpisodePartitionRequestSchema,
          proposed.request as JsonValue,
        ),
        options,
      );
    else if (proposed.action === "journal")
      await kernel.maintenance.commitJournal(
        fromJson(CommitJournalRequestSchema, proposed.request as JsonValue),
        options,
      );
    else if (proposed.action === "topology") {
      const result = await kernel.maintenance.commitTopology(
        fromJson(CommitTopologyRequestSchema, proposed.request as JsonValue),
        options,
      );
      const parsed: unknown = JSON.parse(result.outcomeJson);
      const committed = z
        .object({ status: z.enum(["committed", "no_change"]) })
        .parse(parsed);
      const outcome = { status: committed.status };
      await kernel.modelWorkflow.saveWorkflow(
        { ...lease, outcomeJson: JSON.stringify(outcome) },
        options,
      );
      return outcome;
    } else if (proposed.action === "consolidation") {
      const saved = proposed;
      const outcome = await executeConsolidation(
        kernel,
        plan,
        consolidationSchema.parse(saved.proposal),
        operationId,
        fromJson(ProducerSignatureSchema, saved.producer as JsonValue),
        options,
        saved.progress,
        async (progress) => {
          saved.progress = [...progress];
          await kernel.modelWorkflow.saveWorkflow(
            { ...lease, proposalJson: JSON.stringify(saved) },
            options,
          );
        },
      );
      await kernel.modelWorkflow.saveWorkflow(
        { ...lease, outcomeJson: JSON.stringify(outcome) },
        options,
      );
      return outcome;
    } else if (proposed.action === "withdraw")
      await kernel.memory.withdrawJournal(
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
      await kernel.maintenance.refreshMaintenance({ claimed: need }, options);
    const outcome = {
      status:
        proposed.action === "no_change"
          ? ("no_change" as const)
          : ("committed" as const),
    };
    await kernel.modelWorkflow.saveWorkflow(
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
      await kernel.modelWorkflow.saveWorkflow(
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
      await kernel.modelWorkflow.saveWorkflow(
        { ...lease, outcomeJson: JSON.stringify(outcome) },
        options,
      );
      await kernel.maintenance.refreshMaintenance({ claimed: need }, options);
      return outcome;
    }
    throw error;
  } finally {
    await kernel.modelWorkflow.releaseWorkflow(lease, {
      timeoutMs: kernel.execution.workflow_ack_timeout_ms,
    });
  }
}
