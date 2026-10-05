import { parseArgs } from "node:util";
import { readFile, stat, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { loadFunctionalCorpus } from "./functional-plan.js";
import {
  sourceIndex,
  supportedRoute,
  type Hit,
  type Receipts,
  type SourceInspection,
} from "./functional-grading.js";
const { values } = parseArgs({
  options: {
    corpus: { type: "string", default: "docs/research/corpus/functional" },
    result: { type: "string" },
    receipts: { type: "string" },
    output: { type: "string" },
  },
});
if (!values.result || !values.receipts || !values.output)
  throw new Error("--result, --receipts and --output required");
const read = async (path: string) => {
  if ((await stat(path)).size > 32 * 1024 * 1024)
    throw new Error("Bounded Functional result exceeded 32 MiB");
  return readFile(path, "utf8");
};
if (!resolve(values.output).startsWith(resolve("data/research") + "/"))
  throw new Error("Research-owned output required");
const run = JSON.parse(await read(values.result)) as {
  plan: { profiles: string[] };
  rows: {
    inspection?: SourceInspection;
    key?: string;
    profile?: string;
    result?: { status: string; results: Hit[] };
    requestedLimit?: number;
  }[];
};
const inspection = run.rows.find((row) => row.inspection)?.inspection;
if (!inspection)
  throw new Error(
    "Run lacks its frozen owner inspection; rerun the bounded Prepared readout",
  );
const receipts = JSON.parse(await read(values.receipts)) as Receipts;
const sources = sourceIndex(inspection, receipts);
const corpus = await loadFunctionalCorpus(values.corpus!);
const rows = corpus.queries.flatMap((intent) =>
  run.plan.profiles.map((profile) => {
    const row = run.rows.find(
      (candidate) =>
        candidate.key === intent.key && candidate.profile === profile,
    );
    if (!row?.result)
      return {
        key: intent.key,
        profile,
        structuralStatus: "missing_prepared_readout",
        semanticReview: "pending",
      };
    const required = intent.oracle.required_event_keys.map(
      (event) => `${intent.scenario_key}:${event}`,
    );
    const forbidden = new Set(
      intent.oracle.forbidden_event_keys.map(
        (event) => `${intent.scenario_key}:${event}`,
      ),
    );
    const grounded = row.result.results.map((hit) => [
      ...sources(`${hit.reference.kind}:${hit.reference.id}`),
    ]);
    const recovered = new Set(grounded.flat());
    const missing = required.filter((event) => !recovered.has(event));
    const wrongScenario = [...recovered].filter(
      (event) => !event.startsWith(intent.scenario_key + ":"),
    );
    const forbiddenRecovered = [...recovered].filter((event) =>
      forbidden.has(event),
    );
    const pathRanks = row.result.results.flatMap((hit, index) =>
      supportedRoute(hit, intent.prepared_query.minimum_hops ?? 1)
        ? [index + 1]
        : [],
    );
    const ranksValid = row.result.results.every(
      (hit, index) => hit.match_evidence.final_rank === index + 1,
    );
    const limitValid =
      row.requestedLimit !== undefined &&
      row.result.results.length <= row.requestedLimit;
    const supportedPath =
      !intent.oracle.requires_supported_path || pathRanks.length > 0;
    const coverageAt = (k: number) => {
      const prefix = new Set(grounded.slice(0, k).flat());
      return required.length
        ? required.filter((event) => prefix.has(event)).length / required.length
        : null;
    };
    return {
      key: intent.key,
      profile,
      queryStatus: row.result.status,
      structuralStatus:
        missing.length ||
        wrongScenario.length ||
        forbiddenRecovered.length ||
        !supportedPath ||
        !ranksValid ||
        !limitValid
          ? "failed"
          : "supported",
      semanticReview: "pending",
      finalHits: row.result.results.length,
      missingRequiredEvents: missing,
      wrongScenario,
      forbiddenRecovered,
      unknownSourceRanks: grounded.flatMap((events, index) =>
        events.length ? [] : [index + 1],
      ),
      sourceCoverageAt1: coverageAt(1),
      sourceCoverageAt5: coverageAt(5),
      sourceCoverageAt16: coverageAt(16),
      supportedPathRanks: pathRanks,
      ranksValid,
      limitValid,
    };
  }),
);
await writeFile(
  values.output,
  JSON.stringify(
    {
      suite: corpus.manifest.suite,
      expectedQueries: corpus.queries.length,
      profiles: run.plan.profiles,
      scope:
        "Source grounding and witnessed path contracts only. Semantic relevance, concept correctness and causal explanation require manual review; supported does not mean full acceptance.",
      rows,
    },
    null,
    2,
  ) + "\n",
  { mode: 0o600 },
);
console.log(
  JSON.stringify({
    rows: rows.length,
    structuralSupported: rows.filter(
      (row) => row.structuralStatus === "supported",
    ).length,
    missingReadouts: rows.filter(
      (row) => row.structuralStatus === "missing_prepared_readout",
    ).length,
    semanticReview: "pending",
    providerCalls: 0,
  }),
);
