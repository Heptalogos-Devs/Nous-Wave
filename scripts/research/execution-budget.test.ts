import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, it } from "vitest";
import { ExecutionBudget } from "./execution-budget.js";
it("reserves provider and item work atomically across concurrent calls and restart", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-execution-"));
  const limits = {
    providerCalls: 3,
    newEmbeddingItems: 4,
    rerankCalls: 0,
    newServingGenerations: 1,
    newArtifactBytes: 1024,
    runtimeSeconds: 30,
  };
  try {
    const path = join(root, "ledger.json");
    const budget = await ExecutionBudget.open(path, "fixture", limits);
    const attempts = await Promise.allSettled([
      budget.reserve({ providerCalls: 1, newEmbeddingItems: 2 }),
      budget.reserve({ providerCalls: 1, newEmbeddingItems: 2 }),
      budget.reserve({ providerCalls: 1, newEmbeddingItems: 1 }),
    ]);
    expect(attempts.filter((r) => r.status === "fulfilled")).toHaveLength(2);
    expect(budget.snapshot().used.providerCalls).toBe(2);
    expect(budget.snapshot().stopReason).toBe("newEmbeddingItems exhausted");
    await budget.close();
    const resumed = await ExecutionBudget.open(path, "fixture", limits);
    await expect(resumed.reserve({ providerCalls: 1 })).rejects.toThrow(
      "exhausted",
    );
    await resumed.close();
    await expect(
      ExecutionBudget.open(path, "other-corpus", limits),
    ).rejects.toThrow("match");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
it("defaults can forbid providers and rerank while retaining a finite runtime", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-zero-budget-"));
  try {
    const budget = await ExecutionBudget.open(
      join(root, "ledger.json"),
      "fixture",
      {
        providerCalls: 0,
        newEmbeddingItems: 0,
        rerankCalls: 0,
        newServingGenerations: 1,
        newArtifactBytes: 1024,
        runtimeSeconds: 30,
      },
    );
    await expect(budget.reserve({ rerankCalls: 1 })).rejects.toThrow(
      "rerankCalls",
    );
    expect(budget.signal.aborted).toBe(true);
    await budget.close();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

it("expires an idle run and persists its stop reason without another request", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-deadline-"));
  try {
    const budget = await ExecutionBudget.open(
      join(root, "ledger.json"),
      "fixture",
      {
        providerCalls: 1,
        newEmbeddingItems: 1,
        rerankCalls: 0,
        newServingGenerations: 1,
        newArtifactBytes: 1024,
        runtimeSeconds: 1,
      },
    );
    await once(budget.signal, "abort");
    await budget.close();
    expect(budget.snapshot().stopReason).toBe("runtimeSeconds exhausted");
    await expect(budget.reserve({ newServingGenerations: 1 })).rejects.toThrow(
      "runtimeSeconds",
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
