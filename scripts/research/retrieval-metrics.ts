/** Retrieval-only metrics. Relevance is fixed before a result is observed. */
export type RecallOracle = {
  grade: -1 | 0 | 1 | 2 | 3;
  reason: string;
  source?: string;
  harmfulKind?:
    | "stale"
    | "future"
    | "hub"
    | "wrong_entity"
    | "wrong_session"
    | "wrong_time";
};
export function retrievalMetrics(
  returned: readonly string[],
  oracle: Readonly<Record<string, RecallOracle>>,
  cutoffs: readonly number[] = [1, 5, 10],
) {
  if (
    Object.values(oracle).some(
      (entry) =>
        !Number.isInteger(entry.grade) ||
        entry.grade < -1 ||
        entry.grade > 3 ||
        !entry.reason.trim(),
    )
  )
    throw new Error("Oracle requires a bounded grade and reason");
  if (cutoffs.some((k) => !Number.isInteger(k) || k <= 0))
    throw new Error("Metric cutoffs must be positive integers");
  const positive = Object.entries(oracle).filter(
    ([, entry]) => entry.grade > 0,
  );
  const relevant = new Set(positive.map(([id]) => id));
  const expectedSources = new Set(
    positive.flatMap(([, entry]) => (entry.source ? [entry.source] : [])),
  );
  const ideal = positive
    .map(([, entry]) => 2 ** entry.grade - 1)
    .sort((a, b) => b - a);
  const seen = new Set<string>();
  // A repeated result occupies a rank but earns no additional gain or recall.
  const rows = returned.map((id, index) => {
    const duplicate = seen.has(id);
    seen.add(id);
    const entry = oracle[id];
    return {
      id,
      rank: index + 1,
      entry,
      positive: !duplicate && relevant.has(id),
      gain: !duplicate && entry && entry.grade > 0 ? 2 ** entry.grade - 1 : 0,
    };
  });
  const discount = (rank: number) => 1 / Math.log2(rank + 1);
  const first = rows.find((row) => row.positive)?.rank;
  let positives = 0;
  let precisionSum = 0;
  for (const row of rows) {
    if (row.positive) {
      positives++;
      precisionSum += positives / row.rank;
    }
  }
  const atK = Object.fromEntries(
    cutoffs.map((k) => {
      const head = rows.slice(0, k);
      const found = head.filter((row) => row.positive).length;
      const dcg = head.reduce(
        (sum, row) => sum + row.gain * discount(row.rank),
        0,
      );
      const idcg = ideal
        .slice(0, k)
        .reduce((sum, gain, i) => sum + gain * discount(i + 1), 0);
      const harmful = head.filter((row) => row.entry?.grade === -1);
      const sources = new Set(
        head
          .filter((row) => row.positive)
          .flatMap((row) => (row.entry?.source ? [row.entry.source] : [])),
      );
      const harmfulKinds = Object.fromEntries(
        [
          "stale",
          "future",
          "hub",
          "wrong_entity",
          "wrong_session",
          "wrong_time",
        ].map((kind) => [
          kind,
          harmful.filter((row) => row.entry?.harmfulKind === kind).length,
        ]),
      );
      return [
        k,
        {
          recall: relevant.size ? found / relevant.size : null,
          hitRate: found > 0 ? 1 : 0,
          ndcg: idcg ? dcg / idcg : null,
          sourceSetRecall: expectedSources.size
            ? sources.size / expectedSources.size
            : null,
          harmfulCount: harmful.length,
          harmfulExposure: harmful.reduce(
            (sum, row) => sum + discount(row.rank),
            0,
          ),
          harmfulKinds,
        },
      ];
    }),
  );
  return {
    atK,
    reciprocalRank: first ? 1 / first : 0,
    averagePrecision: relevant.size ? precisionSum / relevant.size : null,
    weightedHarmfulExposure: rows
      .filter((row) => row.entry?.grade === -1)
      .reduce((sum, row) => sum + discount(row.rank), 0),
    duplicateCount: returned.length - seen.size,
    unjudgedCount: rows.filter((row) => !row.entry).length,
    emptyResult: returned.length === 0,
    oraclePositiveCount: relevant.size,
  };
}
