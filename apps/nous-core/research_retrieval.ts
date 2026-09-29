import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

type Corpus = {
  version: string;
  query_plan: {
    identity: string;
    hard_constraints: string[];
    candidate_budget: number;
    fusion: string;
  };
  embedding_space: Record<string, unknown>;
  queries: Array<{
    id: string;
    relevant: string[];
    candidates: Record<string, string[]>;
    support_precision: Record<string, number>;
    entity_error: Record<string, number>;
    latency_ms: Record<string, number>;
    work_units: {
      candidate_count: Record<string, number>;
      topology_visited: Record<string, number>;
    };
  }>;
};

const fixturePath = join(
  process.cwd(),
  "apps",
  "nous-kernel",
  "tests",
  "fixtures",
  "retrieval-research",
  "corpus.json",
);
const digest = (bytes: Uint8Array) =>
  createHash("sha256").update(bytes).digest("hex");
const percentile = (values: number[], fraction: number) => {
  const ordered = [...values].sort((a, b) => a - b);
  return (
    ordered[
      Math.min(ordered.length - 1, Math.floor((ordered.length - 1) * fraction))
    ] ?? 0
  );
};
const canonical = (value: unknown) => JSON.stringify(value);

async function main() {
  const bytes = await readFile(fixturePath);
  const corpus = JSON.parse(bytes.toString("utf8")) as Corpus;
  const variants = ["baseline", "baseline_plus_wave"] as const;
  const runs = variants.map((variant) => {
    const recall = corpus.queries.map((query) => {
      const candidates = query.candidates[variant] ?? [];
      return query.relevant.some((value) =>
        candidates.slice(0, corpus.query_plan.candidate_budget).includes(value),
      )
        ? 1
        : 0;
    });
    const reciprocalRanks = corpus.queries.map((query) => {
      const candidates = query.candidates[variant] ?? [];
      const index = candidates.findIndex((value) =>
        query.relevant.includes(value),
      );
      return index < 0 ? 0 : 1 / (index + 1);
    });
    const supportPrecision = corpus.queries.map(
      (query) => query.support_precision[variant] ?? 0,
    );
    const entityErrors = corpus.queries.map(
      (query) => query.entity_error[variant] ?? 1,
    );
    const latency = corpus.queries.map(
      (query) => query.latency_ms[variant] ?? 0,
    );
    const candidateCount = corpus.queries.map(
      (query) => query.work_units.candidate_count[variant] ?? 0,
    );
    const topologyVisited = corpus.queries.map(
      (query) => query.work_units.topology_visited[variant] ?? 0,
    );
    return {
      algorithm_variant: variant,
      metrics: {
        recall_at_k: {
          k: corpus.query_plan.candidate_budget,
          value: recall.reduce<number>((a, b) => a + b, 0) / recall.length,
        },
        mrr:
          reciprocalRanks.reduce((a, b) => a + b, 0) / reciprocalRanks.length,
        support_provenance_precision:
          supportPrecision.reduce((a, b) => a + b, 0) / supportPrecision.length,
        entity_error_rate:
          entityErrors.reduce((a, b) => a + b, 0) / entityErrors.length,
        latency_ms: {
          p50: percentile(latency, 0.5),
          p95: percentile(latency, 0.95),
        },
        resource_work: {
          candidate_count:
            candidateCount.reduce((a, b) => a + b, 0) / candidateCount.length,
          topology_visited_nodes:
            topologyVisited.reduce((a, b) => a + b, 0) / topologyVisited.length,
        },
      },
    };
  });
  const result = {
    corpus_digest: digest(bytes),
    query_plan_digest: digest(Buffer.from(canonical(corpus.query_plan))),
    query_plan: corpus.query_plan,
    embedding_space_signature: corpus.embedding_space,
    variants,
    runs,
  };
  const outputPath = join(
    process.cwd(),
    "data",
    "research",
    "retrieval-result.json",
  );
  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, JSON.stringify(result, null, 2) + "\n");
  process.stdout.write(
    "RETRIEVAL_RESEARCH_PASS corpus=" +
      result.corpus_digest +
      " variants=" +
      variants.join(",") +
      " output=" +
      outputPath +
      "\n",
  );
}

main().catch((error) => {
  console.error(
    error instanceof Error ? (error.stack ?? error.message) : String(error),
  );
  process.exitCode = 1;
});
