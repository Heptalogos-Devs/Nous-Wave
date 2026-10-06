import { CONFIG_REVISION } from "../../apps/nous-core/src/config.js";
import { connectNousInstance } from "@nous-wave/client/node";
import { spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { writeFile, appendFile, rm, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { workspaceTemp } from "../workspace.js";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createServer } from "node:http";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const tsx = join(root, "node_modules/tsx/dist/cli.mjs");
const kernel =
  process.env.NOUS_WAVE_KERNEL_EXECUTABLE ??
  join(
    root,
    "target/debug",
    process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
  );
const dataRoot = await workspaceTemp("smoke", "model-");
const configPath = join(dataRoot, "bootstrap.toml");
// Deterministic local provider contract wiring, never live-model or corpus evidence.
let structuredEnabled = false;
let providerCalls = 0;
let resourceProviderCalls = 0;
let resourceContent = "RAGFlow external chunk contract marker.";
let formationEnabled = false;
const provider = createServer((request, response) => {
  void (async () => {
    const chunks: Buffer[] = [];
    for await (const chunk of request) {
      const bytes: unknown = chunk;
      if (!(bytes instanceof Uint8Array))
        throw new Error("Invalid fixture request bytes");
      chunks.push(Buffer.from(bytes));
    }
    if (request.url?.startsWith("/api/v1/")) {
      resourceProviderCalls++;
      response.setHeader("Content-Type", "application/json");
      const content = resourceContent;
      const data =
        request.method === "POST"
          ? {
              chunks: [
                {
                  id: "chunk1",
                  dataset_id: "dataset1",
                  document_id: "document1",
                  content,
                  document_keyword: "source.txt",
                  similarity: 0.8,
                },
              ],
            }
          : request.url.includes("/documents/")
            ? {
                id: "chunk1",
                doc_id: "document1",
                content_with_weight: content,
                available_int: 1,
              }
            : [{ id: "dataset1" }];
      response.end(JSON.stringify({ code: 0, data }));
      return;
    }
    if (request.url === "/v1/embeddings") {
      response.setHeader("Content-Type", "application/json");
      response.end(
        JSON.stringify({
          object: "list",
          model: "local-embedding",
          data: [{ object: "embedding", index: 0, embedding: [1, 0] }],
          usage: { prompt_tokens: 1, total_tokens: 1 },
        }),
      );
      return;
    }
    providerCalls++;
    response.setHeader("Content-Type", "application/json");
    if (!structuredEnabled) {
      response.writeHead(503);
      response.end('{"error":"smoke unavailable"}');
      return;
    }
    const body = JSON.parse(Buffer.concat(chunks).toString()) as {
      messages: unknown;
      response_format: {
        type: string;
        json_schema: { strict: boolean; schema: { required: string[] } };
      };
    };
    if (body.response_format.json_schema.schema.required.includes("text")) {
      if (!formationEnabled) {
        response.writeHead(503);
        response.end('{"error":"fixture-secret must not escape"}');
        return;
      }
      response.end(
        JSON.stringify({
          id: "formation",
          created: 1,
          model: "local-contract",
          choices: [
            {
              index: 0,
              message: {
                role: "assistant",
                content: JSON.stringify({
                  text: "Streaming artifact consumer wiring.",
                  semanticRole: "fact",
                  title: "Streaming wiring",
                  selectedEntityKeys: [],
                }),
              },
              finish_reason: "stop",
            },
          ],
          usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
        }),
      );
      return;
    }
    assert.equal(body.response_format.type, "json_schema");
    assert.equal(body.response_format.json_schema.strict, true);
    assert(
      body.response_format.json_schema.schema.required.includes(
        "uncertainties",
      ),
    );
    const key = JSON.stringify(body.messages).match(/D\d{3}/)?.[0] ?? "S000";
    const result = {
      summary: {
        content: "Local protocol continuity marker.",
        support_keys: [key],
      },
      coverage: {
        visual: "not_available",
        audio: "not_available",
        embedded_text: "not_available",
        source_text: "observed",
      },
      observations: [
        {
          kind: "text",
          content: "Local protocol continuity marker.",
          evidence_channel: "source_text",
          basis: "direct",
          certainty: "clear",
          start_ms: null,
          end_ms: null,
          support_keys: [key],
        },
      ],
      mentions: [],
      embedded_text: [],
      source_text: [],
      speech: [],
      interpretations: [],
      uncertainties: [],
    };
    response.end(
      JSON.stringify({
        id: "smoke",
        created: 1,
        model: "local-contract",
        choices: [
          {
            index: 0,
            message: { role: "assistant", content: JSON.stringify(result) },
            finish_reason: "stop",
          },
        ],
        usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
      }),
    );
  })().catch(() => response.destroy());
});
await new Promise<void>((done) => provider.listen(0, "127.0.0.1", done));
const providerAddress = provider.address();
if (!providerAddress || typeof providerAddress === "string")
  throw new Error("Smoke provider missing port");
await mkdir(join(dataRoot, "config"));
await writeFile(
  join(dataRoot, "config", "nous.toml"),
  `config_revision = ${CONFIG_REVISION}\n[host]\nkernel_executable = ${JSON.stringify(kernel)}\nport = 0\n[object_store]\nmax_upload_bytes = 1048576\n[[consumers]]\nconsumer_id = "default"\n[gateway_profiles.smoke]\nbase_url = "http://127.0.0.1:${providerAddress.port}/v1"\ncredential_env = "NOUS_SMOKE_GATEWAY"\n[model_profiles.local]\ngateway = "smoke"\nprotocol = "openai-chat"\nmodel = "local-contract"\ncapabilities = ["text", "structured_output"]\n[roles.material_structuring]\nmodel = "local"\n[resource_profiles.ragflow]\nmax_material_bytes = 1048576\nadapter_kind = "ragflow"\nbase_url = "http://127.0.0.1:${providerAddress.port}/api/v1"\ncredential_env = "NOUS_SMOKE_GATEWAY"\n`,
);
const runtimeRoot = process.env.NOUS_WAVE_POSTGRES_RUNTIME
  ? dirname(process.env.NOUS_WAVE_POSTGRES_RUNTIME)
  : join(root, "data/runtime/installed");
await writeFile(
  configPath,
  `[paths]\nprogram = ${JSON.stringify(root)}\nruntime = ${JSON.stringify(runtimeRoot)}\n`,
);
let core: ChildProcess | undefined;
async function boot() {
  core = spawn(
    process.execPath,
    [
      tsx,
      "apps/nous-core/src/main.ts",
      "--development",
      "--locator",
      configPath,
      "--stop-on-stdin-close",
    ],
    {
      cwd: root,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
      env: {
        ...process.env,
        NOUS_SMOKE_GATEWAY: "synthetic-local-contract",
      },
    },
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
      "--run-root",
      join(dataRoot, "run"),
      "--instance-root",
      join(dataRoot, "instance"),
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
  // Synthetic local wiring wiring; it is never real corpus or live-model evidence.
  const observation = await cli(
    "observe",
    "text",
    "--text",
    "Local consumer wiring marker: protocol continuity.",
  );
  const occurrenceId = String(observation.occurrenceId);
  const client = await connectNousInstance({ runRoot: join(dataRoot, "run") });
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
  structuredEnabled = true;
  const direct = await client.model.deriveMaterial({
    subjectId,
    sourceRegionId: String(observation.sourceRegionId),
    strategy: "direct_structured",
  });
  assert.equal(direct.degradation.length, 0);
  assert.equal(direct.representations.length, 1);
  const directPayload = direct.representations[0]!.structuredPayload!;
  assert.deepEqual(directPayload.observations, [
    {
      kind: "text",
      content: "Local protocol continuity marker.",
      evidence_channel: "source_text",
      basis: "direct",
      certainty: "clear",
      start_ms: null,
      end_ms: null,
      supports: [
        { kind: "source_region", value: String(observation.sourceRegionId) },
      ],
    },
  ]);
  const twoStage = await client.model.deriveMaterial({
    subjectId,
    sourceRegionId: String(observation.sourceRegionId),
    strategy: "describe_then_structure",
  });
  assert.equal(twoStage.degradation.length, 0);
  assert.equal(twoStage.representations.length, 2);
  const structured = twoStage.representations[1]!;
  assert.equal(twoStage.selectedRepresentationId, structured.representationId);
  const persisted = await client.material.representation({
    subjectId,
    id: structured.representationId,
  });
  assert.deepEqual(persisted.structuredPayload, structured.structuredPayload);
  assert.match(persisted.producer!.outputSchemaDigest!, /^[a-f0-9]{64}$/);
  const observations = persisted.structuredPayload!.observations as {
    supports: { kind: string; value: string }[];
  }[];
  assert.equal(observations[0]!.supports[0]!.kind, "derived_region");
  const segment = await client.material.derivedRegion({
    subjectId,
    id: observations[0]!.supports[0]!.value,
  });
  assert.equal(
    segment.representationId,
    twoStage.representations[0]!.representationId,
  );
  assert.equal(segment.coordinateKind, "description_segment");
  const segmentMaterial = await client.material.materialize({
    subjectId,
    reference: { kind: "derived_region", value: segment.derivedRegionId },
    maxBytes: 4096n,
  });
  assert.equal(segmentMaterial.partial, false);
  assert.equal(
    new TextDecoder().decode(segmentMaterial.content),
    "Local consumer wiring marker: protocol continuity.",
  );
  const beforeReplay = providerCalls;
  const replay = await client.model.deriveMaterial({
    subjectId,
    sourceRegionId: String(observation.sourceRegionId),
    strategy: "describe_then_structure",
  });
  assert.equal(replay.selectedRepresentationId, structured.representationId);
  assert.equal(providerCalls, beforeReplay);
  const resourceDescriptor = await client.resources.put({
    subjectId,
    descriptor: {
      resourceRef: "resource:smoke",
      displayLabel: "Qualification KB",
      authorityClass: "external_material",
      accessCostClass: "local_fixture",
      readiness: "ready",
      adapterKind: "ragflow",
      providerProfile: "ragflow",
      providerLocator: JSON.stringify({ dataset_ids: ["dataset1"] }),
    },
  });
  assert.equal(resourceDescriptor.providerProfile, "ragflow");
  const resourceResult = await client.cognition.recall(
    subjectId,
    '(("external chunk" $resource $current(required)) || ("unrelated" $memory)) $limit(2)',
  );
  assert.equal(resourceResult.resourceActions.length, 0);
  assert.equal(resourceResult.resourceRecords.length, 1);
  assert.equal(
    resourceResult.resourceRecords[0]!.content,
    "RAGFlow external chunk contract marker.",
  );
  assert.equal(
    resourceResult.resourceRecords[0]!.reference!.resourceRef,
    resourceDescriptor.resourceRef,
  );
  assert.equal(resourceResult.hits.length, 0);
  assert.equal(resourceProviderCalls, 2);
  const selectedRef = resourceResult.resourceRecords[0]!.reference!;
  const observedAt = {
    seconds: BigInt(Math.floor(Date.now() / 1000)),
    nanos: 0,
  };
  const selectedRequest = {
    subjectId,
    operationId: crypto.randomUUID(),
    reference: selectedRef,
    observedAt,
  };
  const selected = await client.resources.materialize(selectedRequest);
  assert(
    selected.observation?.artifactId && selected.observation.sourceRegionId,
  );
  const selectedOccurrence = await client.material.occurrence({
    subjectId,
    id: selected.observation.occurrenceId,
  });
  assert.equal(selectedOccurrence.sourceClass, "resource");
  assert.match(
    selectedOccurrence.externalObjectRef!,
    /^object:resource-entry:[a-f0-9]{64}$/,
  );
  const selectedSource = await client.material.materialize({
    subjectId,
    reference: {
      kind: "source_region",
      value: selected.observation.sourceRegionId,
    },
    maxBytes: 4096n,
  });
  assert.equal(
    new TextDecoder().decode(selectedSource.content),
    "RAGFlow external chunk contract marker.",
  );
  const afterSelected = resourceProviderCalls;
  const duplicateSelected = await client.resources.materialize(selectedRequest);
  assert.equal(
    duplicateSelected.observation?.occurrenceId,
    selected.observation.occurrenceId,
  );
  assert.equal(resourceProviderCalls, afterSelected);
  const secondExposure = await client.resources.materialize({
    ...selectedRequest,
    operationId: crypto.randomUUID(),
  });
  assert.notEqual(
    secondExposure.observation?.occurrenceId,
    selected.observation.occurrenceId,
  );
  assert.equal(
    secondExposure.observation?.artifactId,
    selected.observation.artifactId,
  );
  assert.equal((await client.material.limits({})).maxUploadBytes, 1048576n);
  for (const boundedContent of [
    "x".repeat(1048576),
    "\u0001".repeat(1048576),
  ]) {
    resourceContent = boundedContent;
    // The known provider locator is re-read and its current digest is verified by materialize.
    // Query results have their own 2 MiB aggregate bound; selection need not repeat search.
    const largeRequest = {
      ...selectedRequest,
      operationId: crypto.randomUUID(),
      reference: {
        ...selectedRef,
        contentDigest: createHash("sha256")
          .update(boundedContent)
          .digest("hex"),
      },
    };
    const largeSelected = await client.resources.materialize(largeRequest);
    assert.equal(largeSelected.content, boundedContent);
    const callsBeforeLargeReplay: number = resourceProviderCalls;
    const largeReplay = await client.resources.materialize(largeRequest);
    assert.equal(largeReplay.content, boundedContent);
    assert.equal(
      largeReplay.observation?.occurrenceId,
      largeSelected.observation?.occurrenceId,
    );
    assert.equal(resourceProviderCalls, callsBeforeLargeReplay);
  }
  resourceContent = "RAGFlow external chunk contract marker.";
  await client.resources.put({
    subjectId,
    descriptor: {
      ...resourceDescriptor,
      resourceRef: "resource:unavailable_fixture",
      providerProfile: "unconfigured",
    },
  });
  const unavailableResource = await client.cognition.recall(
    subjectId,
    '"external chunk" $resource $limit(2)',
  );
  assert.equal(unavailableResource.resourceRecords.length, 1);
  assert.equal(unavailableResource.resourceActions.length, 1);
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
        providerClass: "deterministic-local-smoke",
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
  await writeFile(file, "Streaming artifact consumer wiring.");
  const uploaded = await cli("observe", "file", file);
  assert(
    typeof uploaded.occurrenceId === "string" &&
      typeof uploaded.sourceRegionId === "string",
  );
  await stop();
  await appendFile(
    join(dataRoot, "config/nous.toml"),
    `
[model_profiles.embedding]
gateway = "smoke"
protocol = "openai-embeddings"
model = "local-embedding"
capabilities = ["embedding"]
[model_profiles.embedding.embedding]
dimension = 2
max_batch_size = 1
weights_revision = "fixture-1"
task = "retrieval"
input_representation = "text"
preprocessing_identity = "identity"
preprocessing_revision = "1"
normalization = "l2"
output_semantics = "dense"
[roles.query_embedding]
model = "embedding"
[roles.memory_formation]
model = "local"
`,
  );
  await boot();
  const restarted = await connectNousInstance({
    runRoot: join(dataRoot, "run"),
  });
  const prepared = await restarted.model.prepareEmbeddings({
    subjectId,
    limit: 1,
  });
  assert(prepared.committed > 0 && prepared.degradation.length === 0);
  const recall = await restarted.cognition.recall(
    subjectId,
    '"protocol continuity" $memory $limit(5)',
  );
  assert(recall.hits.some((hit) => hit.revision?.value === memory.revisionId));
  const formationRequest = {
    subjectId,
    operationId: crypto.randomUUID(),
    occurrenceId: uploaded.occurrenceId,
    aboutnessMode: "none",
  };
  const unavailableFormation =
    await restarted.model.formFromObservation(formationRequest);
  assert(!unavailableFormation.memory);
  assert.equal(
    unavailableFormation.degradation[0]?.code,
    "memory_formation_failed",
  );
  assert.match(
    unavailableFormation.degradation[0]?.detail ?? "",
    /gateway_http_503/,
  );
  assert(!JSON.stringify(unavailableFormation).includes("fixture-secret"));
  formationEnabled = true;
  const recoveredFormation =
    await restarted.model.formFromObservation(formationRequest);
  assert(
    recoveredFormation.memory && recoveredFormation.degradation.length === 0,
  );
  await cli("session", "show");
  console.log("Model/Material/Resource public smoke completed");
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
  await new Promise<void>((done) => provider.close(() => done()));
  await rm(dataRoot, { recursive: true, force: true });
}
