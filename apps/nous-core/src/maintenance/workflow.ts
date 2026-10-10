// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { conceptMaintenanceSchema } from "../model/schemas/concept-maintenance.js";
import { executeConceptMaintenance } from "./concept-maintenance.js";
import { executeConsolidation } from "./consolidation.js";
import { consolidationSchema } from "../model/schemas/consolidation.js";
import { maintenanceOperationId } from "./identity.js";
import { Code, ConnectError } from "@connectrpc/connect";
import { executionOptions, type ExecutionOptions } from "../execution.js";
import { fromJson, toJson, type JsonValue } from "@bufbuild/protobuf";
import { z } from "zod";
import {
  MaintenancePlanSchema,
  ApplyEpisodePartitionRequestSchema,
  CommitJournalRequestSchema,
  type MaintenanceNeed,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import { ProducerSignatureSchema } from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import {
  GenerationFailure,
  failedExecutionTelemetry,
} from "../model/execution/routes.js";
import { type KernelClient } from "../kernel-client.js";
import { type ModelRuntime } from "../model/runtime.js";

import { canonicalDigest } from "../digest.js";

import { readOutcome, snapshotSchema, proposalSchema } from "./records.js";
import { buildMaintenanceProposal } from "./proposal.js";

export async function runModelMaintenance(
  kernel: KernelClient,
  models: ModelRuntime,
  need: MaintenanceNeed,
  options: ExecutionOptions,
  reserveModelCall: () => void,
) {
  const calls = executionOptions(kernel.execution.opportunity, options);
  options = calls;
  const { opportunity } = calls;
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
        : need.kind === "concept_maintenance"
          ? "concept_maintenance"
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
      leaseSeconds: opportunity.leaseSeconds,
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
      proposed = await buildMaintenanceProposal({
        models,
        need,
        plan,
        snapshot,
        operationId,
        options,
        reserveModelCall,
        recordExecution: async (value) => {
          await kernel.modelWorkflow.saveWorkflow(
            { ...lease, executionTelemetryJson: JSON.stringify(value) },
            opportunity.cleanup(),
          );
        },
      });
      await kernel.modelWorkflow.saveWorkflow(
        {
          ...lease,
          proposalJson: JSON.stringify(proposed),
        },
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
    else if (proposed.action === "concept") {
      const saved = proposed;
      const outcome = await executeConceptMaintenance(
        kernel,
        plan,
        conceptMaintenanceSchema.parse(saved.proposal),
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
    if (failedExecutionTelemetry(error))
      await kernel.modelWorkflow.saveWorkflow(
        {
          ...lease,
          executionTelemetryJson: JSON.stringify(
            failedExecutionTelemetry(error),
          ),
        },
        opportunity.cleanup(),
      );
    if (
      (error instanceof ConnectError &&
        [Code.InvalidArgument, Code.FailedPrecondition].includes(error.code)) ||
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
        opportunity.cleanup(),
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
        opportunity.cleanup(),
      );
      await kernel.maintenance.refreshMaintenance({ claimed: need }, options);
      return outcome;
    }
    throw error;
  } finally {
    await kernel.modelWorkflow.releaseWorkflow(lease, opportunity.cleanup());
  }
}
