import { spawn, execFile, type ChildProcess } from "node:child_process";
import { promisify, parseArgs } from "node:util";
import { once } from "node:events";
import { mkdtemp, cp, mkdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { tmpdir } from "node:os";
import assert from "node:assert/strict";
const execute = promisify(execFile);
const { values } = parseArgs({ options: { bundle: { type: "string" } } });
if (!values.bundle) throw new Error("Provide --bundle <assembled directory>");
const outside = await mkdtemp(join(tmpdir(), "nous-portable-"));
const bundle = join(outside, "installation"),
  home = join(outside, "separate-instance");
await cp(resolve(values.bundle), bundle, { recursive: true });
await mkdir(join(home, "config"), { recursive: true });
await writeFile(
  join(home, "config/nous.toml"),
  (await readFile(join(bundle, "config/nous.toml"), "utf8")).replace(
    "port = 9470",
    "port = 0",
  ),
);
const node = join(bundle, "runtime/node/node.exe"),
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
    [launcher, "serve", "--home", home, "--stop-on-stdin-close"],
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
    [launcher, "--home", home, "--json", ...args],
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
    runRoot: join(home, "run"),
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
    join(home, "instance/postgres.json"),
    "utf8",
  );
  await stop();
  await boot();
  const restarted = await clientModule.connectNousInstance({
    runRoot: join(home, "run"),
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
    await readFile(join(home, "instance/postgres.json"), "utf8"),
    databaseState,
  );
  console.log(
    JSON.stringify({
      result: "PASS",
      platform: process.platform,
      bundle,
      outsideRepository: true,
      developerPath: false,
      sourceLess: true,
      restart: true,
      stableDatabasePort: true,
      subjectId,
      memoryId: memory.memoryId,
      liveModel: "NOT_RUN",
      corpus: "synthetic_portable_wiring",
    }),
  );
} finally {
  await stop();
}
