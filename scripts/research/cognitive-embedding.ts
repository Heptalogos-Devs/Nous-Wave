import { setTimeout as delay } from "node:timers/promises";
import { createHash } from "node:crypto";
import { ModelInvocations } from "../../apps/nous-core/src/model/invocations.js";
import { resolveLocations } from "../../apps/nous-core/src/locations.js";
import { parseArgs } from "node:util";
import { readFile, writeFile, rename, mkdir } from "node:fs/promises";
import { resolve, dirname, relative } from "node:path";
import {
  parseConfiguration,
  parseEffectiveConfiguration,
  loadCredentials,
} from "../../apps/nous-core/src/config.js";
import { resolvedEmbedding } from "../../apps/nous-core/src/model/embedding-profile.js";

const { values } = parseArgs({
  options: {
    config: { type: "string" },
    output: { type: "string" },
    needs: { type: "string" },
    vectors: { type: "string" },
    locator: { type: "string" },
    "interval-ms": { type: "string", default: "6000" },
  },
});
if (!values.config || !values.output)
  throw new Error("--config and --output required");
const output = resolve(values.output);
if (!output.startsWith(resolve("data/research") + "/"))
  throw new Error("Output must be under ignored data/research");
const parsed = parseConfiguration(await readFile(values.config, "utf8"), true);
const configuration = parseEffectiveConfiguration(parsed.document).models;
const embedding = resolvedEmbedding(configuration);
if (!embedding) throw new Error("Configured embedding role unavailable");
await writeFile(output, JSON.stringify(embedding, null, 2) + "\n", {
  mode: 0o600,
});
console.log(
  JSON.stringify({
    space: embedding.space.space_hash,
    producer: embedding.producer.signature_hash,
    dimension: embedding.space.dimension,
  }),
);

if (values.needs || values.vectors) {
  if (!values.needs || !values.vectors || !values.locator)
    throw new Error("--needs --vectors --locator required together");
  const vectorsPath = resolve(values.vectors);
  if (!vectorsPath.startsWith(resolve("data/research") + "/"))
    throw new Error("Vectors must be under ignored data/research");
  const locations = await resolveLocations({
    locator: values.locator,
    installationHome: resolve("."),
  });
  await loadCredentials(
    locations,
    parsed.config.host.dotenv_file,
    Object.values(configuration.gateway_profiles).map(
      (profile) => profile.credential_env,
    ),
  );
  const model = await ModelInvocations.create(configuration);
  const needs = JSON.parse(await readFile(values.needs, "utf8")) as {
    text: string;
  }[];
  type Cache = {
    config: typeof embedding;
    vectors?: { text: string; vector: number[] }[];
    vector_files?: string[];
  };
  let cache: Cache = { config: embedding, vectors: [] };
  try {
    cache = JSON.parse(await readFile(vectorsPath, "utf8")) as Cache;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  if (JSON.stringify(cache.config) !== JSON.stringify(embedding))
    throw new Error("Embedding cache identity mismatch");
  const digest = (text: string) =>
    createHash("sha256").update(text).digest("hex");
  const existing = new Set(
    (cache.vectors ?? []).map((entry) => digest(entry.text)),
  );
  for (const file of cache.vector_files ?? []) {
    const entries = JSON.parse(
      await readFile(resolve(dirname(vectorsPath), file), "utf8"),
    ) as { text: string; vector: number[] }[];
    for (const entry of entries) existing.add(digest(entry.text));
  }
  const partsRoot = `${vectorsPath}.parts`;
  await mkdir(partsRoot, { recursive: true, mode: 0o700 });
  cache.vector_files ??= [];
  const publish = async () => {
    const staging = `${vectorsPath}.staging`;
    await writeFile(staging, JSON.stringify(cache) + "\n", { mode: 0o600 });
    await rename(staging, vectorsPath);
  };
  // Migrate an existing flat cache once; future batches publish only a small index.
  if (cache.vectors?.length) {
    const legacy = resolve(partsRoot, "legacy.json");
    await writeFile(legacy, JSON.stringify(cache.vectors) + "\n", {
      mode: 0o600,
    });
    cache.vector_files.push(relative(dirname(vectorsPath), legacy));
    delete cache.vectors;
    await publish();
  } else delete cache.vectors;
  const pending = new Map(
    needs
      .filter((entry) => !existing.has(digest(entry.text)))
      .map((entry) => [digest(entry.text), entry.text]),
  );
  const texts = [...pending.values()];
  const size =
    configuration.model_profiles[configuration.roles.query_embedding!.model]!
      .embedding!.max_batch_size;
  const intervalMs = Number(values["interval-ms"]);
  if (!Number.isFinite(intervalMs) || intervalMs < 0 || intervalMs > 60000)
    throw new Error("Invalid embedding interval");
  for (let offset = 0; offset < texts.length; offset += size) {
    await delay(intervalMs);
    const batch = texts.slice(offset, offset + size);
    const result = await model.embeddingBatch(
      batch,
      embedding.space.model_identity,
    );
    const entries = batch.map((text, i) => ({
      text,
      vector: result.value[i]!,
    }));
    const part = resolve(
      partsRoot,
      `batch-${String(cache.vector_files.length).padStart(6, "0")}.json`,
    );
    const partStaging = `${part}.staging`;
    await writeFile(partStaging, JSON.stringify(entries) + "\n", {
      mode: 0o600,
    });
    await rename(partStaging, part);
    cache.vector_files.push(relative(dirname(vectorsPath), part));
    for (const text of batch) existing.add(digest(text));
    await publish();
    console.log(
      JSON.stringify({
        completed: Math.min(offset + size, texts.length),
        pending: texts.length,
        cached: existing.size,
      }),
    );
  }
}
