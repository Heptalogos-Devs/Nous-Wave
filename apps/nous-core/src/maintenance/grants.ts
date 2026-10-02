import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import { timestampDate, timestampFromDate } from "@bufbuild/protobuf/wkt";
import type {
  MaintenanceGrantRequest,
  MaintenanceOperationResult,
} from "@nous-wave/protocol/nous/wave/v1alpha1/journal_pb.js";
import type { KernelClient } from "../kernel-client.js";
import type { ModelRuntime } from "../model/runtime.js";
import { runModelMaintenance } from "./workflow.js";

const modelKinds = [
  "episode_resegment",
  "journal_review",
  "journal_revalidate",
];
export async function grantMaintenance(
  kernel: KernelClient,
  models: ModelRuntime,
  input: MaintenanceGrantRequest,
  options: CallOptions = {},
) {
  if (
    !input.subjectId ||
    input.maxOperations < 1 ||
    input.maxOperations > 32 ||
    input.maxModelCalls > 32 ||
    input.maxElapsedMs < 1 ||
    input.maxElapsedMs > 300000
  )
    throw new ConnectError(
      "Invalid maintenance opportunity envelope",
      Code.InvalidArgument,
    );
  const started = performance.now();
  const signal = options.signal
    ? AbortSignal.any([options.signal, AbortSignal.timeout(input.maxElapsedMs)])
    : AbortSignal.timeout(input.maxElapsedMs);
  const calls = { ...options, signal };
  const policy = await kernel.authority.getMaintenancePolicy(
    { subjectId: input.subjectId },
    calls,
  );
  const results: Omit<MaintenanceOperationResult, "$typeName">[] = [];
  let modelCalls = 0;
  if (!policy.enabled)
    return {
      results,
      modelCalls,
      elapsedMs: Math.round(performance.now() - started),
    };
  const limit = Math.min(input.maxOperations, policy.maxOperations);
  for (let index = 0; index < limit && !signal.aborted; index++) {
    const needs = await kernel.authority.claimMaintenance(
      {
        subjectId: input.subjectId,
        allowedKinds: ["episode_segment", ...modelKinds],
        limit: 1,
        leaseSeconds: policy.workerLeaseSeconds,
      },
      calls,
    );
    const need = needs.needs[0];
    if (!need) break;
    let status: string;
    let problemCode: string | undefined;
    let nextDue;
    try {
      if (need.kind === "episode_segment") {
        const organized = await kernel.authority.organizeExperience(
          { subjectId: need.subjectId, limit: 256, close: false },
          calls,
        );
        nextDue = organized.nextDue;
        status = nextDue
          ? "blocked_dependency"
          : organized.episodes.length
            ? "committed"
            : "no_change";
      } else {
        const outcome = await runModelMaintenance(
          kernel,
          models,
          need,
          calls,
          () => {
            if (modelCalls >= input.maxModelCalls)
              throw new ConnectError(
                "Maintenance model call budget exhausted",
                Code.ResourceExhausted,
              );
            modelCalls++;
          },
        );
        status = outcome.status;
        problemCode =
          "problemCode" in outcome ? outcome.problemCode : undefined;
        nextDue = "nextDue" in outcome ? outcome.nextDue : undefined;
      }
    } catch (error) {
      if (
        error instanceof ConnectError &&
        error.code === Code.InvalidArgument
      ) {
        status = "rejected_invalid";
        problemCode = "proposal_invalid";
      } else {
        status = "blocked_dependency";
        problemCode =
          error instanceof ConnectError && error.code === Code.ResourceExhausted
            ? "grant_budget_exhausted"
            : "model_or_transport_unavailable";
      }
    }
    if (status === "blocked_dependency" && !nextDue) {
      const current = await kernel.authority.getMaintenancePolicy(
        { subjectId: input.subjectId },
        { timeoutMs: 5000 },
      );
      if (!current.cognitiveNow)
        throw new ConnectError(
          "Maintenance cognitive time unavailable",
          Code.Internal,
        );
      nextDue = timestampFromDate(
        new Date(timestampDate(current.cognitiveNow).getTime() + 30000),
      );
    }
    await kernel.authority.finishMaintenance(
      {
        claimed: need,
        disposition:
          status === "blocked_dependency"
            ? "pending"
            : status === "obsolete"
              ? "obsolete"
              : "satisfied",
        nextDue,
        problemCode,
      },
      { timeoutMs: 5000 },
    );
    results.push({ needId: need.needId, kind: need.kind, status, problemCode });
  }
  return {
    results,
    modelCalls,
    elapsedMs: Math.round(performance.now() - started),
  };
}

export function startMaintenanceLoop(
  kernel: KernelClient,
  models: ModelRuntime,
) {
  const stopped = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let active: Promise<void> | undefined;
  const tick = async () => {
    let delay = 30000;
    try {
      const options = { signal: stopped.signal, timeoutMs: 10000 };
      const policy = await kernel.authority.getMaintenancePolicy(
        { subjectId: "" },
        options,
      );
      delay = policy.pollIntervalSeconds * 1000;
      if (policy.enabled) {
        const subjects = await kernel.authority.listSubjects({}, options);
        let remaining = policy.maxOperations;
        for (const subject of subjects.items) {
          if (remaining <= 0 || stopped.signal.aborted) break;
          const result = await grantMaintenance(
            kernel,
            models,
            {
              $typeName: "nous.wave.v1alpha1.MaintenanceGrantRequest",
              subjectId: subject.subjectId,
              maxOperations: remaining,
              maxModelCalls: remaining,
              maxElapsedMs: 60000,
            },
            { signal: stopped.signal },
          );
          remaining -= result.results.length;
        }
      }
    } catch {
      // Due work and leases remain durable; the next opportunity retries acquisition.
    } finally {
      if (!stopped.signal.aborted) {
        timer = setTimeout(() => {
          active = tick();
        }, delay);
        timer.unref();
      }
    }
  };
  active = tick();
  return async () => {
    stopped.abort();
    if (timer) clearTimeout(timer);
    await active;
  };
}
