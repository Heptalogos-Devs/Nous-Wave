import { describe, expect, it } from "vitest";
import { retrievalMetrics } from "./retrieval-metrics.js";

describe("retrieval oracle metrics", () => {
  it("separates multi-target recall, positive gain and harmful exposure", () => {
    const metrics = retrievalMetrics(["a", "bad", "a", "unjudged", "b"], {
      a: { grade: 3, reason: "direct_fact", source: "s1" },
      b: { grade: 2, reason: "causal_precursor", source: "s2" },
      c: { grade: 1, reason: "temporal_context", source: "s2" },
      bad: { grade: -1, reason: "stale", harmfulKind: "stale" },
    });
    expect(metrics.atK[1]!.recall).toBe(1 / 3);
    expect(metrics.atK[1]!.hitRate).toBe(1);
    expect(metrics.atK[5]!.recall).toBe(2 / 3);
    expect(metrics.atK[5]!.sourceSetRecall).toBe(1);
    expect(metrics.averagePrecision).toBeCloseTo((1 + 2 / 5) / 3);
    expect(metrics.atK[5]!.ndcg).toBeCloseTo(
      (7 + 3 / Math.log2(6)) / (7 + 3 / Math.log2(3) + 1 / Math.log2(4)),
    );
    expect(metrics.atK[5]!.harmfulCount).toBe(1);
    expect(metrics.atK[5]!.harmfulKinds.stale).toBe(1);
    expect(metrics.weightedHarmfulExposure).toBeCloseTo(1 / Math.log2(3));
    expect(metrics.duplicateCount).toBe(1);
    expect(metrics.unjudgedCount).toBe(1);
  });
  it("does not invent positive denominators for negative-only oracles", () => {
    const metrics = retrievalMetrics([], {
      bad: { grade: -1, reason: "future", harmfulKind: "future" },
    });
    expect(metrics.atK[10]!.recall).toBeNull();
    expect(metrics.atK[10]!.ndcg).toBeNull();
    expect(metrics.averagePrecision).toBeNull();
    expect(metrics.emptyResult).toBe(true);
    expect(() =>
      retrievalMetrics([], { bad: { grade: 3, reason: "" } }),
    ).toThrow();
    expect(() => retrievalMetrics([], {}, [0])).toThrow();
  });
});
