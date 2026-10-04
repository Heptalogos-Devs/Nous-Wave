import { expect, test } from "vitest";
import { cognitiveMetrics } from "./cognitive-metrics.js";
import type { RecallOracle } from "./retrieval-metrics.js";

test("directed chain coverage counts missing precursors and preserves rank order", () => {
  const oracle: Record<string, RecallOracle> = {
    cue: { grade: 0, reason: "distractor" },
    near: { grade: 2, reason: "causal_precursor" },
    middle: { grade: 1, reason: "association" },
    far: { grade: 3, reason: "association" },
  };
  const paths = [{ nodes: ["cue", "near", "middle", "far"], length: 3 }];
  const ordered = cognitiveMetrics(["near", "middle", "far"], oracle, paths);
  expect(ordered[1]).toEqual({
    associationTargetRecall: 0,
    chainCoverage: 1 / 3,
    precursorRecall: 1,
    orderedChainScore: 0,
  });
  expect(ordered[5]).toEqual({
    associationTargetRecall: 1,
    chainCoverage: 1,
    precursorRecall: 1,
    orderedChainScore: 1,
  });
  const missing = cognitiveMetrics(["far", "near", "near"], oracle, paths);
  expect(missing[10]!.chainCoverage).toBe(2 / 3);
  expect(missing[10]!.orderedChainScore).toBe(0);
  expect(cognitiveMetrics([], oracle, [])[10]!.orderedChainScore).toBeNull();
  expect(() =>
    cognitiveMetrics([], oracle, [{ nodes: ["cue", "far"], length: 3 }]),
  ).toThrow();
  expect(() =>
    cognitiveMetrics([], oracle, [{ nodes: ["cue", "foreign"], length: 1 }]),
  ).toThrow();
});
