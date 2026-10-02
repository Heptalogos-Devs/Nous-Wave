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
import { grantMaintenance } from "./grants.js";
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
  it("keeps a blocked need pending when the host supplies no model-call budget", async () => {
    const state = fixture();
    const finish = vi.fn(async () => ({}));
    Object.assign(state.kernel.authority, {
      getMaintenancePolicy: vi.fn(async () => ({
        enabled: true,
        maxOperations: 4,
        workerLeaseSeconds: 120,
        cognitiveNow: timestampFromDate(new Date("2026-10-03T00:00:00Z")),
      })),
      claimMaintenance: vi.fn(async () => ({ needs: [need] })),
      finishMaintenance: finish,
    });
    const result = await grantMaintenance(state.kernel, state.models, {
      $typeName: "nous.wave.v1alpha1.MaintenanceGrantRequest",
      subjectId: subject,
      maxOperations: 1,
      maxModelCalls: 0,
      maxElapsedMs: 1000,
    });
    expect(result.modelCalls).toBe(0);
    expect(result.results[0]?.status).toBe("blocked_dependency");
    expect(state.synthesize).not.toHaveBeenCalled();
    expect(finish).toHaveBeenCalledWith(
      expect.objectContaining({
        disposition: "pending",
        problemCode: "grant_budget_exhausted",
      }),
      expect.anything(),
    );
  });
});
