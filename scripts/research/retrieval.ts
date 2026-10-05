import { retrievalMetrics } from "./retrieval-metrics.js";
import { workspacePaths } from "../workspace.js";
import type { connectNousInstance } from "@nous-wave/client/node";
import { webSource } from "@nous-wave/client";
import { createHash, randomUUID } from "node:crypto";
import { readFile, writeFile, mkdir, rename } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { performance } from "node:perf_hooks";
import { setTimeout as delay } from "node:timers/promises";

type Source = {
  id: string;
  url: string;
  raw_sha256: string;
  topic: string;
  entity_ref: string;
  entities?: { surface: string; entityRef: string; semanticRole: string }[];
  publication_date?: string;
  rights?: string;
  extraction?: string;
};
type Unit = { id: string; source: string; paragraph: number; sha256: string };
type Query = {
  id: string;
  text: string;
  expected_units: string[];
  grounding_patterns: string[];
  oracle_status: string;
  anchor_sha256: string;
};
type Receipt = {
  createdAtMs: number;
  observeId: string;
  operationId: string;
  occurrenceId?: string;
  sourceRegionId?: string;
  representationId?: string;
  memoryId?: string;
  revisionId?: string;
  memoryText?: string;
  aboutness?: string[];
  failure?: string;
};
type Track = {
  createId: string;
  subjectId?: string;
  sessionId?: string;
  sessionClosed?: boolean;
  units: Record<string, Receipt>;
};
type State = {
  manifestDigest: string;
  controlled?: Track;
  "end-to-end"?: Track;
};

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    suite: { type: "string", default: "legacy" },
    "raw-file": { type: "string" },
    "prepared-root": { type: "string" },
    "run-root": { type: "string" },
    "client-module": { type: "string" },
    manifest: { type: "string", default: "docs/research/corpus/manifest.json" },
    queries: { type: "string", default: "docs/research/corpus/queries.json" },
    texts: {
      type: "string",
      default: resolve(workspacePaths.research, "corpus/unit-texts.json"),
    },
    state: {
      type: "string",
      default: resolve(workspacePaths.research, "runs/corpus-state.json"),
    },
    track: { type: "string", default: "controlled" },
    variant: { type: "string", default: "baseline" },
    output: {
      type: "string",
      default: resolve(workspacePaths.research, "runs/results.json"),
    },
    concurrency: { type: "string", default: "4" },
    limit: { type: "string", default: "0" },
    "embedding-batch": { type: "string", default: "64" },
    "skip-embeddings": { type: "boolean", default: false },
    session: { type: "boolean", default: false },
    "embedding-interval-ms": { type: "string", default: "0" },
  },
});
if (positionals[0] === "prepare") {
  if (!values["raw-file"] || !values["prepared-root"])
    throw new Error("prepare requires --raw-file --prepared-root");
  if (values.suite === "hard-text") {
    const { prepareHardText } = await import("./retrieval/hard-text.js");
    await prepareHardText(
      values["raw-file"],
      values.queries,
      values.texts,
      values["prepared-root"],
    );
    process.exit(0);
  }
  const { prepareExternal } = await import("./retrieval/external.js");
  await prepareExternal(
    values.suite,
    values["raw-file"],
    values["prepared-root"],
  );
  process.exit(0);
}
if (values.suite !== "legacy")
  throw new Error(
    "External suite import/run uses the clock-injected Kernel harness; public-host track integration pending",
  );
if (!values["run-root"] || !values["client-module"])
  throw new Error(
    "--run-root and distributed --client-module required; credentials are read by the runtime",
  );
if (!["controlled", "end-to-end"].includes(values.track))
  throw new Error("Unknown track");
if (!["baseline", "model-rerank", "wave", "combined"].includes(values.variant))
  throw new Error("Unknown variant");
const trackName = values.track as "controlled" | "end-to-end";
const manifestBytes = await readFile(values.manifest),
  digest = createHash("sha256").update(manifestBytes).digest("hex");
const manifest = JSON.parse(manifestBytes.toString("utf8")) as {
  sources: Source[];
  units: Unit[];
};
const queries = (
  JSON.parse(await readFile(values.queries, "utf8")) as { queries: Query[] }
).queries;
const texts = JSON.parse(await readFile(values.texts, "utf8")) as Record<
  string,
  string
>;
for (const unit of manifest.units)
  if (
    createHash("sha256")
      .update(texts[unit.id] ?? "")
      .digest("hex") !== unit.sha256
  )
    throw new Error(`Source unit digest mismatch: ${unit.id}`);
for (const query of queries) {
  if (query.oracle_status !== "PASS" || !query.expected_units.length)
    throw new Error(`Unverified oracle: ${query.id}`);
  for (const unit of query.expected_units)
    if (
      !query.grounding_patterns.every((pattern) =>
        new RegExp(pattern, "is").test(texts[unit] ?? ""),
      )
    )
      throw new Error(`Oracle anchor mismatch: ${query.id}/${unit}`);
}
let state: State;
try {
  state = JSON.parse(await readFile(values.state, "utf8")) as State;
} catch (error) {
  if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  state = { manifestDigest: digest };
}
if (state.manifestDigest !== digest)
  throw new Error("Research state belongs to a different corpus manifest");
let saving: Promise<void> = Promise.resolve();
const json = (value: unknown) =>
  JSON.stringify(
    value,
    (_, item: unknown) => (typeof item === "bigint" ? item.toString() : item),
    2,
  );
function save() {
  saving = saving.then(async () => {
    const path = resolve(values.state),
      temporary = `${path}.${randomUUID()}.tmp`;
    await mkdir(dirname(path), { recursive: true });
    await writeFile(temporary, json(state), { mode: 0o600 });
    await rename(temporary, path);
  });
  return saving;
}
const distributed = (await import(
  pathToFileURL(resolve(values["client-module"])).href
)) as {
  connectNousInstance: typeof connectNousInstance;
};
const client = await distributed.connectNousInstance({
  runRoot: resolve(values["run-root"]),
});
const cancellation = new AbortController();
process.once("SIGINT", () =>
  cancellation.abort(new Error("Research interrupted")),
);
const options = { timeoutMs: 300000, signal: cancellation.signal };
const now = (milliseconds = Date.now()) => ({
  seconds: BigInt(Math.floor(milliseconds / 1000)),
  nanos: 0,
});
const sources = new Map(manifest.sources.map((source) => [source.id, source]));

async function importCorpus() {
  const track = (state[trackName] ??= { createId: randomUUID(), units: {} });
  await save();
  if (!track.subjectId) {
    const subject = await client.subjects.create(
      {
        operationId: track.createId,
        capabilities: { memory: true },
        cognitiveSeed: {
          text: "schema_version = 1",
          format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
        },
        metadata: { research_track: trackName, corpus_digest: digest },
      },
      options,
    );
    track.subjectId = subject.subjectId;
    await save();
  }
  const subjectId = track.subjectId;
  const requested = Number(values.limit),
    selected =
      requested > 0 ? manifest.units.slice(0, requested) : manifest.units;
  const concurrency = Number(values.concurrency);
  if (!Number.isInteger(requested) || requested < 0)
    throw new Error("--limit must be a nonnegative integer");
  const embeddingBatch = Number(values["embedding-batch"]);
  if (
    !Number.isInteger(embeddingBatch) ||
    embeddingBatch < 1 ||
    embeddingBatch > 64
  )
    throw new Error("--embedding-batch must be 1..64");
  const embeddingInterval = Number(values["embedding-interval-ms"]);
  if (
    !Number.isInteger(embeddingInterval) ||
    embeddingInterval < 0 ||
    embeddingInterval > 60000
  )
    throw new Error("--embedding-interval-ms must be 0..60000");
  if (!Number.isInteger(concurrency) || concurrency < 1 || concurrency > 4)
    throw new Error("--concurrency must be 1..4");
  if (values.session && !track.sessionId) {
    track.sessionId = (
      await client.cognition.openSession({ subjectId }, options)
    ).sessionId;
    await save();
  }
  let cursor = 0,
    completed = 0;
  await Promise.all(
    Array.from({ length: concurrency }, async () => {
      while (cursor < selected.length && !cancellation.signal.aborted) {
        const unit = selected[cursor++]!,
          source = sources.get(unit.source)!;
        const receipt = (track.units[unit.id] ??= {
          createdAtMs: Date.now(),
          observeId: randomUUID(),
          operationId: randomUUID(),
        });
        if (receipt.revisionId) continue;
        await save();
        const text = `Document topic: ${source.topic}\n\n${texts[unit.id]}`;
        try {
          if (!receipt.occurrenceId) {
            const observation = await client.cognition.observe(
              {
                subjectId,
                requestId: receipt.observeId,
                sessionId: track.sessionId,
                ...webSource(source.url),
                context: {
                  source_url: source.url,
                  raw_source_sha256: source.raw_sha256,
                  unit_sha256: unit.sha256,
                  paragraph: unit.paragraph,
                  extraction: source.extraction ?? "html-p-text-v1",
                  ...(source.publication_date
                    ? { publication_date: source.publication_date }
                    : {}),
                  ...(source.rights ? { rights: source.rights } : {}),
                },
                entities: source.entities ?? [
                  {
                    surface: source.topic,
                    entityRef: source.entity_ref,
                    semanticRole: "document_topic",
                  },
                ],
                observedAt: now(receipt.createdAtMs),
                admit: true,
                material: {
                  case: "inlineText",
                  value: { text, mediaType: "text/plain" },
                },
              },
              options,
            );
            receipt.occurrenceId = observation.occurrenceId;
            receipt.sourceRegionId = observation.sourceRegionId;
            await save();
          }
          if (!receipt.representationId) {
            const derived = await client.model.deriveMaterial(
              {
                subjectId,
                sourceRegionId: receipt.sourceRegionId!,
                strategy: "description_only",
              },
              options,
            );
            if (!derived.selectedRepresentationId || derived.degradation.length)
              throw new Error("Real text derivation incomplete");
            receipt.representationId = derived.selectedRepresentationId;
            await save();
          }
          const formed =
            trackName === "controlled"
              ? {
                  memory: await client.memory.form(
                    {
                      subjectId,
                      operationId: receipt.operationId,
                      input: {
                        cognitiveRole: "declarative",
                        formationMode: "grounded",
                        groundingOccurrenceId: receipt.occurrenceId,
                        semanticRole: "source_statement",
                        text,
                        aboutness: [source.entity_ref],
                        epistemicClass: "reported",

                        supports: [
                          {
                            support: {
                              case: "evidence",
                              value: {
                                occurrenceId: receipt.occurrenceId!,
                                locator: {
                                  case: "derivedRepresentationId",
                                  value: receipt.representationId,
                                },
                                supportRole: "direct",
                              },
                            },
                          },
                        ],
                      },
                    },
                    options,
                  ),
                }
              : await client.model.formFromObservation(
                  {
                    subjectId,
                    occurrenceId: receipt.occurrenceId!,
                    representationId: receipt.representationId,
                    operationId: receipt.operationId,
                    aboutnessMode: "select_from_resolved_mentions",
                  },
                  options,
                );
          if (!formed.memory)
            throw new Error(
              `Formation did not commit Memory: ${json("degradation" in formed ? formed.degradation : [])}`,
            );
          receipt.memoryId = formed.memory.memoryId;
          receipt.revisionId = formed.memory.revisionId;
          receipt.memoryText = formed.memory.text;
          receipt.aboutness = formed.memory.aboutness;
          delete receipt.failure;
        } catch (error) {
          receipt.failure =
            error instanceof Error ? error.message : "Operation failed";
        }
        await save();
        completed++;
        process.stdout.write(
          json({
            unit: unit.id,
            status: receipt.revisionId ? "PASS" : "FAIL",
            completed,
            selected: selected.length,
          }) + "\n",
        );
      }
    }),
  );
  if (
    track.sessionId &&
    !track.sessionClosed &&
    selected.every((unit) => track.units[unit.id]?.revisionId)
  ) {
    await client.cognition.closeSession(
      { subjectId, id: track.sessionId },
      options,
    );
    track.sessionClosed = true;
    await save();
  }
  for (const source of manifest.sources) {
    if (
      !manifest.units.some(
        (unit) => unit.source === source.id && track.units[unit.id]?.revisionId,
      )
    )
      continue;
    for (const entity of source.entities ?? [
      { surface: source.topic, entityRef: source.entity_ref },
    ])
      await client.identity.bind(
        {
          subjectId,
          canonical: { kind: "entity", value: entity.entityRef },
          displayName: entity.surface,
        },
        options,
      );
  }
  let committed = 0;
  while (!values["skip-embeddings"]) {
    const batch = await client.model.prepareEmbeddings(
      { subjectId, limit: Number(values["embedding-batch"]) },
      options,
    );
    committed += batch.committed;
    await save();
    if (batch.degradation.length)
      throw new Error(
        `Real embedding preparation incomplete: ${JSON.stringify(batch.degradation)}`,
      );
    if (!batch.committed) break;
    if (embeddingInterval)
      await delay(embeddingInterval, undefined, {
        signal: cancellation.signal,
      });
  }
  process.stdout.write(
    json({
      status: Object.values(track.units).some((unit) => !unit.revisionId)
        ? "FAIL"
        : "PASS",
      track: trackName,
      subjectId,
      units: Object.values(track.units).filter((unit) => unit.revisionId)
        .length,
      embeddingsCommitted: committed,
    }) + "\n",
  );
  if (Object.values(track.units).some((unit) => !unit.revisionId))
    process.exitCode = 1;
}

async function runQueries() {
  const track = state[trackName];
  if (!track?.subjectId) throw new Error("Import the selected track first");
  const subjectId = track.subjectId,
    before = await client.subjects.get({ subjectId });
  const byRevision = new Map(
    Object.entries(track.units)
      .filter(([, receipt]) => receipt.revisionId)
      .map(([unit, receipt]) => [receipt.revisionId!, unit]),
  );
  const rows: Record<string, unknown>[] = [];
  const output = resolve(values.output);
  await mkdir(dirname(output), { recursive: true });
  const wantsRerank = ["model-rerank", "combined"].includes(values.variant),
    wantsWave = ["wave", "combined"].includes(values.variant);
  for (const query of queries) {
    const available = query.expected_units.filter((unit) =>
      query.grounding_patterns.every((pattern) =>
        new RegExp(pattern, "is").test(track.units[unit]?.memoryText ?? ""),
      ),
    );
    const start = performance.now();
    const response = await client.cognition.query(
      {
        subjectId,
        nousql: `${JSON.stringify(query.text)} $memory $limit(10) $diagnostics(full)${wantsWave ? " $explore" : ""}`,
      },
      options,
    );
    const latency = performance.now() - start;
    if (
      wantsRerank !==
      response.hits.some((hit) => hit.score?.rerank !== undefined)
    )
      throw new Error(
        `Variant mechanism mismatch: ${values.variant}/${query.id}`,
      );
    const units = response.hits.map((hit) =>
      byRevision.get(hit.reference?.value ?? ""),
    );
    const rank =
      units.findIndex(
        (unit) => unit !== undefined && available.includes(unit),
      ) + 1;
    let provenanceCorrect = 0,
      wrongSource = 0,
      wrongEntity = 0,
      stale = 0;
    for (const hit of response.hits) {
      const unitId = byRevision.get(hit.reference?.value ?? "");
      if (!unitId) {
        stale++;
        continue;
      }
      const unit = manifest.units.find((candidate) => candidate.id === unitId)!,
        source = sources.get(unit.source)!;
      const memory = await client.memory.revision({
        subjectId,
        id: hit.reference!.value,
      });
      const current = await client.memory.get({
        subjectId,
        id: memory.memoryId,
      });
      if (current.revisionId !== memory.revisionId) stale++;
      const evidence = memory.supports.find(
        (support) => support.support.case === "evidence",
      )?.support;
      if (evidence?.case === "evidence") {
        const occurrence = await client.material.occurrence({
          subjectId,
          id: evidence.value.occurrenceId,
        });
        if (occurrence.context?.source_url === source.url) provenanceCorrect++;
        else wrongSource++;
      } else wrongSource++;
      if (hit.entityRefs.length && !hit.entityRefs.includes(source.entity_ref))
        wrongEntity++;
    }
    const irMetrics = retrievalMetrics(
      units.map(
        (id, index) =>
          id ?? `unmapped:${response.hits[index]?.reference?.value ?? index}`,
      ),
      Object.fromEntries(
        query.expected_units.map((id) => [
          id,
          {
            grade: 1 as const,
            reason: "direct_fact",
            source: manifest.units.find((unit) => unit.id === id)!.source,
          },
        ]),
      ),
    );
    rows.push({
      query: query.id,
      status: "PASS",
      expectedUnits: query.expected_units,
      availableUnits: available,
      returnedUnits: units,
      rank,
      recall1: irMetrics.atK[1]!.recall,
      recall5: irMetrics.atK[5]!.recall,
      recall10: irMetrics.atK[10]!.recall,
      hitRate1: irMetrics.atK[1]!.hitRate,
      hitRate5: irMetrics.atK[5]!.hitRate,
      hitRate10: irMetrics.atK[10]!.hitRate,
      ndcg5: irMetrics.atK[5]!.ndcg,
      ndcg10: irMetrics.atK[10]!.ndcg,
      averagePrecision: irMetrics.averagePrecision,
      sourceSetRecall10: irMetrics.atK[10]!.sourceSetRecall,
      irMetrics,
      reciprocalRank: rank > 0 ? 1 / rank : 0,
      formationCoverage: available.length ? 1 : 0,
      latencyMs: latency,
      provenanceCorrect,
      wrongSource,
      wrongEntity,
      stale,
      hitCount: response.hits.length,
      response,
    });
    await writeFile(
      output,
      json({
        status: "NOT_RUN",
        manifestDigest: digest,
        track: trackName,
        variant: values.variant,
        subjectId,
        authoritySeq: before.authoritySeq,
        queryCount: rows.length,
        rows,
        reason: "Measurement in progress; final Authority check pending",
        cost: "unknown",
      }),
    );
    process.stdout.write(
      json({ query: query.id, status: "PASS", rank, completed: rows.length }) +
        "\n",
    );
  }
  const after = await client.subjects.get({ subjectId });
  if (after.authoritySeq !== before.authoritySeq)
    throw new Error("Authority changed during a measured variant");
  const mean = (field: string, selected = rows) =>
    selected.length
      ? selected.reduce((sum, row) => sum + Number(row[field]), 0) /
        selected.length
      : null;
  const conditional = rows.filter((row) => Number(row.formationCoverage) === 1);
  const latency = rows
    .map((row) => Number(row.latencyMs))
    .sort((a, b) => a - b);
  const total = (field: string) =>
    rows.reduce((sum, row) => sum + Number(row[field]), 0);
  await writeFile(
    output,
    json({
      status: "PASS",
      manifestDigest: digest,
      track: trackName,
      variant: values.variant,
      subjectId,
      authoritySeq: before.authoritySeq,
      queryCount: rows.length,
      rows,
      metrics: {
        formationCoverage: mean("formationCoverage"),
        recall1: mean("recall1"),
        recall5: mean("recall5"),
        recall10: mean("recall10"),
        mrr: mean("reciprocalRank"),
        hitRate1: mean("hitRate1"),
        hitRate5: mean("hitRate5"),
        hitRate10: mean("hitRate10"),
        ndcg5: mean("ndcg5"),
        ndcg10: mean("ndcg10"),
        averagePrecision: mean("averagePrecision"),
        sourceSetRecall10: mean("sourceSetRecall10"),
        conditionalRecall1: mean("recall1", conditional),
        conditionalRecall5: mean("recall5", conditional),
        conditionalRecall10: mean("recall10", conditional),
        provenancePrecision: total("hitCount")
          ? total("provenanceCorrect") / total("hitCount")
          : null,
        wrongSource: total("wrongSource"),
        wrongEntity: total("wrongEntity"),
        staleLeakage: total("stale"),
        emptyRate:
          rows.filter((row) => Number(row.hitCount) === 0).length / rows.length,
        latencyP50Ms: latency[Math.ceil(latency.length * 0.5) - 1],
        latencyP95Ms: latency[Math.ceil(latency.length * 0.95) - 1],
      },
      cost: "unknown",
      measurement:
        "actual official Client/public NousQL response; semantic fact-marker availability is conservative and requires review for paraphrases",
    }),
  );
  process.stdout.write(
    json({ status: "PASS", output, queryCount: rows.length }) + "\n",
  );
}

async function auditFormation() {
  const track = state["end-to-end"];
  if (trackName !== "end-to-end" || !track?.subjectId)
    throw new Error("audit-formation requires an imported end-to-end track");
  const subjectId = track.subjectId;
  const before = await client.subjects.get({ subjectId }, options);
  let audited = 0;
  for (const receipt of Object.values(track.units)) {
    if (!receipt.revisionId) continue;
    const replay = await client.model.formFromObservation(
      {
        subjectId,
        occurrenceId: receipt.occurrenceId!,
        representationId: receipt.representationId,
        operationId: receipt.operationId,
        aboutnessMode: "select_from_resolved_mentions",
      },
      options,
    );
    if (replay.memory?.revisionId !== receipt.revisionId)
      throw new Error("Formation replay changed its committed identity");
    audited++;
  }
  const after = await client.subjects.get({ subjectId }, options);
  if (after.authoritySeq !== before.authoritySeq)
    throw new Error("Formation audit changed Authority");
  await save();
  process.stdout.write(
    json({ status: "PASS", audited, authoritySeq: before.authoritySeq }) + "\n",
  );
}

if (positionals[0] === "import") await importCorpus();
else if (positionals[0] === "run") await runQueries();
else if (positionals[0] === "audit-formation") await auditFormation();
else throw new Error("Use import, run or audit-formation");
