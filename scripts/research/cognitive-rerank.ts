/** Serial research stdio adapter; production invocation owns the model boundary. */
import { parseArgs } from "node:util";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";
import { ModelInvocations } from "../../apps/nous-core/src/model/invocations.js";
import { resolveLocations } from "../../apps/nous-core/src/locations.js";
import {
  parseConfiguration,
  parseEffectiveConfiguration,
  loadCredentials,
} from "../../apps/nous-core/src/config.js";
const { values } = parseArgs({
  options: { config: { type: "string" }, locator: { type: "string" } },
});
if (!values.config || !values.locator)
  throw new Error("--config --locator required");
const parsed = parseConfiguration(await readFile(values.config, "utf8"), true);
const config = parseEffectiveConfiguration(parsed.document).models;
const locations = await resolveLocations({
  locator: values.locator,
  installationHome: resolve("."),
});
await loadCredentials(
  locations,
  parsed.config.host.dotenv_file,
  Object.values(config.gateway_profiles).map((p) => p.credential_env),
);
const model = await ModelInvocations.create(config);
const snapshot = model.snapshot("query_rerank");
if (!snapshot) throw new Error("Configured rerank role unavailable");
process.stdout.write(
  JSON.stringify({
    ready: true,
    profile_digest: snapshot.profileDigest,
    config_digest: snapshot.configDigest,
  }) + "\n",
);
for await (const line of createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
})) {
  try {
    const request = JSON.parse(line) as { query: string; documents: string[] };
    await delay(6000);
    const start = performance.now();
    const result = await model.rerank(
      request.query,
      request.documents,
      request.documents.length,
    );
    process.stdout.write(
      JSON.stringify({
        order: result.value,
        producer: result.producerMetadata,
        latency_ms: performance.now() - start,
        error: null,
      }) + "\n",
    );
  } catch {
    process.stdout.write(
      JSON.stringify({
        order: [],
        error: "production_rerank_unavailable_or_invalid",
      }) + "\n",
    );
  }
}
