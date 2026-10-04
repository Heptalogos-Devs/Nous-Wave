import type { RecallOracle } from "./retrieval-metrics.js";

export type RecallPath = { nodes: string[]; length: number };
/** Paths are frozen source-graph annotations, never inferred from returned hits. */
export function cognitiveMetrics(
  returned: readonly string[],
  oracle: Readonly<Record<string, RecallOracle>>,
  paths: readonly RecallPath[],
) {
  for (const path of paths) {
    if (
      path.length < 1 ||
      path.nodes.length !== path.length + 1 ||
      new Set(path.nodes).size !== path.nodes.length
    )
      throw new Error(
        "Recall path must contain one simple directed source path",
      );
    if (path.nodes.some((id) => !(id in oracle)))
      throw new Error("Recall path crosses the Subject oracle");
  }
  const targets = new Set(paths.map((path) => path.nodes.at(-1)!));
  const chain = new Set(paths.flatMap((path) => path.nodes.slice(1)));
  const precursors = new Set(
    Object.entries(oracle)
      .filter(
        ([, entry]) => entry.grade > 0 && entry.reason === "causal_precursor",
      )
      .map(([id]) => id),
  );
  const recall = (expected: ReadonlySet<string>, found: ReadonlySet<string>) =>
    expected.size
      ? [...expected].filter((id) => found.has(id)).length / expected.size
      : null;
  return Object.fromEntries(
    [1, 5, 10].map((k) => {
      const head = returned.slice(0, k),
        found = new Set(head);
      let ordered = 0,
        pairs = 0;
      for (const path of paths) {
        const expected = path.nodes.slice(1);
        for (let left = 0; left < expected.length; left++) {
          for (let right = left + 1; right < expected.length; right++) {
            pairs++;
            const a = head.indexOf(expected[left]!),
              b = head.indexOf(expected[right]!);
            if (a >= 0 && b >= 0 && a < b) ordered++;
          }
        }
      }
      return [
        k,
        {
          associationTargetRecall: recall(targets, found),
          chainCoverage: recall(chain, found),
          precursorRecall: recall(precursors, found),
          // Ordering means cue-to-precursor graph order in the ranked retrieval list.
          // Missing nodes earn no pair credit. It is not a generated narrative score.
          orderedChainScore: pairs ? ordered / pairs : null,
        },
      ];
    }),
  );
}
