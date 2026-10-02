import { spawn } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const located = args.includes("--home") || args.includes("--locator");
const child = spawn(
  process.execPath,
  [
    join(root, "node_modules/tsx/dist/cli.mjs"),
    join(root, "apps/nous-core/src/launcher.ts"),
    "--development",
    ...(located ? [] : ["--locator", join(root, "data/dev/bootstrap.toml")]),
    ...args,
  ],
  { stdio: "inherit", windowsHide: true },
);
child.once("exit", (code) => {
  process.exitCode = code ?? 1;
});
child.once("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
});
