import { parseArgs } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { cognitiveMetrics, type RecallPath } from "./cognitive-metrics.js";
import { retrievalMetrics, type RecallOracle } from "./retrieval-metrics.js";
const { values } = parseArgs({
  options: {
    input: { type: "string" },
    output: { type: "string" },
    corpus: { type: "string", default: "docs/research/corpus/cognitive" },
  },
});
if (!values.input || !values.output)
  throw new Error("--input --output required");
const output = resolve(values.output);
if (!output.startsWith(resolve("data/research") + "/"))
  throw new Error("Results must be under ignored data/research");
type Event = { event_id: string; subject: string; session?: string };
type Row = {
  query_id: string;
  subject?: string;
  session_oracle?: string[];
  unresolved_evidence?: string[];
  required_paths?: RecallPath[];
  lane_diagnostics?: {
    candidate_counts: Record<string, number>;
    topology_discarded_mass: number | null;
  };
  category: string;
  profile: string;
  as_of: string;
  authority_watermark: number;
  oracle: Record<
    string,
    {
      grade: RecallOracle["grade"];
      reason: string;
      harmful_kind?: RecallOracle["harmfulKind"];
    }
  >;
  returned: { event_id: string | null }[];
  latency_ms: number;
  degradation: { code: string }[];
  error: unknown;
};
const events: Event[] = [];
const manifest = JSON.parse(
  await readFile(resolve(values.corpus!, "manifest.json"), "utf8"),
) as { scenario_files: { path: string }[] };
for (const file of manifest.scenario_files)
  events.push(
    ...(
      JSON.parse(
        await readFile(resolve(values.corpus!, file.path), "utf8"),
      ) as { events: Event[] }
    ).events,
  );
const frozenQueries = JSON.parse(
  await readFile(resolve(values.corpus!, "queries.json"), "utf8"),
) as { queries: { query_id: string; required_paths?: RecallPath[] }[] };
const frozenPaths = new Map(
  frozenQueries.queries.map((query) => [
    query.query_id,
    query.required_paths ?? [],
  ]),
);
const input = (await readFile(values.input, "utf8")).trim();
const raw = input
  ? input.split("\n").map((line) => JSON.parse(line) as Row)
  : [];
const eventSessions = new Map(
  events.map((event) => [event.event_id, event.session]),
);
const seen = new Set<string>();
const rows = raw.map((row) => {
  const key = `${row.query_id}:${row.profile}`;
  if (seen.has(key)) throw new Error(`Duplicate result ${key}`);
  seen.add(key);
  const subject = row.subject ?? row.query_id.split("-q")[0]!;
  const oracle: Record<string, RecallOracle> = Object.fromEntries(
    events
      .filter((event) => event.subject === subject)
      .map((event) => [
        event.event_id,
        { grade: 0 as const, reason: "distractor", source: event.event_id },
      ]),
  );
  for (const [id, entry] of Object.entries(row.oracle)) {
    if (!(id in oracle)) throw new Error("Oracle event outside Subject");
    oracle[id] = {
      grade: entry.grade,
      reason: entry.reason,
      source: id,
      harmfulKind: entry.harmful_kind,
    };
  }
  const returned = row.returned.map(
    (item, index) => item.event_id ?? `unmapped:${index}`,
  );
  if (returned.some((id) => !id.startsWith("unmapped:") && !(id in oracle)))
    throw new Error("Returned event outside Subject");
  const expectedSessions = new Set(row.session_oracle ?? []);
  const sessionRecall = Object.fromEntries(
    [1, 5, 10].map((k) => {
      const returnedSessions = new Set(
        returned
          .slice(0, k)
          .map((id) => eventSessions.get(id))
          .filter((session): session is string => Boolean(session)),
      );
      return [
        k,
        expectedSessions.size
          ? [...expectedSessions].filter((session) =>
              returnedSessions.has(session),
            ).length / expectedSessions.size
          : null,
      ];
    }),
  );
  return {
    ...row,
    sessionRecall,
    cognitive: cognitiveMetrics(
      returned,
      oracle,
      row.required_paths ?? frozenPaths.get(row.query_id) ?? [],
    ),
    metrics: retrievalMetrics(returned, oracle),
  };
});
const groups = new Map<string, typeof rows>();
for (const row of rows) {
  const key = `${row.category}:${row.profile}`;
  const group = groups.get(key) ?? [];
  group.push(row);
  groups.set(key, group);
}
const summary = [...groups].map(([key, items]) => {
  const mean = (f: (row: (typeof rows)[number]) => number | null) => {
    const measured = items.map(f).filter((v): v is number => v !== null);
    return measured.length
      ? measured.reduce((a, b) => a + b, 0) / measured.length
      : null;
  };
  const latency = items.map((item) => item.latency_ms).sort((a, b) => a - b);
  const percentile = (p: number) =>
    latency[Math.ceil(latency.length * p) - 1] ?? null;
  return {
    group: key,
    count: items.length,
    recall1: mean((r) => r.metrics.atK[1]!.recall),
    recall5: mean((r) => r.metrics.atK[5]!.recall),
    recall10: mean((r) => r.metrics.atK[10]!.recall),
    ndcg5: mean((r) => r.metrics.atK[5]!.ndcg),
    ndcg10: mean((r) => r.metrics.atK[10]!.ndcg),
    mrr: mean((r) => r.metrics.reciprocalRank),
    averagePrecision: mean((r) => r.metrics.averagePrecision),
    sessionRecall1: mean((r) => r.sessionRecall[1] ?? null),
    sessionRecall5: mean((r) => r.sessionRecall[5] ?? null),
    sessionRecall10: mean((r) => r.sessionRecall[10] ?? null),
    unresolvedOracleRows: items.filter((r) => r.unresolved_evidence?.length)
      .length,
    sourceSetRecall10: mean((r) => r.metrics.atK[10]!.sourceSetRecall),
    associationTargetRecall10: mean(
      (r) => r.cognitive[10]!.associationTargetRecall,
    ),
    chainCoverage10: mean((r) => r.cognitive[10]!.chainCoverage),
    precursorRecall10: mean((r) => r.cognitive[10]!.precursorRecall),
    orderedChainScore10: mean((r) => r.cognitive[10]!.orderedChainScore),
    harmfulByKind: Object.fromEntries(
      [1, 5, 10].map((k) => [
        k,
        Object.fromEntries(
          [
            "stale",
            "future",
            "hub",
            "wrong_entity",
            "wrong_session",
            "wrong_time",
          ].map((kind) => [
            kind,
            mean((r) => r.metrics.atK[k]!.harmfulKinds[kind] ?? 0),
          ]),
        ),
      ]),
    ),
    activatedEdges: mean(
      (r) =>
        r.lane_diagnostics?.candidate_counts.topology_activated_edges ?? null,
    ),
    observedMaxHop: Math.max(
      ...items.map(
        (r) =>
          r.lane_diagnostics?.candidate_counts.topology_max_hop_observed ?? 0,
      ),
    ),
    seedCount: mean(
      (r) => r.lane_diagnostics?.candidate_counts.topology_seed_count ?? null,
    ),
    discardedMass: mean((r) =>
      r.profile.startsWith("vcp-")
        ? null
        : (r.lane_diagnostics?.topology_discarded_mass ?? null),
    ),
    harmful1: mean((r) => r.metrics.atK[1]!.harmfulCount),
    harmful5: mean((r) => r.metrics.atK[5]!.harmfulCount),
    harmful10: mean((r) => r.metrics.atK[10]!.harmfulCount),
    weightedHarmfulExposure: mean((r) => r.metrics.weightedHarmfulExposure),
    emptyRate: mean((r) => (r.metrics.emptyResult ? 1 : 0)),
    degradedRows: items.filter((r) => r.degradation.length).length,
    unjudgedCount: items.reduce((sum, r) => sum + r.metrics.unjudgedCount, 0),
    p50: percentile(0.5),
    p95: percentile(0.95),
    p99: percentile(0.99),
  };
});
const categoryDeltas = summary.map((group) => {
  const category = group.group.split(":")[0]!;
  const baseline = summary.find(
    (candidate) => candidate.group === `${category}:baseline-rrf`,
  );
  const difference = (a: number | null, b: number | null | undefined) =>
    a !== null && b != null ? a - b : null;
  return {
    group: group.group,
    recall10Delta: difference(group.recall10, baseline?.recall10),
    ndcg10Delta: difference(group.ndcg10, baseline?.ndcg10),
    harmful10Delta: difference(group.harmful10, baseline?.harmful10),
  };
});
await writeFile(
  output,
  JSON.stringify(
    {
      queryCount: new Set(rows.map((row) => row.query_id)).size,
      resultCount: rows.length,
      summary,
      categoryDeltas,
      rows,
      status: "measured_rows_only_not_suite_completion",
    },
    null,
    2,
  ) + "\n",
  { mode: 0o600 },
);
console.log(
  JSON.stringify({
    queries: new Set(rows.map((r) => r.query_id)).size,
    rows: rows.length,
    categoryProfileGroups: groups.size,
  }),
);
