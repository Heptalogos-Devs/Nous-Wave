import { parseArgs } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
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
type Event = { event_id: string; subject: string };
type Row = {
  query_id: string;
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
for (const scenario of ["archive", "garden", "observatory"])
  events.push(
    ...(
      JSON.parse(
        await readFile(
          resolve(values.corpus!, "scenarios", `${scenario}.json`),
          "utf8",
        ),
      ) as { events: Event[] }
    ).events,
  );
const input = (await readFile(values.input, "utf8")).trim();
const raw = input
  ? input.split("\n").map((line) => JSON.parse(line) as Row)
  : [];
const seen = new Set<string>();
const rows = raw.map((row) => {
  const key = `${row.query_id}:${row.profile}`;
  if (seen.has(key)) throw new Error(`Duplicate result ${key}`);
  seen.add(key);
  const subject = row.query_id.split("-q")[0]!;
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
  return { ...row, metrics: retrievalMetrics(returned, oracle) };
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
    sourceSetRecall10: mean((r) => r.metrics.atK[10]!.sourceSetRecall),
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
await writeFile(
  output,
  JSON.stringify(
    {
      queryCount: new Set(rows.map((row) => row.query_id)).size,
      resultCount: rows.length,
      summary,
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
