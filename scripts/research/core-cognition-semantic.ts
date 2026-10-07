// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { readFile, writeFile, mkdir, open, unlink } from "node:fs/promises";
import { resolve, join } from "node:path";
import { parseArgs } from "node:util";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import {
  ConfigExposure,
  ConfigurationView,
  type NousClient,
  webSource,
} from "@nous-wave/client";
import { connectNousInstance } from "@nous-wave/client/node";
import {
  assertResume,
  digest,
  ignoredPath,
  json,
  loadManifest,
  normalizeHit,
  parseJson,
  saveJson,
  sourceMetrics,
  stableId,
  verifyDigest,
  type Identities,
  type Manifest,
  type SemanticQuery,
  type Source,
} from "./core-cognition-contracts.js";

type Observation = Awaited<ReturnType<NousClient["cognition"]["observe"]>>;
type Memory = Awaited<ReturnType<NousClient["memory"]["form"]>>;
type Tag = Awaited<ReturnType<NousClient["concepts"]["getTag"]>>;
type Grant = Awaited<ReturnType<NousClient["cognition"]["grantMaintenance"]>>;
type Query = Parameters<NousClient["cognition"]["query"]>[0];
type Response = Awaited<ReturnType<NousClient["cognition"]["query"]>>;
export interface Operation {
  digest: string;
  status: "pending" | "complete";
  replayable: boolean;
  result?: unknown;
  error?: string;
}
export interface RunState {
  version: 1;
  run_id: string;
  identities: Identities;
  sealed: boolean;
  started_at: string;
  subjects: { formed: string; raw_control: string };
  operations: Record<string, Operation>;
  checkpoints: Record<
    string,
    { cut: string; authority_seq: string; snapshot_digest: string }
  >;
  phases: Record<string, string>;
  executions?: {
    phase: string;
    started_at: string;
    identities: Identities;
    max_calls: number;
    max_elapsed_ms: number;
    status: string;
    error?: string;
  }[];
  freeze?: {
    formed: string;
    raw_control: string;
    snapshot_digest: string;
    review_digest: string;
  };
}
/** Persist pending intent before the remote effect; replay only operations with public stable identity. */
export async function operation<T>(
  state: RunState,
  key: string,
  request: unknown,
  replayable: boolean,
  save: () => Promise<void>,
  call: () => Promise<T>,
): Promise<T> {
  const hash = digest(json(request)),
    old = state.operations[key];
  if (old && old.digest !== hash)
    throw new Error(`Operation request changed: ${key}`);
  if (old?.status === "complete") return old.result as T;
  if (old && !old.replayable)
    throw new Error(
      `BLOCKED: uncertain non-idempotent operation ${key}; inspect public state and ledger before continuing in a new run`,
    );
  state.operations[key] = { digest: hash, status: "pending", replayable };
  await save();
  try {
    const result = await call();
    state.operations[key] = {
      digest: hash,
      status: "complete",
      replayable,
      result,
    };
    await save();
    return result;
  } catch (error) {
    state.operations[key]!.error =
      error instanceof Error ? error.message : String(error);
    await save();
    throw error;
  }
}
export function sourceUnits(text: string, limit = 12000) {
  const units: { text: string; start: number; end: number }[] = [];
  let current = "",
    start = 0,
    bytes = 0;
  // Unicode boundaries preserve exact bytes; prefer line boundaries without query/oracle-dependent selection.
  for (const line of text.match(/[^\n]*\n|[^\n]+$/g) ?? []) {
    if (Buffer.byteLength(current + line) <= limit) {
      current += line;
      continue;
    }
    if (current) {
      units.push({
        text: current,
        start,
        end: start + Buffer.byteLength(current),
      });
      start += Buffer.byteLength(current);
      current = "";
    }
    for (const char of line) {
      if (Buffer.byteLength(current) + Buffer.byteLength(char) > limit) {
        bytes = Buffer.byteLength(current);
        units.push({ text: current, start, end: start + bytes });
        start += bytes;
        current = "";
      }
      current += char;
    }
  }
  if (current)
    units.push({
      text: current,
      start,
      end: start + Buffer.byteLength(current),
    });
  return units;
}
interface Snapshot {
  memories: Memory[];
  tags: Tag[];
  episodes: Awaited<ReturnType<NousClient["memory"]["listEpisodes"]>>["items"];
  journals: Awaited<ReturnType<NousClient["memory"]["listJournals"]>>["items"];
  associations: Awaited<
    ReturnType<NousClient["concepts"]["neighborhood"]>
  >["associations"];
  schemas: Awaited<ReturnType<NousClient["concepts"]["getSchema"]>>[];
  partial: string[];
}
const at = (s: string) => timestampFromDate(new Date(s));
const profiles = [
  "baseline-rrf",
  "nous-node-potential-v1",
  "vcp-dtsc-v9.2.1-adapter-v1",
  "vcp-rivermemo-v3.1-adapter-v1",
];
export class SemanticRun {
  readonly options: { signal: AbortSignal; timeoutMs: number };
  constructor(
    readonly client: NousClient,
    readonly manifest: Manifest,
    readonly state: RunState,
    readonly output: string,
    readonly ledger: string,
    readonly maxCalls: number,
    readonly maxElapsed: number,
    readonly traceRoot: string,
  ) {
    this.options = {
      signal: AbortSignal.timeout(maxElapsed),
      timeoutMs: Math.min(maxElapsed, 300000),
    };
  }
  save = () => saveJson(join(this.output, "state.json"), this.state);
  id(key: string) {
    return stableId(this.state.run_id, key);
  }
  async call<T>(
    key: string,
    request: unknown,
    replayable: boolean,
    call: () => Promise<T>,
  ) {
    this.options.signal.throwIfAborted();
    return operation(this.state, key, request, replayable, this.save, call);
  }
  async budget() {
    this.options.signal.throwIfAborted();
    let ledger: { count: number };
    try {
      ledger = JSON.parse(await readFile(this.ledger, "utf8")) as {
        count: number;
      };
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
      ledger = { count: 0 };
    }
    if (!Number.isSafeInteger(ledger.count) || ledger.count >= this.maxCalls)
      throw new Error("Live gateway call budget exhausted");
    return this.maxCalls - ledger.count;
  }
  async checkSources() {
    for (const s of this.manifest.sources) {
      verifyDigest(
        await readFile(ignoredPath(s.raw_path)),
        s.raw_sha256,
        `${s.id}/raw`,
      );
      verifyDigest(
        await readFile(ignoredPath(s.text_path)),
        s.source_text_sha256,
        `${s.id}/text`,
      );
    }
    this.state.phases["acquire-check"] = "PASS";
    await this.save();
  }
  async createSubjects() {
    for (const track of ["formed", "raw_control"] as const) {
      const subjectId = this.state.subjects[track];
      const request = {
        subjectId,
        operationId: this.id(`subject/${track}`),
        capabilities: { memory: true },
        cognitiveSeed: {
          text: "schema_version = 1",
          format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
          provenance: {},
        },
      };
      await this.call(`subject/${track}`, request, true, () =>
        this.client.subjects.create(request, this.options),
      );
    }
    for (const entity of new Map(
      this.manifest.sources
        .flatMap((s) => s.entities ?? [])
        .map((e) => [e.entityRef, e]),
    ).values()) {
      const request = {
        subjectId: this.state.subjects.formed,
        canonical: { kind: "entity", value: entity.entityRef },
        displayName: entity.surface,
      };
      await this.call(`entity/${entity.entityRef}`, request, true, () =>
        this.client.identity.bind(request, this.options),
      );
    }
  }
  async config(
    subjectId: string,
    key: string,
    value: string | number | boolean,
  ) {
    // Deliberately perform every transition; caching a previous setter would silently leave the latest arm active.
    await this.client.configuration.setSubject(
      {
        subjectId,
        operationId: this.id(
          `config/${key}/${json(value)}/${Object.keys(this.state.operations).length}`,
        ),
        path: key,
        value,
      },
      this.options,
    );
  }
  async observe(
    source: Source,
    index: number,
    text: string,
    track: "formed" | "raw_control",
    sessionId?: string,
  ) {
    const key = `observe/${track}/${source.id}/${index}`;
    const request = {
      subjectId: this.state.subjects[track],
      sessionId,
      requestId: this.id(key),
      ...webSource(source.url),
      occurredTime: {
        value: { case: "instant" as const, value: at(source.publication_time) },
      },
      admit: true,
      entities:
        track === "formed"
          ? source.entities?.filter((e) => text.includes(e.surface))
          : [],
      material: {
        case: "inlineText" as const,
        value: { text, mediaType: "text/plain" },
      },
    };
    return this.call(key, request, true, () =>
      this.client.cognition.observe(request, this.options),
    );
  }
  async ingestCheckpoint(
    checkpoint: Manifest["checkpoints"][number],
    form: boolean,
  ) {
    const track = "formed",
      subjectId = this.state.subjects.formed;
    const session = await this.call(
      `session/${checkpoint.id}`,
      { subjectId },
      false,
      () => this.client.cognition.openSession({ subjectId }, this.options),
    );
    for (const id of checkpoint.sources) {
      const source = this.manifest.sources.find((s) => s.id === id)!;
      const text = await readFile(source.text_path, "utf8");
      for (const [index, unit] of sourceUnits(text).entries()) {
        const observation = await this.observe(
          source,
          index,
          unit.text,
          track,
          session.sessionId,
        );
        await this.observe(source, index, unit.text, "raw_control");
        if (!form) continue;
        const rawKey = `raw/${id}/${index}`;
        const rawObservation = this.state.operations[
          `observe/raw_control/${id}/${index}`
        ]!.result as Observation;
        const rawRequest = {
          subjectId: this.state.subjects.raw_control,
          operationId: this.id(rawKey),
          input: {
            cognitiveRole: "declarative",
            formationMode: "grounded",
            groundingOccurrenceId: rawObservation.occurrenceId,
            semanticRole: "raw_source_control",
            text: unit.text,
            epistemicClass: "observed",
            supports: [
              {
                support: {
                  case: "evidence" as const,
                  value: {
                    occurrenceId: rawObservation.occurrenceId,
                    locator: { case: "wholeOccurrence" as const, value: true },
                    supportRole: "direct",
                  },
                },
              },
            ],
          },
        };
        await this.call(rawKey, rawRequest, true, () =>
          this.client.memory.form(rawRequest, this.options),
        );
        const formKey = `form/${id}/${index}`;
        if (!this.state.operations[formKey]) await this.budget();
        const request = {
          subjectId,
          occurrenceId: observation.occurrenceId,
          operationId: this.id(formKey),
          aboutnessMode: "select_from_resolved_mentions",
        };
        const formed = await this.call(formKey, request, true, () =>
          this.client.model.formFromObservation(request, this.options),
        );
        if (!formed.memory)
          throw new Error(`Formation returned no Memory: ${id}/${index}`);
        console.log(
          json({
            stage: "formation",
            source: id,
            unit: index,
            memory: formed.memory.memoryId,
            degradation: formed.degradation,
          }).trim(),
        );
      }
    }
  }
  async grant(key: string, subjectId: string, maxModelCalls = 8) {
    const old = this.state.operations[key];
    if (old?.status === "complete") return old.result as Grant;
    if (old) throw new Error(`BLOCKED uncertain grant: ${key}`);
    const remaining = await this.budget();
    const request = {
      subjectId,
      maxOperations: 16,
      maxModelCalls: Math.min(remaining, maxModelCalls),
      maxElapsedMs: Math.min(120000, this.maxElapsed),
    };
    return this.call(key, request, false, () =>
      this.client.cognition.grantMaintenance(request, this.options),
    );
  }
  async captureCut(subjectId: string) {
    const prepared = await this.client.cognition.prepareQuery(
      {
        subjectId,
        nousql: '"source qualification checkpoint"',
        capabilities: {
          textEmbedding: "forbidden",
          rerank: "forbidden",
          queryConceptEnrichment: "forbidden",
        },
      },
      this.options,
    );
    const bound = JSON.parse(prepared.boundQuery) as {
      temporal_frame: {
        clock_now?: string;
        captured_now?: string;
        now?: string;
      };
      authority_watermark: string;
    };
    const clock =
      bound.temporal_frame.clock_now ??
      bound.temporal_frame.captured_now ??
      bound.temporal_frame.now;
    if (!clock || !Number.isFinite(Date.parse(clock)))
      throw new Error(
        `SPEC_GAP: unknown captured CognitiveClock shape: ${json(bound.temporal_frame)}`,
      );
    return { cut: clock, authority_seq: String(bound.authority_watermark) };
  }
  async formation(ingestOnly = false, throughCheckpoint?: string) {
    if (this.state.freeze)
      throw new Error("Frozen Authority cannot accept formation");
    await this.createSubjects();
    await this.config(this.state.subjects.formed, "maintenance.enabled", true);
    for (const c of this.manifest.checkpoints) {
      if (this.state.checkpoints[c.id]) continue;
      await this.ingestCheckpoint(c, !ingestOnly);
      if (ingestOnly) {
        this.state.phases.ingest = "PASS";
        await this.save();
        return;
      }
      const closeKey = `close/${c.id}`;
      const session = this.state.operations[`session/${c.id}`]!.result as {
        sessionId: string;
      };
      await this.call(closeKey, session, true, () =>
        this.client.cognition.closeSession(
          { subjectId: this.state.subjects.formed, id: session.sessionId },
          this.options,
        ),
      );
      for (let g = 0; g < c.maintenance_grants; g++) {
        const receipt = await this.grant(
          `maintenance/${c.id}/${g}`,
          this.state.subjects.formed,
        );
        if (receipt.results.some((r) => r.status === "internal_failure"))
          throw new Error(`BLOCKED production maintenance failure at ${c.id}`);
        if (!receipt.results.length) break;
      }
      const snapshot = await this.snapshot();
      await saveJson(join(this.output, `checkpoint-${c.id}.json`), snapshot);
      this.state.checkpoints[c.id] = {
        ...(await this.captureCut(this.state.subjects.formed)),
        snapshot_digest: digest(json(snapshot)),
      };
      await this.save();
      if (throughCheckpoint === c.id) break;
    }
    this.state.phases.formation = this.manifest.checkpoints.every(
      (c) => this.state.checkpoints[c.id],
    )
      ? "PASS"
      : "IN_PROGRESS";
    await this.save();
  }
  async snapshot(): Promise<Snapshot> {
    const subjectId = this.state.subjects.formed;
    async function list<T>(
      fetch: (token: string) => Promise<{ items: T[]; nextPageToken: string }>,
    ) {
      const items: T[] = [];
      let token = "";
      do {
        const page = await fetch(token);
        items.push(...page.items);
        token = page.nextPageToken;
      } while (token);
      return items;
    }
    const memories = await list((t) =>
      this.client.memory.list(
        { subjectId, page: { pageSize: 64, pageToken: t } },
        this.options,
      ),
    );
    const tags = await list((t) =>
      this.client.concepts.listTags(
        { subjectId, page: { pageSize: 64, pageToken: t } },
        this.options,
      ),
    );
    const episodes = await list((t) =>
      this.client.memory.listEpisodes(
        { subjectId, page: { pageSize: 64, pageToken: t } },
        this.options,
      ),
    );
    const journals = await list((t) =>
      this.client.memory.listJournals(
        { subjectId, page: { pageSize: 64, pageToken: t } },
        this.options,
      ),
    );
    const associations = new Map<string, Snapshot["associations"][number]>(),
      schemaIds = new Set<string>(),
      partial: string[] = [];
    for (const root of [
      ...memories.map((m) => ({
        kind: "memory_revision",
        value: m.revisionId,
      })),
      ...tags.map((t) => ({ kind: "tag", value: t.tagId })),
    ]) {
      const n = await this.client.concepts.neighborhood(
        { subjectId, root, maxNodes: 256, maxDepth: 1 },
        this.options,
      );
      if (n.truncated) partial.push(`neighborhood:${root.kind}:${root.value}`);
      for (const a of n.associations) associations.set(a.associationId, a);
      for (const node of n.nodes)
        if (node.kind === "cognitive_schema" || node.kind === "schema")
          schemaIds.add(node.value);
    }
    const schemas: Snapshot["schemas"] = [];
    for (const schemaId of schemaIds)
      schemas.push(
        await this.client.concepts.getSchema(
          { subjectId, schemaId },
          this.options,
        ),
      );
    // Public API exposes no exhaustive Schema/Accretion inventory. Do not use private Kernel/SQL to fill it.
    partial.push(
      "schema_inventory_only_discovered_neighborhood; accretion_review_from_maintenance_trace",
    );
    return {
      memories,
      tags,
      episodes,
      journals,
      associations: [...associations.values()],
      schemas,
      partial,
    };
  }
  async reviewExport() {
    if (this.state.phases.formation !== "PASS")
      throw new Error("Complete staged formation before review");
    const snapshot = await this.snapshot(),
      fingerprint = digest(json(snapshot));
    await saveJson(join(this.output, "formation-summary.json"), {
      fingerprint,
      snapshot,
      source_receipts: Object.fromEntries(
        Object.entries(this.state.operations).filter(([key]) =>
          key.startsWith("observe/formed/"),
        ),
      ),
      maintenance: Object.fromEntries(
        Object.entries(this.state.operations).filter(([key]) =>
          key.startsWith("maintenance/"),
        ),
      ),
      checkpoints: this.state.checkpoints,
    });
    await writeFile(
      join(this.output, "formation-review.md"),
      `# Formation review\n\nSnapshot: ${fingerprint}\n\nReview every accepted claim, exact source membership, identity, temporal metadata, Tag reuse/overmerge, Association support, Schema boundaries and Accretion trace.\n\nDecision: INCONCLUSIVE until source-backed review.json is supplied.\n`,
      { mode: 0o600 },
    );
    this.state.phases["review-export"] = "PASS";
    await this.save();
  }
  async freeze() {
    if (this.state.freeze) return;
    const review = parseJson<{
      decision: string;
      snapshot_digest: string;
      notes: string[];
      blockers: string[];
    }>(await readFile(join(this.output, "review.json"), "utf8"));
    if (
      review.decision !== "ACCEPTABLE_FOR_RETRIEVAL" ||
      !review.notes?.length ||
      review.blockers?.length
    )
      throw new Error("BLOCKED: formation review does not authorize retrieval");
    const snapshot = await this.snapshot();
    if (digest(json(snapshot)) !== review.snapshot_digest)
      throw new Error("Review snapshot drift");
    const formed = await this.client.subjects.get(
      { subjectId: this.state.subjects.formed },
      this.options,
    );
    const raw = await this.client.subjects.get(
      { subjectId: this.state.subjects.raw_control },
      this.options,
    );
    this.state.freeze = {
      formed: String(formed.authoritySeq),
      raw_control: String(raw.authoritySeq),
      snapshot_digest: review.snapshot_digest,
      review_digest: digest(json(review)),
    };
    this.state.phases.review = "PASS";
    await this.save();
  }
  async assertFrozen(track: "formed" | "raw_control") {
    if (!this.state.freeze) throw new Error("Formation review/freeze required");
    const subject = await this.client.subjects.get(
      { subjectId: this.state.subjects[track] },
      this.options,
    );
    if (String(subject.authoritySeq) !== this.state.freeze[track])
      throw new Error(
        `BLOCKED Authority mutation during frozen retrieval: ${track}`,
      );
    return String(subject.authoritySeq);
  }
  sourceMap(snapshot: Snapshot, track: string) {
    const roots = new Map<string, Set<string>>();
    const add = (key: string | undefined, ids: Iterable<string>) => {
      if (key) {
        const set = roots.get(key) ?? new Set<string>();
        for (const id of ids) set.add(id);
        roots.set(key, set);
      }
    };
    for (const [key, op] of Object.entries(this.state.operations)) {
      if (key.startsWith(`observe/${track}/`) && op.status === "complete") {
        const id = key.split("/")[2]!,
          o = op.result as Observation;
        for (const ref of [o.occurrenceId, o.sourceRegionId, o.artifactId])
          add(ref, [id]);
      }
      if (
        key.startsWith("raw/") &&
        track === "raw_control" &&
        op.status === "complete"
      ) {
        const m = op.result as Memory;
        add(m.memoryId, [key.split("/")[1]!]);
        add(m.revisionId, [key.split("/")[1]!]);
      }
    }
    const propagate = (target: string, refs: (string | undefined)[]) => {
      const ids = refs.flatMap((r) => (r ? [...(roots.get(r) ?? [])] : []));
      add(target, ids);
    };
    for (let pass = 0; pass < 6; pass++) {
      for (const m of snapshot.memories) {
        const refs = m.supports.map((s) =>
          s.support.case === "evidence"
            ? s.support.value.occurrenceId
            : s.support.case === "cognitionDependency"
              ? s.support.value.targetRevision?.value
              : undefined,
        );
        propagate(m.revisionId, refs);
        propagate(m.memoryId, [m.revisionId]);
      }
      for (const e of snapshot.episodes) {
        propagate(
          e.currentRevisionId,
          (e.currentRevision?.members ?? []).map((m) => m.reference?.value),
        );
        propagate(e.episodeId, [e.currentRevisionId]);
      }
      for (const j of snapshot.journals) {
        propagate(
          j.currentRevisionId,
          (j.currentRevision?.sources ?? []).map((s) => s.value),
        );
        propagate(j.journalId, [j.currentRevisionId]);
      }
      for (const s of snapshot.schemas) {
        propagate(
          s.currentRevisionId,
          s.evidenceLinks.map((l) =>
            l.support?.support.case === "evidence"
              ? l.support.support.value.occurrenceId
              : l.support?.support.case === "cognitionDependency"
                ? l.support.support.value.targetRevision?.value
                : undefined,
          ),
        );
        propagate(s.schemaId, [s.currentRevisionId]);
      }
    }
    return roots;
  }
  async queryRequest(
    q: SemanticQuery,
    variant: string,
    track: "formed" | "raw_control",
    tags: Tag[],
  ): Promise<Query> {
    const subjectId = this.state.subjects[track],
      modifiers: NonNullable<Query["expression"]>["modifiers"] = {
        limit: 10,
        materialize: true,
        diagnostics: "full",
      };
    const cues: NonNullable<Query["expression"]>["cues"] = [
      { cue: { case: "text", value: q.text } },
    ];
    if (q.knowledge_cut)
      modifiers.asOf = at(this.state.checkpoints[q.knowledge_cut]!.cut);
    if (variant === "history") modifiers.history = true;
    if (variant === "time" && q.time_axis)
      modifiers.constraints = {
        [q.time_axis]: {
          start: q.time_start ? at(q.time_start) : undefined,
          end: q.time_end ? at(q.time_end) : undefined,
        },
      };
    if (variant === "entity" && q.entity_ref)
      cues.push({ cue: { case: "entityRef", value: q.entity_ref } });
    if (variant === "tag_direct" || variant === "explore") {
      const resolved = await this.client.identity.resolve(
        {
          subjectId,
          kind: "tag",
          locator: { case: "name", value: q.concept_text ?? "" },
        },
        this.options,
      );
      const id =
        resolved.status === "BOUND"
          ? resolved.candidates[0]?.canonical?.value
          : undefined;
      if (!id || !tags.some((t) => t.tagId === id))
        throw new Error(`NOT_RUN: no unique naturally formed Tag for ${q.id}`);
      cues.splice(0, cues.length, { cue: { case: "tagId", value: id } });
      if (variant === "explore") modifiers.exploration = "bounded_associative";
    }
    if (variant === "profiles") modifiers.exploration = "bounded_associative";
    if (track === "raw_control") modifiers.projection = { domains: ["memory"] };
    const request: Query = {
      subjectId,
      expression: { operation: "atom", cues, modifiers },
      capabilities: {
        rerank: "forbidden",
        textEmbedding: variant === "tag_direct" ? "forbidden" : "optional",
        queryConceptEnrichment:
          variant === "model_enrichment" ? "optional" : "forbidden",
      },
      textOnlyCompatibility:
        track === "raw_control" && !q.knowledge_cut && variant === "plain",
    };
    if (variant === "context") {
      const key = "work-context",
        workRequest = {
          subjectId,
          operationId: this.id(key),
          purpose: `Review ${this.manifest.pack_id} source chronology and current support boundaries.`,
        };
      const work = await this.call(key, workRequest, true, () =>
        this.client.cognition.createWorkContext(workRequest, this.options),
      );
      request.workContextId = work.workContext?.workContextId;
    }
    return request;
  }
  async retrieval() {
    await this.freeze();
    const snapshot = await this.snapshot();
    for (const q of this.manifest.queries)
      for (const variant of q.variants)
        for (const track of variant === "plain"
          ? (["formed", "raw_control"] as const)
          : (["formed"] as const)) {
          let preparedDigest: string | undefined;
          for (const profile of variant === "profiles"
            ? profiles
            : ["baseline-rrf"]) {
            const key = `query/${track}/${q.id}/${variant}/${profile}`;
            if (this.state.operations[key]?.status === "complete") continue;
            const authority = await this.assertFrozen(track);
            await this.config(
              this.state.subjects[track],
              "retrieval.cognitive.profile",
              profile,
            );
            await this.config(
              this.state.subjects[track],
              "retrieval.query.concept_enrichment",
              variant === "existing_enrichment"
                ? "existing"
                : variant === "model_enrichment"
                  ? "model"
                  : "off",
            );
            let request: Query;
            try {
              request = await this.queryRequest(
                q,
                variant,
                track,
                snapshot.tags,
              );
            } catch (error) {
              if (
                error instanceof Error &&
                error.message.startsWith("NOT_RUN")
              ) {
                await this.call(
                  key,
                  { q, variant, profile },
                  true,
                  async () => ({ status: "NOT_RUN", reason: error.message }),
                );
                continue;
              }
              throw error;
            }
            const prepared = await this.client.cognition.prepareQuery(
                request,
                this.options,
              ),
              hash = digest(prepared.embeddingText);
            if (preparedDigest && preparedDigest !== hash)
              throw new Error(
                "Prepared semantic input changed across profiles",
              );
            preparedDigest = hash;
            await this.budget();
            const response = await this.call(
              `${key}/response`,
              request,
              false,
              () => this.client.cognition.query(request, this.options),
            );
            const rootMap = this.sourceMap(snapshot, track);
            const matches = response.hits.map((hit) => [
              ...new Set(
                [
                  hit.reference?.value,
                  hit.revision?.value,
                  ...hit.evidence.map((e) => e.reference?.value),
                ].flatMap((r) => (r ? [...(rootMap.get(r) ?? [])] : [])),
              ),
            ]);
            const record = {
              pack: this.manifest.pack_id,
              query_id: response.queryId,
              query_key: q.id,
              track,
              variant,
              profile,
              prepared_digest: hash,
              authority_cut: authority,
              result_refs: response.hits.map(normalizeHit),
              source_matches: matches,
              metrics: sourceMetrics(
                matches,
                q.expected_sources,
                q.forbidden_sources,
              ),
              status: response.status,
              degradation: response.degradation,
              diagnostics: response.diagnostics,
              bound_query: response.boundQuery,
            };
            await this.assertFrozen(track);
            await this.call(
              key,
              { q, variant, profile },
              true,
              async () => record,
            );
            await saveJson(
              join(this.output, "query-results.json"),
              Object.entries(this.state.operations)
                .filter(
                  ([k, o]) =>
                    k.startsWith("query/") &&
                    !k.endsWith("/response") &&
                    o.status === "complete",
                )
                .map(([, o]) => o.result),
            );
          }
        }
    this.state.phases.retrieval = "PASS";
    await this.save();
  }
  async feedback() {
    if (this.state.phases.retrieval !== "PASS")
      throw new Error("Complete main frozen retrieval before feedback");
    const subjectId = this.state.subjects.formed;
    const chosen = Object.entries(this.state.operations).find(
      ([k, o]) =>
        k.startsWith("query/formed/") &&
        k.endsWith("/response") &&
        (o.result as Response).hits.some((h) => h.revision),
    );
    if (!chosen)
      throw new Error(
        "BLOCKED: feedback requires an actual returned exact revision",
      );
    const response = chosen[1].result as Response,
      ref = response.hits.find((h) => h.revision)!.revision!;
    await this.config(subjectId, "maintenance.concept_use_review_interval", 1);
    const records = [];
    for (const kind of ["presented", "result_supported", "result_refuted"]) {
      const key = `feedback/${kind}`,
        old = this.state.operations[`${key}/input`];
      const request = (old?.result as
        Parameters<NousClient["cognition"]["reportUse"]>[0] | undefined) ?? {
        subjectId,
        consumerRef: "consumer:core-cognition-qualification",
        events: [
          {
            eventId: this.id(key),
            reference: ref,
            kind,
            occurredAt: at(new Date().toISOString()),
            queryId: response.queryId,
          },
        ],
      };
      await this.call(`${key}/input`, { kind }, true, async () => request);
      const before = await this.client.subjects.get(
          { subjectId },
          this.options,
        ),
        tagsBefore = await this.snapshot();
      const accepted = await this.call(key, request, true, () =>
        this.client.cognition.reportUse(request, this.options),
      );
      const duplicate = await this.call(`${key}/duplicate`, request, true, () =>
        this.client.cognition.reportUse(request, this.options),
      );
      const tagsAfter = await this.snapshot(),
        after = await this.client.subjects.get({ subjectId }, this.options);
      records.push({
        kind,
        request,
        accepted,
        duplicate,
        authority_before: before.authoritySeq,
        authority_after: after.authoritySeq,
        concepts_unchanged:
          digest(
            json({
              tags: tagsBefore.tags,
              associations: tagsBefore.associations,
            }),
          ) ===
          digest(
            json({
              tags: tagsAfter.tags,
              associations: tagsAfter.associations,
            }),
          ),
      });
      if (duplicate.duplicateCount !== 1 || duplicate.acceptedCount !== 0)
        throw new Error("BLOCKED duplicate UseEvent reinforcement");
      if (
        digest(json(tagsBefore.tags)) !== digest(json(tagsAfter.tags)) ||
        digest(json(tagsBefore.associations)) !==
          digest(json(tagsAfter.associations))
      )
        throw new Error("BLOCKED concept mutation without Host grant");
    }
    const grant = await this.grant("feedback/maintenance", subjectId, 8);
    await saveJson(join(this.output, "feedback-results.json"), {
      override: { concept_use_review_interval: 1 },
      records,
      grant,
      after: await this.snapshot(),
      limitation:
        "Negative signal and presented need behavior require maintenance input/trace review; legal no_change/reject accepted.",
    });
    this.state.phases.feedback = "PASS";
    await this.save();
  }
  async accounting() {
    const telemetryPath = this.ledger + ".telemetry.jsonl";
    let rows: {
      attempt: number;
      endpoint: string;
      usage: Record<string, number>;
      status: number | null;
    }[] = [];
    try {
      rows = (await readFile(telemetryPath, "utf8"))
        .trim()
        .split("\n")
        .filter(Boolean)
        .map((l) => JSON.parse(l) as (typeof rows)[number]);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    const roles: Record<string, number> = {},
      usage: Record<string, number> = {};
    let embeddingItems = 0;
    for (const row of rows) {
      const meta = JSON.parse(
        await readFile(
          join(this.traceRoot, String(row.attempt), "meta.json"),
          "utf8",
        ),
      ) as { request: { file: string } };
      const req = JSON.parse(
        await readFile(
          join(this.traceRoot, String(row.attempt), meta.request.file),
          "utf8",
        ),
      ) as {
        input?: unknown;
        response_format?: { json_schema?: { name?: string } };
        text?: { format?: { name?: string } };
      };
      const role =
        row.endpoint === "/embeddings"
          ? "embedding"
          : row.endpoint === "/rerank"
            ? "rerank"
            : (req.response_format?.json_schema?.name ??
              req.text?.format?.name ??
              "unknown_generation");
      roles[role] = (roles[role] ?? 0) + 1;
      if (row.endpoint === "/embeddings")
        embeddingItems += Array.isArray(req.input) ? req.input.length : 1;
      for (const [k, v] of Object.entries(row.usage))
        usage[k] = (usage[k] ?? 0) + v;
    }
    await saveJson(join(this.output, "cost.json"), {
      ledger: this.ledger,
      trace_root: this.traceRoot,
      wire_attempts: rows.length,
      roles,
      embedding_items: embeddingItems,
      usage,
      failures: rows.filter((r) => !r.status || r.status >= 400).length,
      cost: "unknown",
      scope:
        "shared gateway ledger cumulative, including failed/retried attempts",
    });
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  const { values } = parseArgs({
    options: {
      "run-root": { type: "string" },
      "through-checkpoint": { type: "string" },
      manifest: { type: "string" },
      phase: { type: "string", default: "all" },
      output: { type: "string" },
      ledger: { type: "string" },
      "trace-root": { type: "string" },
      identities: { type: "string" },
      "max-model-calls": { type: "string" },
      "max-elapsed-ms": { type: "string" },
    },
  });
  if (
    !values.manifest ||
    !values.output ||
    !values.ledger ||
    !values.identities ||
    !values["trace-root"] ||
    !values["max-model-calls"] ||
    !values["max-elapsed-ms"]
  )
    throw new Error(
      "Require manifest, output, ledger, trace-root, identities, max-model-calls and max-elapsed-ms",
    );
  const maxCalls = Number(values["max-model-calls"]),
    maxElapsed = Number(values["max-elapsed-ms"]);
  if (
    !Number.isSafeInteger(maxCalls) ||
    maxCalls < 1 ||
    maxCalls > 100000 ||
    !Number.isSafeInteger(maxElapsed) ||
    maxElapsed < 1
  )
    throw new Error("Explicit positive bounded budgets required");
  if (
    ![
      "acquire-check",
      "ingest",
      "formation",
      "review-export",
      "retrieval",
      "feedback",
      "all",
    ].includes(values.phase)
  )
    throw new Error("Unknown phase");
  const output = ignoredPath(values.output);
  await mkdir(output, { recursive: true, mode: 0o700 });
  const lock = await open(join(output, "runner.lock"), "wx", 0o600);
  await lock.writeFile(String(process.pid));
  try {
    const loaded = await loadManifest(resolve(values.manifest));
    const supplied = JSON.parse(
      await readFile(values.identities, "utf8"),
    ) as Identities;
    const identities = {
      ...supplied,
      manifest: loaded.manifestDigest,
      oracle: loaded.oracleDigest,
      source: digest(
        json(
          loaded.manifest.sources.map((s) => [
            s.id,
            s.raw_sha256,
            s.source_text_sha256,
          ]),
        ),
      ),
      code: execFileSync("git", ["rev-parse", "HEAD"], {
        encoding: "utf8",
      }).trim(),
      runner: digest(
        (await readFile(new URL(import.meta.url))).toString() +
          (await readFile(
            new URL("./core-cognition-contracts.ts", import.meta.url),
            "utf8",
          )),
      ),
    };
    let state: RunState;
    try {
      state = parseJson<RunState>(
        await readFile(join(output, "state.json"), "utf8"),
      );
      assertResume(state.identities, identities, state.sealed);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
      const runId = stableId(output, loaded.manifestDigest);
      state = {
        version: 1,
        run_id: runId,
        identities,
        sealed: loaded.manifest.role === "sealed",
        started_at: new Date().toISOString(),
        subjects: {
          formed: stableId(runId, "formed"),
          raw_control: stableId(runId, "raw-control"),
        },
        operations: {},
        checkpoints: {},
        phases: {},
      };
    }
    state.executions ??= [];
    state.executions.push({
      phase: values.phase,
      started_at: new Date().toISOString(),
      identities,
      max_calls: maxCalls,
      max_elapsed_ms: maxElapsed,
      status: "IN_PROGRESS",
    });
    await saveJson(join(output, "state.json"), state);
    for (const s of loaded.manifest.sources) {
      verifyDigest(await readFile(s.raw_path), s.raw_sha256, `${s.id}/raw`);
      verifyDigest(
        await readFile(s.text_path),
        s.source_text_sha256,
        `${s.id}/text`,
      );
    }
    if (values.phase === "acquire-check") {
      state.phases["acquire-check"] = "PASS";
      await saveJson(join(output, "state.json"), state);
      console.log("Source digests PASS");
    } else {
      if (!values["run-root"])
        throw new Error("Official existing Core run-root is required");
      const client = await connectNousInstance({ runRoot: values["run-root"] });
      const run = new SemanticRun(
        client,
        loaded.manifest,
        state,
        output,
        ignoredPath(values.ledger),
        maxCalls,
        maxElapsed,
        ignoredPath(values["trace-root"]),
      );
      const config = await client.configuration.get({
        view: ConfigurationView.ACTIVE,
        exposureCeiling: ConfigExposure.DEVELOPER,
      });
      if (supplied.active_config !== config.effectiveDigest)
        throw new Error("Active Core config differs from frozen identity");
      try {
        await run.checkSources();
        if (["ingest", "formation", "all"].includes(values.phase))
          await run.formation(
            values.phase === "ingest",
            values["through-checkpoint"],
          );
        if (
          ["review-export", "formation", "all"].includes(values.phase) &&
          state.phases.formation === "PASS"
        )
          await run.reviewExport();
        if (["retrieval", "all"].includes(values.phase)) await run.retrieval();
        if (values.phase === "feedback") await run.feedback();
      } finally {
        await run.accounting();
        await run.save();
      }
    }
    await saveJson(join(output, "run.json"), {
      ...state,
      operations: undefined,
      output,
      ledger: values.ledger,
      trace_root: values["trace-root"],
      max_calls: maxCalls,
      max_elapsed_ms: maxElapsed,
      updated_at: new Date().toISOString(),
    });
  } finally {
    await lock.close();
    await unlink(join(output, "runner.lock"));
  }
}
