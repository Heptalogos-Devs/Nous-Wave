import { parseArgs } from "node:util";
import { randomBytes } from "node:crypto";
import { loadConfig } from "./config.js";
import { claimInstance } from "./discovery.js";
import { startKernel } from "./process.js";
import { createCore } from "./server.js";
import { ModelRuntime } from "./model/runtime.js";
import { resolveLocations } from "./locations.js";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFile } from "node:fs/promises";
import { stringify } from "smol-toml";
import { verifyRuntime } from "./runtime-packs.js";
import { ModelBudget } from "./model/budget.js";

async function main() {
  const { values } = parseArgs({
    options: {
      home: { type: "string" },
      locator: { type: "string" },
      "stop-on-stdin-close": { type: "boolean", default: false },
    },
  });
  const locations = await resolveLocations({
    home: values.home,
    locator: values.locator,
    installationHome: resolve(dirname(fileURLToPath(import.meta.url)), "../.."),
  });
  const config = await loadConfig(locations);
  if (config.deployment === "portable") {
    await verifyRuntime(locations, "node");
    if (config.kernelBootstrap.bootstrap.database.mode === "managed_private")
      await verifyRuntime(locations, "postgresql");
    if (!config.externalFfmpeg) await verifyRuntime(locations, "ffmpeg");
  }
  const instance = await claimInstance(locations);
  let kernel: Awaited<ReturnType<typeof startKernel>> | undefined;
  let app: Awaited<ReturnType<typeof createCore>> | undefined;
  try {
    const kernelConfig = join(locations.run, "kernel-bootstrap.toml");
    await writeFile(kernelConfig, stringify(config.kernelBootstrap), {
      mode: 0o600,
    });
    kernel = await startKernel(config.kernelExecutable, kernelConfig, {
      credentialEnvironments: Object.values(config.models.gateway_profiles).map(
        (gateway) => gateway.credential_env,
      ),
    });
    const token = randomBytes(32).toString("hex");
    app = await createCore({
      kernel: kernel.client,
      token,
      consumers: config.consumers,
      models: await ModelRuntime.fromConfig(
        config.models,
        join(locations.program, "prompts"),
        join(locations.config, "prompts"),
        new ModelBudget(
          join(locations.instance, "model-budget.json"),
          config.modelBudget,
        ),
      ),
    });
    const endpoint = await app.listen({ host: "127.0.0.1", port: config.port });
    await instance.publish(endpoint, token);
    console.log(JSON.stringify({ endpoint, discovery: instance.path }));
    await new Promise<void>((stopped) => {
      process.once("SIGINT", stopped);
      process.once("SIGTERM", stopped);
      if (values["stop-on-stdin-close"]) {
        process.stdin.once("end", stopped);
        process.stdin.resume();
      }
    });
  } finally {
    await app?.close();
    await kernel?.stop();
    await instance.release();
  }
}
main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
