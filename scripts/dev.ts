import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { initializeConfiguration } from "../apps/nous-core/src/configuration-file.js";
import { resolveLocations } from "../apps/nous-core/src/locations.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const binary = join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
);
if (!existsSync(binary))
  throw new Error("Build the Kernel first: cargo build -p nous-kernel");
const home = join(root, "data", "dev");
await mkdir(join(home, "config"), { recursive: true });
await mkdir(join(home, "secrets"), { recursive: true });
const locator = join(home, "bootstrap.toml");
if (!existsSync(locator))
  await writeFile(
    locator,
    "[paths]\nprogram = " + JSON.stringify(root) + '\nruntime = "runtime"\n',
    { flag: "wx" },
  );
const locations = await resolveLocations({ locator, installationHome: root });
await initializeConfiguration(locations);
const child = spawn(
  process.execPath,
  [
    join(root, "node_modules/tsx/dist/cli.mjs"),
    join(root, "apps/nous-core/src/main.ts"),
    "--locator",
    locator,
    "--development",
  ],
  { stdio: "inherit", windowsHide: true },
);
child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exitCode = code ?? 1;
});
