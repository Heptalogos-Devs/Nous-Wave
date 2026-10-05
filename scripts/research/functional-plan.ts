import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { resolve } from "node:path";
import { parseArgs } from "node:util";
import type { ExecutionLimits } from "./execution-budget.js";

export const budgetOptions = {
  "max-provider-calls": { type: "string", default: "0" },
  "max-new-embedding-items": { type: "string", default: "0" },
  "max-rerank-calls": { type: "string", default: "0" },
  "max-new-serving-generations": { type: "string", default: "3" },
  "max-new-artifact-bytes": { type: "string", default: "67108864" },
  "max-runtime-seconds": { type: "string", default: "180" },
} as const;
export function executionLimits(
  values: Record<string, string | boolean | string[] | undefined>,
): ExecutionLimits {
  const number = (key: keyof typeof budgetOptions) => {
    const value = Number(values[key] ?? budgetOptions[key].default);
    if (
      !Number.isSafeInteger(value) ||
      value < 0 ||
      (key === "max-runtime-seconds" && value === 0)
    )
      throw new Error(`Invalid --${key}`);
    return value;
  };
  return {
    providerCalls: number("max-provider-calls"),
    newEmbeddingItems: number("max-new-embedding-items"),
    rerankCalls: number("max-rerank-calls"),
    newServingGenerations: number("max-new-serving-generations"),
    newArtifactBytes: number("max-new-artifact-bytes"),
    runtimeSeconds: number("max-runtime-seconds"),
  };
}
export interface FunctionalScenario {
  key: string;
  title: string;
  entities: {
    key: string;
    display_name: string;
    aliases: string[];
    description: string;
  }[];
  work_context: { key: string; purpose: string; open_questions: string[] };
  events: {
    key: string;
    occurred_at: string;
    observed_at: string;
    session_key: string;
    text: string;
    entity_keys: string[];
  }[];
  expected_maintenance: Record<string, unknown>;
}
export interface FunctionalQuery {
  key: string;
  scenario_key: string;
  category: string;
  surface_example: string;
  prepared_query: {
    canonical_text: string;
    as_of: string;
    entity_keys?: string[];
    concept_keys?: string[];
    exact_event_keys?: string[];
    work_context_key?: string;
    exploration?: string;
    time_axis?: string;
    time_start?: string;
    time_end?: string;
    minimum_hops?: number;
  };
  oracle: {
    required_event_keys: string[];
    forbidden_event_keys: string[];
    requires_supported_path: boolean;
  };
}
export async function loadFunctionalCorpus(path: string, queryIds?: string[]) {
  const contents = await Promise.all(
    ["manifest.json", "scenarios.json", "queries.json"].map(async (name) => {
      const file = resolve(path, name);
      if ((await stat(file)).size > 256 * 1024)
        throw new Error("Functional input exceeds 256 KiB");
      return readFile(file, "utf8");
    }),
  );
  const digest = createHash("sha256").update(contents.join("\n")).digest("hex");
  const manifest = JSON.parse(contents[0]!) as {
    suite: string;
    event_count: number;
    query_count: number;
  };
  const scenarios = (
    JSON.parse(contents[1]!) as { scenarios: FunctionalScenario[] }
  ).scenarios;
  let queries = (JSON.parse(contents[2]!) as { queries: FunctionalQuery[] })
    .queries;
  if (
    scenarios.length > 4 ||
    scenarios.reduce((sum, s) => sum + s.events.length, 0) > 40 ||
    queries.length > 40
  )
    throw new Error("Functional corpus exceeds the small-run scope");
  if (
    manifest.event_count !==
      scenarios.reduce((sum, s) => sum + s.events.length, 0) ||
    manifest.query_count !== queries.length
  )
    throw new Error("Functional manifest counts do not match");
  const subjects = new Map(scenarios.map((s) => [s.key, s]));
  if (
    subjects.size !== scenarios.length ||
    new Set(queries.map((q) => q.key)).size !== queries.length
  )
    throw new Error("Duplicate scenario/query key");
  for (const query of queries) {
    const scenario = subjects.get(query.scenario_key);
    if (!scenario) throw new Error(`Unknown scenario for ${query.key}`);
    const events = new Set(scenario.events.map((e) => e.key));
    const entities = new Set(scenario.entities.map((e) => e.key));
    if (
      [
        ...query.oracle.required_event_keys,
        ...query.oracle.forbidden_event_keys,
        ...(query.prepared_query.exact_event_keys ?? []),
      ].some((key) => !events.has(key)) ||
      (query.prepared_query.entity_keys ?? []).some((key) => !entities.has(key))
    )
      throw new Error(`Unclosed fixture reference in ${query.key}`);
  }
  if (queryIds) {
    if (queryIds.some((key) => !queries.some((q) => q.key === key)))
      throw new Error("Unknown --query-ids");
    queries = queries.filter((q) => queryIds.includes(q.key));
  }
  return { manifest, scenarios, queries, digest };
}
export async function formationPlan(
  corpusPath: string,
  limits: ExecutionLimits,
  queryIds?: string[],
) {
  const corpus = await loadFunctionalCorpus(corpusPath, queryIds);
  return {
    suite: corpus.manifest.suite,
    phase: "formation-maintenance",
    inputDigest: corpus.digest,
    subjects: corpus.scenarios.length,
    events: corpus.manifest.event_count,
    queries: corpus.queries.length,
    profiles: [],
    uniqueTexts: new Set(
      corpus.scenarios.flatMap((s) => s.events.map((e) => e.text)),
    ).size,
    existingEmbeddings: 0,
    embeddingMisses: 0,
    estimatedEmbeddingRequests: 0,
    rerankRequests: 0,
    sharedServingAssetsReused: 0,
    newAlgorithmAssets: 0,
    newServingGenerations: 0,
    estimatedNewArtifactBytes: {
      upperBound: limits.newArtifactBytes,
      basis: "run-owned disk cap; formation output is model-dependent",
    },
    externalDownloads: 0,
    estimatedRuntimeClass: `bounded by ${limits.runtimeSeconds} seconds`,
    providerRequests: {
      upperBound: limits.providerCalls,
      basis: "all model attempts share one wire cap, including retries",
    },
    limits,
    retrievalDeferred:
      "Freeze actual cognition and full query representation, inspect automatic concepts, then run a separate exact embedding/Serving preflight.",
  };
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === resolve(import.meta.filename)
) {
  const { values } = parseArgs({
    options: {
      ...budgetOptions,
      plan: { type: "boolean" },
      corpus: { type: "string", default: "docs/research/corpus/functional" },
      "query-ids": { type: "string" },
    },
  });
  if (!values.plan)
    throw new Error(
      "This command prepares a formation plan; pass --plan. The owner execution runner is still being connected.",
    );
  console.log(
    JSON.stringify(
      await formationPlan(
        values.corpus!,
        executionLimits(values),
        values["query-ids"]?.split(","),
      ),
      null,
      2,
    ),
  );
}
