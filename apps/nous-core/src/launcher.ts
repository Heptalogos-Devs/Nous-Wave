import { parseArgs } from "node:util";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { parse } from "smol-toml";
import { resolveLocations } from "./locations.js";
import {
  installRuntime,
  listRuntimes,
  verifyRuntime,
} from "./runtime-packs.js";

async function main() {
  const args = process.argv.slice(2);
  const profileArgs: string[] = [];
  const forwarded: string[] = [];
  for (let index = 0; index < args.length; index++) {
    if (args[index] === "--home" || args[index] === "--locator") {
      if (!args[index + 1])
        throw new Error("Missing instance location argument");
      profileArgs.push(args[index]!, args[++index]!);
    } else forwarded.push(args[index]!);
  }
  const { values } = parseArgs({
    args: profileArgs,
    options: { home: { type: "string" }, locator: { type: "string" } },
  });
  const installationHome = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../..",
  );
  const locations = await resolveLocations({
    home: values.home,
    locator: values.locator,
    installationHome,
  });
  const [command, action, name] = forwarded;
  if (command === "runtime") {
    if (action === "list")
      console.log(JSON.stringify(await listRuntimes(locations)));
    else if (action === "verify" && name) {
      const pack = await verifyRuntime(locations, name);
      console.log(
        JSON.stringify({
          component: pack.component,
          version: pack.version,
          status: "PASS",
        }),
      );
    } else if (action === "install" && name) {
      const { values: install } = parseArgs({
        args: forwarded.slice(3),
        options: { pack: { type: "string" } },
      });
      const controller = new AbortController();
      process.once("SIGINT", () => controller.abort());
      console.log(
        JSON.stringify(
          await installRuntime(
            locations,
            name,
            install.pack,
            controller.signal,
          ),
        ),
      );
    } else
      throw new Error(
        "Use runtime list | verify <component> | install <component> [--pack <archive>]",
      );
    return;
  }
  if (command === "serve") {
    await mkdir(locations.config, { recursive: true });
    try {
      await writeFile(
        join(locations.config, "nous.toml"),
        await readFile(join(locations.program, "templates", "nous.toml")),
        { flag: "wx" },
      );
    } catch (error) {
      if (
        (error as NodeJS.ErrnoException).code !== "EEXIST" &&
        (error as NodeJS.ErrnoException).code !== "ENOENT"
      )
        throw error;
    }
  }
  const config = parse(
    await readFile(join(locations.config, "nous.toml"), "utf8"),
  );
  const development = config.deployment === "development";
  const serve = command === "serve";
  const entry = development
    ? join(
        locations.program,
        "apps",
        serve ? "nous-core" : "nous-cli",
        "src",
        "main.ts",
      )
    : join(locations.program, serve ? "core" : "cli", "main.js");
  const nodeArgs = development
    ? [join(locations.program, "node_modules", "tsx", "dist", "cli.mjs"), entry]
    : [entry];
  if (serve) nodeArgs.push(...profileArgs, ...forwarded.slice(1));
  else
    nodeArgs.push(
      "--run-root",
      locations.run,
      "--instance-root",
      locations.instance,
      ...forwarded,
    );
  const child = spawn(process.execPath, nodeArgs, {
    stdio: "inherit",
    windowsHide: true,
  });
  for (const signal of ["SIGINT", "SIGTERM"] as const)
    process.once(signal, () => child.kill(signal));
  child.once("error", (error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
  child.once("exit", (code) => {
    process.exitCode = code ?? 1;
  });
}
main().catch(() => {
  console.error(
    "Nous launcher failed; check instance configuration and runtime installation",
  );
  process.exitCode = 1;
});
