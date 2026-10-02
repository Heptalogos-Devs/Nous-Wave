import { spawn, execFile, type ChildProcess } from "node:child_process";
import { promisify, parseArgs } from "node:util";
import { once } from "node:events";
import { mkdtemp, mkdir, readFile, writeFile, rename } from "node:fs/promises";
import { join, resolve, sep } from "node:path";
import { createReadStream } from "node:fs";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";
import { tmpdir } from "node:os";
import assert from "node:assert/strict";
const execute = promisify(execFile);
const { values } = parseArgs({
  options: {
    bundle: { type: "string" },
    layout: { type: "string", default: "home" },
    relocate: { type: "boolean", default: false },
  },
});
if (!values.bundle || !values.bundle.endsWith(".zip"))
  throw new Error("Provide --bundle <exact portable ZIP>");
const outside = await mkdtemp(join(tmpdir(), "nous-portable-"));
if (!["home", "colocated", "locator"].includes(values.layout))
  throw new Error("Unknown portable layout");
if (values.layout === "colocated" && values.relocate)
  throw new Error("Relocation uses an independent instance");
let bundle = join(outside, "installation");
const home =
  values.layout === "colocated" ? bundle : join(outside, "separate-instance");
const locator = join(outside, "locator/bootstrap.toml");
const configRoot =
  values.layout === "locator"
    ? join(outside, "configuration-volume")
    : join(home, "config");
const runRoot =
  values.layout === "locator"
    ? join(outside, "runtime-state")
    : join(home, "run");
const instanceRoot =
  values.layout === "locator"
    ? join(outside, "identity-volume")
    : join(home, "instance");
const locationArgs =
  values.layout === "locator" ? ["--locator", locator] : ["--home", home];
async function writeLocator() {
  if (values.layout !== "locator") return;
  await mkdir(join(outside, "locator"), { recursive: true });
  const paths = {
    program: join(bundle, "program"),
    runtime: join(bundle, "runtime"),
    config: configRoot,
    run: runRoot,
    instance: instanceRoot,
    data: join(outside, "database-volume"),
    blob: join(outside, "blob-volume"),
    cache: join(outside, "cache-volume"),
    secret: join(outside, "secret-volume"),
    log: join(outside, "log-volume"),
    temp: join(outside, "temp-volume"),
    backup: join(outside, "backup-volume"),
  };
  await writeFile(
    locator,
    "[paths]\n" +
      Object.entries(paths)
        .map(
          ([key, value]) =>
            `${key} = ${JSON.stringify(value.replaceAll("\\", "/"))}`,
        )
        .join("\n"),
  );
}
const archive = resolve(values.bundle);
const literal = (value: string) => "'" + value.replaceAll("'", "''") + "'";
await execute(
  join(
    process.env.SystemRoot!,
    "System32/WindowsPowerShell/v1.0/powershell.exe",
  ),
  [
    "-NoProfile",
    "-Command",
    `Expand-Archive -LiteralPath ${literal(archive)} -DestinationPath ${literal(bundle)}`,
  ],
  { windowsHide: true },
);
const checksums = JSON.parse(
  await readFile(join(bundle, "manifest/checksums.json"), "utf8"),
) as { path: string; sha256: string }[];
for (const item of checksums) {
  const path = resolve(bundle, item.path);
  assert(path.startsWith(resolve(bundle) + sep));
  const hash = createHash("sha256");
  for await (const bytes of createReadStream(path))
    hash.update(bytes as Buffer);
  assert.equal(hash.digest("hex"), item.sha256, item.path);
}
const components = JSON.parse(
  await readFile(join(bundle, "manifest/components.json"), "utf8"),
) as { missing_cargo_notices: string[] };
assert.deepEqual(components.missing_cargo_notices, []);
const sbom = JSON.parse(
  await readFile(join(bundle, "manifest/sbom.spdx.json"), "utf8"),
) as {
  packages: { SPDXID: string }[];
  relationships: { spdxElementId: string; relatedSpdxElement: string }[];
};
const ids = new Set([
  "SPDXRef-DOCUMENT",
  ...sbom.packages.map((p) => p.SPDXID),
]);
assert.equal(ids.size, sbom.packages.length + 1);
for (const relation of sbom.relationships)
  assert(
    ids.has(relation.spdxElementId) && ids.has(relation.relatedSpdxElement),
  );
for (const path of [
  "licenses/THIRD_PARTY_NOTICES.md",
  "licenses/Nous-Wave-MIT.txt",
  "licenses/native/LLVM-LICENSE.txt",
  "licenses/native/Rust-library-COPYRIGHT.html",
])
  assert((await readFile(join(bundle, path))).length > 0);
await writeLocator();
await mkdir(configRoot, { recursive: true });
await writeFile(
  join(configRoot, "nous.toml"),
  (await readFile(join(bundle, "config/nous.toml"), "utf8")).replace(
    "port = 9470",
    "port = 0",
  ),
);
let node = join(bundle, "runtime/node/node.exe"),
  launcher = join(bundle, "program/core/launcher.js");
const env = {
  ...process.env,
  PATH: join(process.env.SystemRoot!, "System32"),
  NODE_PATH: "",
  NODE_OPTIONS: "",
};
let core: ChildProcess | undefined;
async function boot() {
  core = spawn(
    node,
    [launcher, "serve", ...locationArgs, "--stop-on-stdin-close"],
    { cwd: outside, env, stdio: ["pipe", "pipe", "pipe"], windowsHide: true },
  );
  const child = core;
  let errors = "";
  child.stderr!.on("data", (bytes: Buffer) => {
    if (errors.length < 4096) errors += bytes.toString();
  });
  await new Promise<void>((done, failed) => {
    const timer = setTimeout(
      () => failed(new Error("Portable boot timed out: " + errors)),
      120000,
    );
    child.once("exit", (code) => {
      clearTimeout(timer);
      failed(new Error("Portable boot exited " + code + ": " + errors));
    });
    child.once("error", (error) => {
      clearTimeout(timer);
      failed(error);
    });
    let pending = "";
    child.stdout!.on("data", (bytes: Buffer) => {
      pending += bytes.toString();
      if (pending.includes('"discovery"')) {
        clearTimeout(timer);
        done();
      }
    });
  });
}
async function stop() {
  if (!core || core.exitCode !== null) return;
  const exited = once(core, "exit");
  core.stdin!.end();
  await exited;
  core = undefined;
}
async function cli(...args: string[]) {
  const result = await execute(
    node,
    [launcher, ...locationArgs, "--json", ...args],
    { cwd: outside, env, windowsHide: true, maxBuffer: 1048576 },
  );
  return JSON.parse(result.stdout) as Record<string, unknown>;
}
try {
  await boot();
  const status = await cli("status");
  assert(status.status);
  const subject = await cli("subject", "create");
  const subjectId = String(subject.subjectId);
  await cli("session", "open");
  const observation = await cli(
    "observe",
    "text",
    "--text",
    "Synthetic portable wiring: stable source identity.",
  );
  const clientModule = (await import(
    pathToFileURL(join(bundle, "program/client/node.js")).href
  )) as typeof import("@nous-wave/client/node");
  const client = await clientModule.connectNousInstance({
    runRoot,
  });
  const occurrenceId = String(observation.occurrenceId);
  const derived = await client.model.deriveMaterial({
    subjectId,
    sourceRegionId: String(observation.sourceRegionId),
    strategy: "description_only",
  });
  const memory = await client.memory.form({
    subjectId,
    operationId: crypto.randomUUID(),
    input: {
      cognitiveRole: "declarative",
      formationMode: "grounded",
      groundingOccurrenceId: occurrenceId,
      semanticRole: "fact",
      text: "Synthetic portable wiring: stable source identity.",
      epistemicClass: "observed",
      formedAt: {
        $typeName: "google.protobuf.Timestamp",
        seconds: BigInt(Math.floor(Date.now() / 1000)),
        nanos: 0,
      },
      supports: [
        {
          support: {
            case: "evidence",
            value: {
              occurrenceId,
              supportRole: "direct",
              locator: {
                case: "derivedRepresentationId",
                value: derived.selectedRepresentationId!,
              },
            },
          },
        },
      ],
    },
  });
  const response = await client.cognition.recall(
    subjectId,
    '"stable source identity" $memory $limit(5)',
  );
  assert(
    response.hits.some(
      (hit: { revision?: { value: string } }) =>
        hit.revision?.value === memory.revisionId,
    ),
  );
  await cli("trace", `memory:${memory.memoryId}`);
  await cli("use", `memory_revision:${memory.revisionId}`);
  const databaseState = await readFile(
    join(instanceRoot, "postgres.json"),
    "utf8",
  );
  await stop();
  if (values.relocate) {
    const moved = join(outside, "moved-installation");
    await rename(bundle, moved);
    bundle = moved;
    node = join(bundle, "runtime/node/node.exe");
    launcher = join(bundle, "program/core/launcher.js");
    await writeLocator();
  }
  await boot();
  const restarted = await clientModule.connectNousInstance({
    runRoot,
  });
  const recalled = await restarted.cognition.recall(
    subjectId,
    '"stable source identity" $memory $limit(5)',
  );
  assert(
    recalled.hits.some(
      (hit: { revision?: { value: string } }) =>
        hit.revision?.value === memory.revisionId,
    ),
  );
  assert.equal(
    await readFile(join(instanceRoot, "postgres.json"), "utf8"),
    databaseState,
  );
  await stop();
  const postgresPack = join(bundle, "runtime/postgresql");
  const hiddenPack = join(bundle, "runtime/postgresql-missing");
  assert(
    postgresPack.startsWith(bundle + "\\") ||
      postgresPack.startsWith(bundle + "/"),
  );
  assert(
    hiddenPack.startsWith(bundle + "\\") || hiddenPack.startsWith(bundle + "/"),
  );
  await rename(postgresPack, hiddenPack);
  try {
    await assert.rejects(
      execute(
        node,
        [launcher, "serve", ...locationArgs, "--stop-on-stdin-close"],
        { cwd: outside, env, windowsHide: true, timeout: 30000 },
      ),
      (error: unknown) =>
        error instanceof Error &&
        "stderr" in error &&
        /runtime install postgresql/.test(String(error.stderr)),
    );
    assert.equal(
      await readFile(join(instanceRoot, "postgres.json"), "utf8"),
      databaseState,
    );
  } finally {
    await rename(hiddenPack, postgresPack);
  }
  console.log(
    JSON.stringify({
      result: "PASS",
      platform: process.platform,
      bundle,
      archive,
      layout: values.layout,
      relocated: values.relocate,
      outsideRepository: true,
      developerPath: false,
      sourceLess: true,
      restart: true,
      stableDatabasePort: true,
      missingPackRejected: true,
      subjectId,
      memoryId: memory.memoryId,
      liveModel: "NOT_RUN",
      corpus: "synthetic_portable_wiring",
    }),
  );
} finally {
  await stop();
}
