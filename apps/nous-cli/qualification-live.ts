import type { connectNousInstance } from "@nous-wave/client/node";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { randomUUID, createHash } from "node:crypto";
import { parseArgs } from "node:util";
import { performance } from "node:perf_hooks";
import { setTimeout as delay } from "node:timers/promises";

type Client = Awaited<ReturnType<typeof connectNousInstance>>;
type Unit = {
  id: string;
  file: string;
  sha256: string;
  media_type: string;
  source_url: string;
  rights: string;
  context?: Record<string, string>;
  query: string;
  oracle_patterns: string[];
  oracle_status: "PASS" | "NOT_RUN";
};
type Receipt = {
  requestId: string;
  createdAtMs: number;
  artifactId?: string;
  occurrenceId?: string;
  sourceRegionId?: string;
  strategies: Record<
    string,
    { operationId: string; revisionId?: string; result?: unknown }
  >;
};
const { values } = parseArgs({
  options: {
    "run-root": { type: "string" },
    "client-module": { type: "string" },
    manifest: { type: "string", default: "research/corpus/media.json" },
    "raw-root": { type: "string", default: "data/research/corpus/raw" },
    state: { type: "string", default: "data/research/live/media-state.json" },
  },
});
if (!values["run-root"] || !values["client-module"])
  throw new Error("--run-root and distributed --client-module required");
const module = (await import(
  pathToFileURL(resolve(values["client-module"])).href
)) as { connectNousInstance: typeof connectNousInstance };
const client: Client = await module.connectNousInstance({
  runRoot: resolve(values["run-root"]),
});
const cancellation = new AbortController();
process.once("SIGINT", () =>
  cancellation.abort(new Error("Qualification interrupted")),
);
const options = { timeoutMs: 180000, signal: cancellation.signal };
const manifestBytes = await readFile(values.manifest),
  digest = createHash("sha256").update(manifestBytes).digest("hex");
const units = (JSON.parse(manifestBytes.toString("utf8")) as { units: Unit[] })
  .units;
const statePath = resolve(values.state);
const state = JSON.parse(
  await readFile(statePath, "utf8").catch((error: NodeJS.ErrnoException) => {
    if (error.code === "ENOENT")
      return JSON.stringify({
        manifestDigest: digest,
        createId: randomUUID(),
        units: {},
      });
    throw error;
  }),
) as {
  manifestDigest: string;
  createId: string;
  subjectId?: string;
  units: Record<string, Receipt>;
  embeddingInvocations?: unknown[];
};
if (state.manifestDigest !== digest)
  throw new Error("Media manifest changed; select a new state");
const json = (value: unknown) =>
  JSON.stringify(
    value,
    (_, item) => (typeof item === "bigint" ? item.toString() : item),
    2,
  );
const save = async () => {
  await mkdir(dirname(statePath), { recursive: true });
  await writeFile(statePath, json(state));
};
if (!state.subjectId) {
  const subject = await client.subjects.create(
    {
      operationId: state.createId,
      capabilities: { memory: true },
      cognitiveSeed: {
        text: "schema_version = 1",
        format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
      },
    },
    options,
  );
  state.subjectId = subject.subjectId;
  await save();
}
const subjectId = state.subjectId;
const rawRoot = resolve(values["raw-root"]);
for (const unit of units) {
  const file = resolve(rawRoot, unit.file);
  if (!file.startsWith(rawRoot + sep))
    throw new Error("Media cache locator escapes raw root");
  if (
    createHash("sha256")
      .update(await readFile(file))
      .digest("hex") !== unit.sha256
  )
    throw new Error(`Media digest changed: ${unit.id}`);
  const receipt = (state.units[unit.id] ??= {
    requestId: randomUUID(),
    createdAtMs: Date.now(),
    strategies: {},
  });
  await save();
  if (!receipt.artifactId) {
    const artifact = await client.artifacts.uploadFile(subjectId, file, {
      ...options,
      mediaType: unit.media_type,
    });
    receipt.artifactId = artifact.artifactId;
    await save();
  }
  if (!receipt.occurrenceId) {
    const observed = await client.cognition.observe(
      {
        subjectId,
        requestId: receipt.requestId,
        sourceClass: "web",
        context: {
          ...unit.context,
          source_url: unit.source_url,
          source_sha256: unit.sha256,
          rights: unit.rights,
        },
        observedAt: { seconds: BigInt(Math.floor(receipt.createdAtMs / 1000)) },
        admit: true,
        material: { case: "artifactId", value: receipt.artifactId },
      },
      options,
    );
    receipt.occurrenceId = observed.occurrenceId;
    receipt.sourceRegionId = observed.sourceRegionId;
    await save();
  }
  for (const strategy of [
    "description_only",
    "direct_structured",
    "describe_then_structure",
  ]) {
    const operation = (receipt.strategies[strategy] ??= {
      operationId: randomUUID(),
    });
    if (
      operation.revisionId &&
      (operation.result as { status?: string } | undefined)?.status === "PASS"
    )
      continue;
    await save();
    const start = performance.now();
    try {
      const derived = await client.model.deriveMaterial(
        { subjectId, sourceRegionId: receipt.sourceRegionId!, strategy },
        options,
      );
      const selected = derived.representations.find(
        (item) => item.representationId === derived.selectedRepresentationId,
      );
      if (!selected || derived.degradation.length)
        throw new Error(`Derivation incomplete: ${json(derived.degradation)}`);
      const formed = await client.model.formFromObservation(
        {
          subjectId,
          operationId: operation.operationId,
          occurrenceId: receipt.occurrenceId,
          representationId: selected.representationId,
          aboutnessMode: "none",
        },
        options,
      );
      if (!formed.memory) throw new Error("Formation did not commit Memory");
      operation.revisionId = formed.memory.revisionId;
      const missingFacts = unit.oracle_patterns.filter(
        (pattern) => !new RegExp(pattern, "is").test(selected.text ?? ""),
      );
      const roots = new Set<string>(),
        visited = new Set<string>();
      const trace = async (reference: {
        kind: string;
        value: string;
      }): Promise<void> => {
        const key = `${reference.kind}:${reference.value}`;
        if (visited.has(key)) return;
        if (visited.size >= 512)
          throw new Error("Provenance trace bound exhausted");
        visited.add(key);
        if (reference.kind === "derived_representation") {
          const representation = await client.material.representation(
            { subjectId, id: reference.value },
            options,
          );
          for (const input of representation.inputs)
            if (input.reference) await trace(input.reference);
        } else if (reference.kind === "source_region") {
          const region = await client.material.sourceRegion(
            { subjectId, id: reference.value },
            options,
          );
          roots.add(region.artifactId);
        } else if (reference.kind === "derived_region") {
          const region = await client.material.derivedRegion(
            { subjectId, id: reference.value },
            options,
          );
          await trace({
            kind: "derived_representation",
            value: region.representationId,
          });
        }
      };
      await trace({
        kind: "derived_representation",
        value: selected.representationId,
      });
      if (roots.size !== 1 || !roots.has(receipt.artifactId!))
        throw new Error("Provenance does not reach the uploaded source");
      operation.result = {
        status: "PASS",
        quality:
          unit.oracle_status === "NOT_RUN"
            ? "NOT_RUN"
            : missingFacts.length
              ? "FAIL"
              : "PASS",
        missingFacts,
        strategy,
        latencyMs: performance.now() - start,
        derived,
        formation: formed,
        sourceRoots: [...roots],
        traceNodes: visited.size,
      };
    } catch (error) {
      operation.result = {
        status: "FAIL",
        strategy,
        error: error instanceof Error ? error.message : "Operation failed",
      };
    }
    await save();
    process.stdout.write(
      json({
        unit: unit.id,
        strategy,
        result: (operation.result as { status: string }).status,
      }) + "\n",
    );
  }
}
while (true) {
  const batch = await client.model.prepareEmbeddings(
    { subjectId, limit: 1 },
    options,
  );
  (state.embeddingInvocations ??= []).push(...batch.invocations);
  await save();
  if (batch.degradation.length)
    throw new Error(`Media embeddings incomplete: ${json(batch.degradation)}`);
  if (!batch.committed) break;
  await delay(2000, undefined, { signal: cancellation.signal });
}
for (const unit of units) {
  const response = await client.cognition.query(
    {
      subjectId,
      nousql: `${JSON.stringify(unit.query)} $memory $limit(10) $diagnostics(full)`,
    },
    options,
  );
  const expected = Object.values(state.units[unit.id]!.strategies)
    .map((operation) => operation.revisionId)
    .filter(Boolean);
  const recalled = response.hits.filter((hit) =>
    expected.includes(hit.reference?.value),
  );
  (state.units[unit.id] as Receipt & { query?: unknown }).query = {
    status: recalled.length ? "PASS" : "FAIL",
    response,
  };
  await save();
}
const failed = Object.values(state.units)
  .flatMap((unit) => Object.values(unit.strategies))
  .filter(
    (operation) =>
      (operation.result as { status?: string } | undefined)?.status !== "PASS",
  ).length;
process.stdout.write(
  json({
    status: failed ? "FAIL" : "PASS",
    subjectId,
    units: units.length,
    failed,
  }) + "\n",
);
if (failed) process.exitCode = 1;
