import { parseArgs } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  parseConfiguration,
  parseEffectiveConfiguration,
} from "../../apps/nous-core/src/config.js";
import { resolvedEmbedding } from "../../apps/nous-core/src/model/embedding-profile.js";
import { launchFunctionalRuntime } from "./functional-host.js";
import { ExecutionBudget } from "./execution-budget.js";
import { executionLimits, budgetOptions } from "./functional-plan.js";
const { values } = parseArgs({
  options: {
    ...budgetOptions,
    plan: { type: "boolean" },
    config: { type: "string" },
    input: { type: "string" },
    root: { type: "string" },
    cache: { type: "string" },
    output: { type: "string" },
  },
});
if (
  !values.plan ||
  !values.config ||
  !values.input ||
  !values.root ||
  !values.cache ||
  !values.output
)
  throw new Error("--plan --config --input --root --cache --output required");
const root = resolve(values.root),
  cachePath = resolve(values.cache),
  output = resolve(values.output);
for (const path of [root, cachePath, output])
  if (!path.startsWith(resolve("data/research") + "/"))
    throw new Error("Generated research paths must remain under data/research");
const { document } = parseConfiguration(
  await readFile(values.config, "utf8"),
  true,
);
const configuration = parseEffectiveConfiguration(document).models;
const config = resolvedEmbedding(configuration);
if (!config) throw new Error("Embedding role metadata unavailable");
let cache: {
  config: typeof config;
  vectors: { text: string; vector: number[] }[];
} = { config, vectors: [] };
try {
  cache = JSON.parse(await readFile(cachePath, "utf8")) as typeof cache;
  if (JSON.stringify(cache.config) !== JSON.stringify(config))
    throw new Error(
      "Cache space/producer differs from the configured embedding role",
    );
} catch (error) {
  if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
}
const inputs = (
  JSON.parse(await readFile(values.input, "utf8")) as {
    queries: { key: string; as_of: string; query: { subject: string } }[];
  }
).queries;
if (inputs.length > 40) throw new Error("Prepared query envelope exceeded");
const limits = executionLimits(values);
const identity = createHash("sha256")
  .update(
    JSON.stringify({
      config,
      inputs,
      cacheIdentity: cache.config,
      output,
      limits,
    }),
  )
  .digest("hex");
await mkdir(resolve(output, ".."), { recursive: true });
const budget = await ExecutionBudget.open(
  output + ".budget.json",
  identity,
  limits,
);
let host: Awaited<ReturnType<typeof launchFunctionalRuntime>> | undefined;
try {
  await budget.reserve({
    newArtifactBytes: 32 * 1024 * 1024,
    newServingGenerations: new Set(inputs.map((input) => input.query.subject))
      .size,
  });
  await mkdir(resolve(cachePath, ".."), { recursive: true });
  await writeFile(cachePath, JSON.stringify(cache, null, 2) + "\n", {
    mode: 0o600,
  });
  host = await launchFunctionalRuntime(
    root,
    randomUUID() + randomUUID(),
    budget.signal,
    false,
    cachePath,
  );
  const checkpoint = async () => {
    const metrics = await host!.control({ command: "metrics" });
    await budget.observeArtifactBytes(
      Math.max(0, Number(metrics.runBytes) - host!.baselineBytes),
    );
    await budget.observeServingGenerations(
      Math.max(0, Number(metrics.generations) - host!.baselineGenerations),
    );
    budget.signal.throwIfAborted();
  };
  await checkpoint();
  const queries: unknown[] = [];
  const texts = new Set<string>();
  for (const input of inputs) {
    await host.control({
      command: "advance",
      subject: input.query.subject,
      instant: input.as_of,
    });
    const prepared = await host.control({
      command: "prepare",
      key: input.key,
      query: input.query,
    });
    await checkpoint();
    queries.push(prepared);
    texts.add((prepared.representation as { text: string }).text);
  }
  const materials = await host.control({
    command: "embedding_needs",
    subjects: [...new Set(inputs.map((input) => input.query.subject))],
  });
  for (const source of materials.sources as { text: string }[])
    texts.add(source.text);
  const known = new Set(cache.vectors.map((entry) => entry.text));
  const missing = [...texts].filter((text) => !known.has(text));
  const batch =
    configuration.model_profiles[configuration.roles.query_embedding!.model]!
      .embedding!.max_batch_size;
  const report = {
    suite: "prepared-semantic-cache",
    subjects: new Set(inputs.map((input) => input.query.subject)).size,
    queries: inputs.length,
    profiles: [
      "baseline-rrf",
      "nous-node-potential-v1",
      "vcp-dtsc-v9.2.1-adapter-v1",
      "vcp-rivermemo-v3.1-adapter-v1",
    ],
    uniqueTexts: texts.size,
    existingEmbeddings: [...texts].filter((text) => known.has(text)).length,
    embeddingMisses: missing.length,
    estimatedEmbeddingRequests: Math.ceil(missing.length / batch),
    rerankRequests: 0,
    sharedServingAssetsReused: "existing lexical/exact families",
    newAlgorithmAssets: 2,
    newServingGenerations:
      new Set(inputs.map((input) => input.query.subject)).size * 4,
    estimatedNewArtifactBytes:
      missing.length * config.space.dimension * 16 + 1024 * 1024,
    externalDownloads: 0,
    estimatedRuntimeClass: "bounded small corpus",
    config,
    queryPreparations: queries,
    sourceMaterials: materials.sources,
    missingTexts: missing,
    inputDigest: createHash("sha256")
      .update(JSON.stringify({ config, queries, texts: [...texts] }))
      .digest("hex"),
    metrics: await host.control({ command: "metrics" }),
    budget: budget.snapshot(),
  };
  await mkdir(resolve(output, ".."), { recursive: true });
  const reportBytes = JSON.stringify(report, null, 2) + "\n";
  await budget.reserve({ newArtifactBytes: Buffer.byteLength(reportBytes) });
  await writeFile(output, reportBytes, { mode: 0o600 });
  report.budget = budget.snapshot();
  console.log(
    JSON.stringify(
      {
        ...report,
        config: undefined,
        queryPreparations: undefined,
        sourceMaterials: undefined,
        missingTexts: undefined,
      },
      null,
      2,
    ),
  );
} catch (error) {
  await budget.stop(
    error instanceof Error ? error.message : "Embedding preflight failed",
  );
  throw error;
} finally {
  await host?.close();
  await budget.close();
}
