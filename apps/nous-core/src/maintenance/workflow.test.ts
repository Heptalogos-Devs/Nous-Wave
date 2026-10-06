// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { coreExecutionSchema } from "../configuration-catalog.js";
import { ExpectedCognitionSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import { describe, expect, it, vi } from "vitest";
import { Code, ConnectError } from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import {
  MaintenanceNeedSchema,
  MaintenancePlanSchema,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { consolidationSchema } from "../model/schemas/consolidation.js";
import { ModelRuntime } from "../model/runtime.js";
import type { ModelRoleSnapshot } from "../model/invocations.js";
import { grantMaintenance, SubjectMaintenanceScheduler } from "./grants.js";
import { runModelMaintenance } from "./workflow.js";
import { maintenanceOperationId } from "./identity.js";

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
    maintenance: {
      planMaintenance: getPlan,
      commitJournal: commit,
      refreshMaintenance: refresh,
    },
    modelWorkflow: {
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
    Object.assign(state.kernel.maintenance, {
      getMaintenancePolicy: vi.fn(async () => ({
        enabled: true,
        maxOperations: 4,
        experienceBatchSize: 256,
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
  it("retains lease ownership while acknowledging an expired opportunity", async () => {
    const state = fixture();
    const claim = vi.fn(async () => ({ needs: [need] }));
    const finish = vi.fn(async () => ({}));
    Object.assign(state.kernel.maintenance, {
      getMaintenancePolicy: vi.fn(async () => ({
        enabled: true,
        maxOperations: 1,
        experienceBatchSize: 256,
        workerLeaseSeconds: 120,
        retryInitialSeconds: 1,
        retryMaxSeconds: 5,
        retryMaxAttempts: 4,
      })),
      claimMaintenance: claim,
      finishMaintenance: finish,
    });
    state.synthesize.mockRejectedValue(
      new ConnectError("Opportunity expired", Code.DeadlineExceeded),
    );
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      const result = await grantMaintenance(state.kernel, state.models, {
        $typeName: "nous.wave.v1alpha1.MaintenanceGrantRequest",
        subjectId: subject,
        maxOperations: 1,
        maxModelCalls: 1,
        maxElapsedMs: 120000,
      });
      expect(claim).toHaveBeenCalledWith(
        expect.objectContaining({ leaseSeconds: 131 }),
        expect.anything(),
      );
      expect(finish).toHaveBeenCalledWith(
        expect.objectContaining({ disposition: "retry" }),
        expect.objectContaining({ timeoutMs: 5000 }),
      );
      expect(result.results[0]?.status).toBe("retry");
    } finally {
      log.mockRestore();
    }
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
    Object.assign(state.kernel.maintenance, {
      getMaintenancePolicy: vi.fn(async () => ({
        enabled: true,
        maxOperations: 4,
        experienceBatchSize: 256,
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
      subjects: { listSubjects: list },
      maintenance: {
        getMaintenancePolicy: vi.fn(async () => ({
          enabled: true,
          maxOperations: 4,
          experienceBatchSize: 7,
          workerLeaseSeconds: 120,
          maxModelCalls: 4,
          maxElapsedMs: 60000,
          pollIntervalSeconds: 30,
        })),
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
    expect(kernel.maintenance.organizeExperience).toHaveBeenCalledWith(
      expect.objectContaining({ limit: 7 }),
      expect.anything(),
    );
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
  });
});

it("routes concepts through bounded input and replays the saved outcome without another model call", async () => {
  const state = fixture();
  const topoNeed = create(MaintenanceNeedSchema, {
    ...need,
    kind: "concept_maintenance",
    scopeKind: "memory_revision",
    scopeRef: revision,
  });
  const modelInput = JSON.stringify({
    focusKey: "c0",
    cognition: [{ key: "c0", text: "Recorded approval before rollout" }],
    tags: [],
    supports: [{ key: "s0", kind: "exact_cognition", targetKey: "c0" }],
  });
  state.getPlan.mockResolvedValue(
    create(MaintenancePlanSchema, {
      subjectId: subject,
      authoritySeq: 1n,
      status: "ready",
      conceptCatalog: { maxSuggestions: 4 },
      conceptModelInputJson: modelInput,
    }),
  );
  const generate = vi
    .spyOn(state.models, "maintainConcepts")
    .mockResolvedValue({
      value: { actions: [{ action: "no_change" }] },
      producerMetadata: {
        implementation: "semantic-stub",
        protocol: "openai-chat",
        model: "stub",
        profileDigest: "a".repeat(64),
        promptId: "program/memory/concept-maintenance.md",
        promptDigest: "b".repeat(64),
        outputSchemaDigest: "c".repeat(64),
        configDigest: "d".repeat(64),
      },
    });
  const reserveCall = vi.fn();
  const first = await runModelMaintenance(
    state.kernel,
    state.models,
    topoNeed,
    {},
    reserveCall,
  );
  expect(first.status).toBe("no_change");
  expect(
    (
      await runModelMaintenance(
        state.kernel,
        state.models,
        topoNeed,
        {},
        reserveCall,
      )
    ).status,
  ).toBe("no_change");
  expect(generate).toHaveBeenCalledTimes(1);
  expect(generate.mock.calls[0]?.[0]).toBe(modelInput);
  expect(reserveCall).toHaveBeenCalledTimes(1);
  expect(state.synthesize).not.toHaveBeenCalled();
});

it("replays a committed consolidation action after a lost response without another model call", async () => {
  const state = fixture();
  const item = {
    cognitiveRole: "declarative",
    formationMode: "synthesized",
    groundingMemberKey: null,
    semanticRole: "statement",
    text: "A durable supported conclusion",
    title: null,
    supportKeys: ["source"],
    entityKeys: [],
    validTime: { kind: "unknown" },
    epistemicClass: "derived",
  };
  state.getPlan.mockResolvedValue(
    create(MaintenancePlanSchema, {
      ...plan,
      maxConsolidationActions: 4,
      consolidationSource: create(ExpectedCognitionSchema, {
        reference: { kind: "episode_revision", value: revision },
        expectedEpoch: 1n,
      }),
    }),
  );
  const metadata = (await state.synthesize("fixture")).producerMetadata;
  const model = vi.spyOn(state.models, "consolidate").mockResolvedValue({
    value: consolidationSchema.parse({
      actions: [
        { action: "create_memory", content: item },
        {
          action: "create_memory",
          content: { ...item, supportKeys: ["invented"] },
        },
      ],
    }),
    producerMetadata: metadata,
  });
  const receipts = new Map<string, { revisionId: string }>();
  let lost = true;
  const form = vi.fn(async (request: { operationId: string }) => {
    if (receipts.has(request.operationId))
      return receipts.get(request.operationId)!;
    const result = { revisionId: revision };
    receipts.set(request.operationId, result);
    if (lost) {
      lost = false;
      throw new ConnectError(
        "lost committed action response",
        Code.Unavailable,
      );
    }
    return result;
  });
  Object.assign(state.kernel, { memory: { formMemory: form } });
  const reserve = vi.fn();
  const current = { ...need, kind: "memory_consolidate" };
  const run = () =>
    runModelMaintenance(state.kernel, state.models, current, {}, reserve);
  await expect(run()).rejects.toThrow("lost committed action response");
  const outcome = await run();
  expect(outcome.status).toBe("partial");
  expect(
    "actions" in outcome && outcome.actions?.map((action) => action.status),
  ).toEqual(["committed", "rejected_invalid"]);
  expect(receipts.size).toBe(1);
  expect(form.mock.calls[0]).toEqual(form.mock.calls[1]);
  expect(model).toHaveBeenCalledTimes(1);
  expect(reserve).toHaveBeenCalledTimes(1);
  expect((await run()).status).toBe("partial");
  expect(form).toHaveBeenCalledTimes(2);
});

it("exposes an owner invariant failure and stops the grant without a provider retry", async () => {
  const state = fixture();
  state.commit.mockRejectedValue(
    new ConnectError("owner invariant", Code.Internal),
  );
  const finish = vi.fn(async () => ({}));
  const claim = vi.fn(async () => ({ needs: [need] }));
  Object.assign(state.kernel.maintenance, {
    getMaintenancePolicy: vi.fn(async () => ({
      enabled: true,
      maxOperations: 4,
      experienceBatchSize: 256,
      workerLeaseSeconds: 120,
      retryInitialSeconds: 1,
      retryMaxSeconds: 5,
      retryMaxAttempts: 4,
    })),
    claimMaintenance: claim,
    finishMaintenance: finish,
  });
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  try {
    const result = await grantMaintenance(state.kernel, state.models, {
      $typeName: "nous.wave.v1alpha1.MaintenanceGrantRequest",
      subjectId: subject,
      maxOperations: 4,
      maxModelCalls: 4,
      maxElapsedMs: 1000,
    });
    expect(result.results[0]?.status).toBe("internal_failure");
    expect(claim).toHaveBeenCalledTimes(1);
    expect(finish).toHaveBeenCalledWith(
      expect.objectContaining({ disposition: "blocked", retryDelaySeconds: 0 }),
      expect.anything(),
    );
  } finally {
    log.mockRestore();
  }
});
