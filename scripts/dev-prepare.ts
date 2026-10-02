import { promisify } from "node:util";
import { execFile } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readFile, stat } from "node:fs/promises";
import { resolveLocations } from "../apps/nous-core/src/locations.js";
import { installRuntime } from "../apps/nous-core/src/runtime-packs.js";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const installation =
  process.env.NOUS_WAVE_POSTGRES_RUNTIME ??
  join(repo, "data/dev/runtime/postgresql");
const executable = join(
  installation,
  "bin",
  process.platform === "win32" ? "postgres.exe" : "postgres",
);
if (await stat(executable).catch(() => undefined)) {
  const version = await promisify(execFile)(executable, ["--version"], {
    windowsHide: true,
  });
  if (!/PostgreSQL\) 18\.6(?:\s|$)/.test(version.stdout))
    throw new Error("Developer PostgreSQL must be version 18.6");
} else if (process.platform === "win32") {
  const locations = await resolveLocations({
    home: join(repo, "data/dev"),
    installationHome: repo,
    programRoot: repo,
    runtimeRoot: join(repo, "data/dev/runtime"),
  });
  const catalog = JSON.parse(
    await readFile(join(repo, "manifest/runtimes.json"), "utf8"),
  ) as {
    packs: {
      component: string;
      platform: string;
      arch: string;
      archive: string;
    }[];
  };
  const pack = catalog.packs.find(
    (p) =>
      p.component === "postgresql" &&
      p.platform === process.platform &&
      p.arch === process.arch,
  );
  if (!pack)
    throw new Error("Prepare a PostgreSQL runtime pack and catalog first");
  await installRuntime(
    locations,
    "postgresql",
    join(repo, "packs", pack.archive),
  );
} else {
  await promisify(execFile)(
    "cargo",
    ["run", "-p", "nous-kernel", "--bin", "dev-prepare"],
    { cwd: repo, maxBuffer: 1024 * 1024 },
  );
}
console.log("Developer PostgreSQL runtime ready");
