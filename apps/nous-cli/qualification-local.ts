import { connectNousInstance } from "@nous-wave/client/node";
import { spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import assert from "node:assert/strict";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const tsx = join(root, "node_modules/tsx/dist/cli.mjs");
const kernel =
  process.env.NOUS_WAVE_KERNEL_EXECUTABLE ??
  join(
    root,
    "target/debug",
    process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
  );
const dataRoot = await mkdtemp(join(tmpdir(), "nous-real-consumer-"));
const configPath = join(dataRoot, "core.toml");
await writeFile(
  join(dataRoot, "kernel.toml"),
  `[bootstrap.database]\nmode = "managed"\nurl = ""\nmax_connections = 4\nname = "real_consumer"\ninstall_dir = "postgres-install"\ndata_dir = "postgres-data"\n[bootstrap.object_store]\nbackend = "fs"\nroot = "objects"\nmax_upload_bytes = 1048576\n[bootstrap.serving]\nroot = "serving"\n[settings.capabilities.process]\nmemory = true\n[settings.capabilities.subject_defaults]\nmemory = true\n`,
);
await writeFile(
  configPath,
  `kernel_executable = ${JSON.stringify(kernel)}\nkernel_config = "kernel.toml"\ndata_root = ${JSON.stringify(dataRoot)}\nport = 0\n[[consumers]]\nconsumer_id = "default"\n`,
);
let core: ChildProcess | undefined;
async function boot() {
  core = spawn(
    process.execPath,
    [
      tsx,
      "apps/nous-core/src/main.ts",
      "--config",
      configPath,
      "--stop-on-stdin-close",
    ],
    { cwd: root, stdio: ["pipe", "pipe", "pipe"], windowsHide: true },
  );
  const child = core;
  let errors = "";
  child.stderr!.on("data", (chunk: Buffer) => {
    if (errors.length < 8192) errors += chunk.toString();
  });
  await new Promise<void>((done, fail) => {
    const timer = setTimeout(
      () => fail(new Error(`Core boot timed out: ${errors}`)),
      120_000,
    );
    const cleanup = () => {
      clearTimeout(timer);
      child.off("exit", exited);
      child.off("error", errored);
      child.stdout!.off("data", output);
    };
    const exited = (code: number | null) => {
      cleanup();
      fail(new Error(`Core exited during boot (${code}): ${errors}`));
    };
    const errored = (error: Error) => {
      cleanup();
      fail(error);
    };
    let pending = "";
    const output = (chunk: Buffer) => {
      pending += chunk.toString();
      const lines = pending.split("\n");
      pending = lines.pop() ?? "";
      for (const line of lines) {
        try {
          const value: unknown = JSON.parse(line);
          if (value && typeof value === "object" && "discovery" in value) {
            cleanup();
            done();
            return;
          }
        } catch {
          /* Kernel may print non-JSON startup messages. */
        }
      }
    };
    child.once("exit", exited);
    child.once("error", errored);
    child.stdout!.on("data", output);
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
  const child = spawn(
    process.execPath,
    [
      tsx,
      "apps/nous-cli/src/main.ts",
      "--data-root",
      dataRoot,
      "--json",
      ...args,
    ],
    { cwd: root, stdio: ["ignore", "pipe", "pipe"], windowsHide: true },
  );
  let output = "";
  let errors = "";
  child.stdout!.on("data", (chunk: Buffer) => {
    output += chunk.toString();
  });
  child.stderr!.on("data", (chunk: Buffer) => {
    errors += chunk.toString();
  });
  const exit: unknown[] = await once(child, "exit");
  const code = exit[0];
  if (code !== 0) throw new Error(`CLI ${args[0]} failed: ${errors}`);
  return JSON.parse(output) as Record<string, unknown>;
}
try {
  await boot();
  await cli("status");
  const subject = await cli("subject", "create");
  const subjectId = String(subject.subjectId);
  await cli("session", "open");
  // Synthetic local wiring proof; it is never real corpus or live-model evidence.
  const observation = await cli(
    "observe",
    "text",
    "--text",
    "Local consumer wiring marker: protocol continuity.",
  );
  const occurrenceId = String(observation.occurrenceId);
  const client = await connectNousInstance(dataRoot);
  const extracted = await client.model.deriveMaterial({
    subjectId,
    sourceRegionId: String(observation.sourceRegionId),
    strategy: "description_only",
  });
  assert.equal(extracted.representations[0]?.kind, "extracted_text");
  assert.equal(
    extracted.representations[0]?.inputs[0]?.reference?.value,
    observation.sourceRegionId,
  );
  assert(
    extracted.representations[0]?.producer?.preprocessingIdentity.includes(
      "verified-utf8",
    ),
  );
  const partial = await client.model.deriveMaterial({
    subjectId,
    sourceRegionId: String(observation.sourceRegionId),
    strategy: "describe_then_structure",
  });
  assert.equal(partial.representations.length, 1);
  assert.equal(partial.degradation[0]?.code, "material_structuring_failed");
  assert.equal(
    partial.selectedRepresentationId,
    partial.representations[0]?.representationId,
  );
  assert.equal((await client.material.limits({})).maxUploadBytes, 1048576n);
  await assert.rejects(
    client.artifacts.uploadBytes(subjectId, new Uint8Array(1048577), {
      mediaType: "application/octet-stream",
    }),
    /instance limit/,
  );
  const memory = await client.memory.form({
    subjectId,
    operationId: crypto.randomUUID(),
    input: {
      producer: {
        providerClass: "deterministic-local-qualification",
        operation: "memory_formation_text",
        implementation: "local-wiring-v1",
        preprocessingIdentity: "manual-grounded-source",
        preprocessingRevision: "1",
        configDigest: "local-wiring-v1",
      },
      cognitiveRole: "declarative",
      formationMode: "grounded",
      groundingOccurrenceId: occurrenceId,
      semanticRole: "fact",
      text: "Local consumer wiring marker: protocol continuity.",
      epistemicClass: "observed",
      formedAt: { seconds: BigInt(Math.floor(Date.now() / 1000)), nanos: 0 },
      supports: [
        {
          support: {
            case: "evidence",
            value: {
              occurrenceId,
              supportRole: "direct",
              locator: {
                case: "derivedRepresentationId",
                value: extracted.selectedRepresentationId!,
              },
            },
          },
        },
      ],
    },
  });
  const response = await client.cognition.recall(
    subjectId,
    '"protocol continuity" $memory $limit(5)',
  );
  assert(memory.producerSignatureId);
  const producer = await client.material.producer({
    subjectId,
    id: memory.producerSignatureId,
  });
  assert.equal(producer.operation, "memory_formation_text");
  assert.equal(producer.preprocessingIdentity, "manual-grounded-source");
  assert.match(producer.signatureHash, /^[a-f0-9]{64}$/);
  assert(
    response.hits.some((hit) => hit.revision?.value === memory.revisionId),
  );
  await cli("query", '"protocol continuity" $memory $limit(5)');
  const trace = await cli("trace", `memory:${memory.memoryId}`);
  assert(Array.isArray(trace.sources) && trace.sources.length === 1);
  assert(Array.isArray(trace.derivations) && trace.derivations.length >= 2);
  await cli("use", `memory_revision:${memory.revisionId}`);
  const file = join(dataRoot, "source.txt");
  await writeFile(file, "Streaming artifact consumer proof.");
  const uploaded = await cli("observe", "file", file);
  assert(
    typeof uploaded.occurrenceId === "string" &&
      typeof uploaded.sourceRegionId === "string",
  );
  await stop();
  await boot();
  const restarted = await connectNousInstance(dataRoot);
  const recall = await restarted.cognition.recall(
    subjectId,
    '"protocol continuity" $memory $limit(5)',
  );
  assert(recall.hits.some((hit) => hit.revision?.value === memory.revisionId));
  await cli("session", "show");
  console.log(
    JSON.stringify({
      result: "PASS",
      platform: process.platform,
      subjectId,
      memoryId: memory.memoryId,
      revisionId: memory.revisionId,
      publicBoot: true,
      cli: true,
      upload: true,
      trace: true,
      meaningfulUse: true,
      restart: true,
      liveModel: "NOT_RUN",
      corpus: "synthetic_local_wiring",
    }),
  );
} catch (error) {
  console.error(
    JSON.stringify({
      result: "FAIL",
      stage: "real-consumer-local",
      error: error instanceof Error ? error.message : "Unknown failure",
    }),
  );
  process.exitCode = 1;
} finally {
  await stop();
  await rm(dataRoot, { recursive: true, force: true });
}
