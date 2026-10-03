import { promisify } from "node:util";
import { execFile } from "node:child_process";
import { join } from "node:path";
import { repositoryRoot as repo, workspacePaths } from "../workspace.js";
import { readFile, stat } from "node:fs/promises";
import { resolveLocations } from "../../apps/nous-core/src/locations.js";
import { installRuntime } from "../../apps/nous-core/src/runtime-packs.js";
import {
  installPostgresXmlLibrary,
  preparePostgresXmlLibrary,
} from "./postgres-linux.js";

const installation =
  process.env.NOUS_WAVE_POSTGRES_RUNTIME ??
  join(workspacePaths.installedRuntime, "postgresql");
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
    home: workspacePaths.development,
    installationHome: repo,
    programRoot: workspacePaths.runtime,
    runtimeRoot: workspacePaths.installedRuntime,
  });
  const catalog = JSON.parse(
    await readFile(
      join(workspacePaths.runtime, "manifest/runtimes.json"),
      "utf8",
    ),
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
    join(workspacePaths.runtime, "packs", pack.archive),
  );
} else {
  const prepare = (env: NodeJS.ProcessEnv = process.env) =>
    promisify(execFile)(
      "cargo",
      [
        "run",
        "-p",
        "nous-kernel",
        "--features",
        "dev-runtime",
        "--bin",
        "dev-prepare",
      ],
      { cwd: repo, maxBuffer: 1024 * 1024, env },
    );
  try {
    await prepare();
  } catch (error) {
    const detail = error as Error & { stderr?: string };
    if (
      process.platform !== "linux" ||
      !detail.stderr?.includes("libxml2.so.2")
    )
      throw error;
    const libraryRoot = await preparePostgresXmlLibrary();
    await prepare({
      ...process.env,
      LD_LIBRARY_PATH: [libraryRoot, process.env.LD_LIBRARY_PATH]
        .filter(Boolean)
        .join(":"),
    });
    await installPostgresXmlLibrary(installation, libraryRoot);
  }
}
console.log("Developer PostgreSQL runtime ready");
