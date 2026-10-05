import { parseArgs, parseEnv } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { readFile, writeFile, rename } from "node:fs/promises";
import { resolve } from "node:path";
import {
  parseConfiguration,
  parseEffectiveConfiguration,
} from "../../apps/nous-core/src/config.js";
import { resolvedEmbedding } from "../../apps/nous-core/src/model/embedding-profile.js";
import { ExecutionBudget } from "./execution-budget.js";
import { budgetOptions, executionLimits } from "./functional-plan.js";
import { ModelInvocations } from "../../apps/nous-core/src/model/invocations.js";
const { values } = parseArgs({
  options: {
    ...budgetOptions,
    plan: { type: "boolean" },
    input: { type: "string" },
    cache: { type: "string" },
    config: { type: "string" },
    "env-file": { type: "string" },
    "execution-id": { type: "string", default: "cache-1" },
  },
});
if (!values.input || !values.cache || !values.config)
  throw new Error(
    "--input exact embedding plan, --cache and --config required",
  );
const path = resolve(values.cache);
if (!path.startsWith(resolve("data/research") + "/"))
  throw new Error("Research cache required");
const source = JSON.parse(await readFile(values.input, "utf8")) as {
  config: { space: { dimension: number }; producer: unknown };
  missingTexts: string[];
  inputDigest: string;
};
const { document } = parseConfiguration(
  await readFile(values.config, "utf8"),
  true,
);
const configuration = parseEffectiveConfiguration(document).models;
const resolved = resolvedEmbedding(configuration);
if (!resolved || JSON.stringify(resolved) !== JSON.stringify(source.config))
  throw new Error("Configured embedding identity changed since preflight");
const cache = JSON.parse(await readFile(path, "utf8")) as {
  config: typeof resolved;
  vectors: { text: string; vector: number[] }[];
};
if (JSON.stringify(cache.config) !== JSON.stringify(resolved))
  throw new Error("Embedding cache identity mismatch");
const known = new Set(cache.vectors.map((entry) => entry.text));
const missing = [...new Set(source.missingTexts)].filter(
  (text) => !known.has(text),
);
const profile =
  configuration.model_profiles[configuration.roles.query_embedding!.model]!;
const gateway = configuration.gateway_profiles[profile.gateway]!;
const batch = profile.embedding!.max_batch_size;
const limits = executionLimits(values);
const plan = {
  inputDigest: source.inputDigest,
  uniqueTexts: source.missingTexts.length,
  existingEmbeddings: known.size,
  embeddingMisses: missing.length,
  estimatedEmbeddingRequests: Math.ceil(missing.length / batch),
  rerankRequests: 0,
  newServingGenerations: 0,
  newAlgorithmAssets: 0,
  newArtifactBytes: missing.length * resolved.space.dimension * 16,
  externalDownloads: 0,
  limits,
};
const identity = createHash("sha256")
  .update(JSON.stringify({ plan, config: resolved, texts: missing }))
  .digest("hex");
const base = path + "." + values["execution-id"];
const planPath = base + ".plan.json";
if (values.plan) {
  await writeFile(
    planPath,
    JSON.stringify({ identity, plan }, null, 2) + "\n",
    { mode: 0o600 },
  );
  console.log(JSON.stringify(plan, null, 2));
} else await run();
async function run() {
  const approved = JSON.parse(await readFile(planPath, "utf8")) as {
    identity: string;
  };
  if (approved.identity !== identity)
    throw new Error("Run --plan with exact cache state and limits");
  if (values["env-file"]) {
    const env = parseEnv(await readFile(values["env-file"], "utf8"));
    if (!process.env[gateway.credential_env] && env[gateway.credential_env])
      process.env[gateway.credential_env] = env[gateway.credential_env];
  }
  const credential = process.env[gateway.credential_env];
  if (!credential) throw new Error("Configured gateway credential unavailable");
  const budget = await ExecutionBudget.open(
    base + ".budget.json",
    identity,
    limits,
  );
  const models = await ModelInvocations.create(configuration);
  let completed = 0;
  try {
    for (let offset = 0; offset < missing.length; offset += batch) {
      const texts = missing.slice(offset, offset + batch);
      await budget.reserve({
        newArtifactBytes: texts.length * resolved!.space.dimension * 16,
      });
      // embeddingBatch has no SDK retries and this chunk is one configured provider batch.
      await budget.reserve({
        providerCalls: 1,
        newEmbeddingItems: texts.length,
      });
      const result = await models.embeddingBatch(
        texts,
        profile.model,
        budget.signal,
      );
      for (let index = 0; index < texts.length; index++) {
        cache.vectors.push({
          text: texts[index]!,
          vector: result.value[index]!,
        });
        completed++;
      }
      const output = JSON.stringify(cache);
      const temporary = path + "." + randomUUID() + ".tmp";
      await writeFile(temporary, output + "\n", { flag: "wx", mode: 0o600 });
      await rename(temporary, path);
      await budget.observeArtifactBytes(Buffer.byteLength(output));
      budget.signal.throwIfAborted();
    }
  } catch (error) {
    await budget.stop(
      budget.snapshot().stopReason ??
        (error instanceof Error ? error.message : "Embedding cache failed"),
    );
  } finally {
    await budget.close();
    await writeFile(
      base + ".result.json",
      JSON.stringify({ plan, budget: budget.snapshot(), completed }, null, 2) +
        "\n",
      { mode: 0o600 },
    );
  }
  console.log(
    JSON.stringify({
      completed,
      used: budget.snapshot().used,
      stopReason: budget.snapshot().stopReason,
    }),
  );
}
