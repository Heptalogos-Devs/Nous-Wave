import { spawn } from "node:child_process";
import { join } from "node:path";
import { repositoryRoot as root, developmentLocator } from "../workspace.js";

const args = process.argv.slice(2);
const located = args.includes("--home") || args.includes("--locator");
const child = spawn(
  process.execPath,
  [
    join(root, "node_modules/tsx/dist/cli.mjs"),
    join(root, "apps/nous-core/src/launcher.ts"),
    "--development",
    ...(located ? [] : ["--locator", await developmentLocator()]),
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
