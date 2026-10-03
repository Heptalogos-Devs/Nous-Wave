import { coreExecutionSchema } from "../configuration-catalog.js";
import type { CommitLongitudinalConsolidationRequest } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/consolidation_pb.js";
import { ExpectedCognitionSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/consolidation_pb.js";
import { describe, expect, it, vi } from "vitest";
import { Code, ConnectError } from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import {
  MaintenanceNeedSchema,
  MaintenancePlanSchema,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { ModelRuntime } from "../model/runtime.js";
import type { ModelRoleSnapshot } from "../model/invocations.js";
import { grantMaintenance, SubjectMaintenanceScheduler } from "./grants.js";
import { maintenanceOperationId, runModelMaintenance } from "./workflow.js";

const subject = "10000000-0000-4000-8000-000000000001";
const revision = "20000000-0000-4000-8000-000000000001";
const need = create(MaintenanceNeedSchema, {
  subjectId: subject,
  needId: "30000000-0000-4000-8000-000000000001",
  kind: "journal_review",
  scopeKind: "track",
  scopeRef: "interaction",
  triggerAuthoritySeq: 4n,
});
const plan = create(MaintenancePlanSchema, {
  subjectId: subject,
  authoritySeq: 4n,
  cognitiveNow: timestampFromDate(new Date("2026-10-03T00:00:00Z")),
  status: "ready",
  sources: [{ revisionId: revision, expectedEpoch: 1n }],
  supports: [
    {
      key: "source",
      support: {
        support: {
          case: "cognitionDependency",
          value: {
            targetRevision: { kind: "episode_revision", value: revision },
            supportRole: "direct",
          },
        },
      },
    },
  ],
});
function fixture() {
  const models = new ModelRuntime();
  vi.spyOn(models.invocations, "snapshot").mockReturnValue(
    {} as ModelRoleSnapshot,
  );
  const synthesize = vi.spyOn(models, "synthesizeJournal").mockResolvedValue({
    value: {
      action: "commit",
      title: null,
      narrative: "A supported decision.",
      points: [
        {
          role: "decision",
          text: "Continue the plan.",
          supportKeys: ["source"],
        },
      ],
    },
    producerMetadata: {
      implementation: "semantic-stub",
      protocol: "openai-chat",
      model: "stub",
      profileDigest: "a".repeat(64),
      promptId: "program/journal/synthesis.md",
      promptDigest: "b".repeat(64),
      outputSchemaDigest: "c".repeat(64),
      configDigest: "d".repeat(64),
    },
  });
  let snapshotJson: string | undefined;
  let proposalJson: string | undefined;
  let outcomeJson: string | undefined;
  const commit = vi.fn(async () => ({}));
  const refresh = vi.fn(async () => ({}));
  const getPlan = vi.fn(async () => plan);
  const kernel = {
    execution: coreExecutionSchema.parse(undefined),
    authority: {
      planMaintenance: getPlan,
      commitJournal: commit,
      refreshMaintenance: refresh,
    },
    modelMaterial: {
      findWorkflow: vi.fn(async () => ({
        found: !!snapshotJson,
        snapshotJson,
        proposalJson,
        outcomeJson,
      })),
      reserveWorkflow: vi.fn(async (input: { snapshotJson: string }) => {
        snapshotJson ??= JSON.stringify({
          ...(JSON.parse(input.snapshotJson) as Record<string, unknown>),
          cognitive_formed_at: "2026-10-03T00:00:00Z",
        });
        return {
          snapshotJson,
          proposalJson,
          outcomeJson,
          leaseToken: "lease",
          busy: false,
        };
      }),
      saveWorkflow: vi.fn(
        async (input: { proposalJson?: string; outcomeJson?: string }) => {
          proposalJson = input.proposalJson ?? proposalJson;
          outcomeJson = input.outcomeJson ?? outcomeJson;
          return {};
        },
      ),
      releaseWorkflow: vi.fn(async () => ({})),
    },
  } as unknown as KernelClient;
  return { kernel, models, synthesize, commit, refresh, getPlan };
}
describe("maintenance fixed workflow retry", () => {
  it("reuses the exact proposal after an owner response is lost and replays the outcome", async () => {
    const state = fixture();
    state.commit.mockRejectedValueOnce(
      new ConnectError("Response lost after commit", Code.Unavailable),
    );
    const reserveCall = vi.fn();
    await expect(
      runModelMaintenance(state.kernel, state.models, need, {}, reserveCall),
    ).rejects.toThrow();
    const first = state.commit.mock.calls[0];
    const result = await runModelMaintenance(
      state.kernel,
      state.models,
      need,
      {},
      reserveCall,
    );
    expect(result.status).toBe("committed");
    expect(state.commit.mock.calls[1]).toEqual(first);
    expect(state.synthesize).toHaveBeenCalledTimes(1);
    expect(reserveCall).toHaveBeenCalledTimes(1);
    expect(state.getPlan).toHaveBeenCalledTimes(1);
    expect(
      (
        await runModelMaintenance(
          state.kernel,
          state.models,
          need,
          {},
          reserveCall,
        )
      ).status,
    ).toBe("committed");
    expect(state.commit).toHaveBeenCalledTimes(2);
    expect(
      maintenanceOperationId({ ...need, triggerAuthoritySeq: 5n }),
    ).not.toBe(maintenanceOperationId(need));
  });
  it("records a stale outcome and refreshes the need; invented supports are terminal", async () => {
    const state = fixture();
    state.commit.mockRejectedValueOnce(
      new ConnectError("Source changed", Code.Aborted),
    );
    expect(
      (
        await runModelMaintenance(
          state.kernel,
          state.models,
          need,
          {},
          () => {},
        )
      ).status,
    ).toBe("obsolete");
    expect(state.refresh).toHaveBeenCalledTimes(1);
    expect(
      (
        await runModelMaintenance(
          state.kernel,
          state.models,
          need,
          {},
          () => {},
        )
      ).status,
    ).toBe("obsolete");
    expect(state.synthesize).toHaveBeenCalledTimes(1);
    const invalid = fixture();
    const result = await invalid.synthesize("fixture");
    if (typeof result.value === "string" || result.value.action !== "commit")
      throw new Error("Expected commit");
    result.value.points[0]!.supportKeys = ["invented"];
    invalid.synthesize.mockResolvedValue(result);
    expect(
      (
        await runModelMaintenance(
          invalid.kernel,
          invalid.models,
          need,
          {},
          () => {},
        )
      ).status,
    ).toBe("rejected_invalid");
    expect(invalid.commit).not.toHaveBeenCalled();
  });
  it("does not lease model work without executable roles or model budget", async () => {
    const state = fixture();
    let attempts = 0;
    const claim = vi.fn(async (input: { allowedKinds: string[] }) => {
      if (!input.allowedKinds.includes(need.kind)) return { needs: [] };
      attempts++;
      return { needs: [need] };
    });
    Object.assign(state.kernel.authority, {
      getMaintenancePolicy: vi.fn(async () => ({
        enabled: true,
        maxOperations: 4,
        workerLeaseSeconds: 120,
      })),
      claimMaintenance: claim,
      finishMaintenance: vi.fn(),
    });
    const input = {
      $typeName: "nous.wave.v1alpha1.MaintenanceGrantRequest" as const,
      subjectId: subject,
      maxOperations: 1,
      maxModelCalls: 1,
      maxElapsedMs: 1000,
    };
    for (let tick = 0; tick < 10; tick++)
      await grantMaintenance(state.kernel, state.models, input);
    expect(attempts).toBe(0);
    vi.spyOn(state.models.invocations, "capabilities", "get").mockReturnValue([
      { name: "model.journal_synthesis", state: "READY", detail: "Configured" },
    ]);
    await grantMaintenance(state.kernel, state.models, {
      ...input,
      maxModelCalls: 0,
    });
    expect(attempts).toBe(0);
    expect(state.synthesize).not.toHaveBeenCalled();
  });
  it("uses bounded exponential retry for provider failure and then blocks", async () => {
    const state = fixture();
    vi.spyOn(state.models.invocations, "capabilities", "get").mockReturnValue([
      { name: "model.journal_synthesis", state: "READY", detail: "Configured" },
    ]);
    state.synthesize.mockRejectedValue(
      new ConnectError("provider unavailable", Code.Unavailable),
    );
    let retryCount = 0;
    const finish = vi.fn(async (input: { disposition: string }) => {
      if (input.disposition === "retry") retryCount++;
      return {};
    });
    Object.assign(state.kernel.authority, {
      getMaintenancePolicy: vi.fn(async () => ({
        enabled: true,
        maxOperations: 4,
        workerLeaseSeconds: 120,
        retryInitialSeconds: 2,
        retryMaxSeconds: 5,
        retryMaxAttempts: 4,
      })),
      claimMaintenance: vi.fn(async () => ({
        needs: [{ ...need, retryCount }],
      })),
      finishMaintenance: finish,
    });
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    for (let tick = 0; tick < 4; tick++)
      await grantMaintenance(state.kernel, state.models, {
        $typeName: "nous.wave.v1alpha1.MaintenanceGrantRequest",
        subjectId: subject,
        maxOperations: 1,
        maxModelCalls: 1,
        maxElapsedMs: 1000,
      });
    expect(finish.mock.calls.map(([input]) => input)).toMatchObject([
      { disposition: "retry", retryDelaySeconds: 2 },
      { disposition: "retry", retryDelaySeconds: 4 },
      { disposition: "retry", retryDelaySeconds: 5 },
      { disposition: "blocked", problemCode: "maintenance_retry_exhausted" },
    ]);
    expect(log).toHaveBeenCalled();
    log.mockRestore();
  });
  it("continues through more than one Subject page under sustained due work", async () => {
    const ids = Array.from({ length: 75 }, (_, index) => `subject-${index}`);
    const opportunities: string[] = [];
    const list = vi.fn(async (input: { page?: { pageToken?: string } }) => {
      const offset = Number(input.page?.pageToken || 0);
      const items = ids
        .slice(offset, offset + 50)
        .map((subjectId) => ({ subjectId, capabilities: { memory: true } }));
      return {
        items,
        nextPageToken: offset + 50 < ids.length ? String(offset + 50) : "",
      };
    });
    const kernel = {
      execution: coreExecutionSchema.parse(undefined),
      authority: {
        getMaintenancePolicy: vi.fn(async () => ({
          enabled: true,
          maxOperations: 4,
          workerLeaseSeconds: 120,
          maxModelCalls: 4,
          maxElapsedMs: 60000,
          pollIntervalSeconds: 30,
        })),
        listSubjects: list,
        claimMaintenance: vi.fn(async (input: { subjectId: string }) => ({
          needs: [
            { ...need, subjectId: input.subjectId, kind: "episode_segment" },
          ],
        })),
        organizeExperience: vi.fn(async (input: { subjectId: string }) => {
          opportunities.push(input.subjectId);
          return { episodes: [], nextDue: undefined };
        }),
        finishMaintenance: vi.fn(async () => ({})),
      },
    } as unknown as KernelClient;
    const scheduler = new SubjectMaintenanceScheduler(
      kernel,
      new ModelRuntime(),
    );
    for (let tick = 0; tick < 20; tick++)
      await scheduler.poll(new AbortController().signal);
    expect(opportunities.slice(0, 75)).toEqual(ids);
    expect(opportunities.slice(75)).toEqual(ids.slice(0, 5));
    expect(list).toHaveBeenCalledWith(
      expect.objectContaining({ page: { pageToken: "50" } }),
      expect.anything(),
    );
    list.mockRejectedValueOnce(
      new ConnectError("expired page token", Code.InvalidArgument),
    );
    for (let tick = 0; tick < 40; tick++)
      await scheduler.poll(new AbortController().signal);
    expect(
      opportunities.filter((id) => id === ids[74]).length,
    ).toBeGreaterThanOrEqual(2);
  });
  it("executes consolidation through the typed owner and replays a no-change outcome", async () => {
    const state = fixture();
    const consolidationNeed = {
      ...need,
      kind: "memory_consolidate",
      scopeKind: "episode_revision",
      scopeRef: revision,
    };
    const consolidationPlan = create(MaintenancePlanSchema, {
      ...plan,
      consolidationSource: create(ExpectedCognitionSchema, {
        reference: { kind: "episode_revision", value: revision },
        expectedEpoch: 1n,
      }),
      maxConsolidationActions: 8,
    });
    state.getPlan.mockResolvedValue(consolidationPlan);
    const model = vi.spyOn(state.models, "consolidate").mockResolvedValue({
      value: {
        actions: [
          {
            action: "skip",
            reason: "The source adds no reusable cognition.",
          },
        ],
      },
      producerMetadata: {
        implementation: "semantic-stub",
        protocol: "openai-chat",
        model: "stub",
        profileDigest: "a".repeat(64),
        promptId: "program/memory/consolidation.md",
        promptDigest: "b".repeat(64),
        outputSchemaDigest: "c".repeat(64),
        configDigest: "d".repeat(64),
      },
    });
    const commit = vi.fn(
      async (_request: CommitLongitudinalConsolidationRequest) => ({
        status: "no_change",
        authoritySeq: 4n,
        results: [],
      }),
    );
    Object.assign(state.kernel.authority, {
      commitLongitudinalConsolidation: commit,
    });
    expect(
      (
        await runModelMaintenance(
          state.kernel,
          state.models,
          consolidationNeed,
          {},
          () => {},
        )
      ).status,
    ).toBe("no_change");
    expect(commit.mock.calls[0]![0].source?.expectedEpoch).toBe(1n);
    expect(commit.mock.calls[0]![0].actions[0]?.action.case).toBe("skip");
    expect(
      (
        await runModelMaintenance(
          state.kernel,
          state.models,
          consolidationNeed,
          {},
          () => {},
        )
      ).status,
    ).toBe("no_change");
    expect(model).toHaveBeenCalledTimes(1);
    expect(commit).toHaveBeenCalledTimes(1);
  });
});
