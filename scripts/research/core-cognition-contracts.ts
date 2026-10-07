// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { createHash } from "node:crypto";
import { readFile, rename, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve, relative, sep } from "node:path";
import type { NousClient } from "@nous-wave/client";

export const digest = (value: string | Uint8Array) =>
  createHash("sha256").update(value).digest("hex");
export const json = (value: unknown) =>
  JSON.stringify(
    value,
    (_key, v: unknown) => (typeof v === "bigint" ? { $bigint: String(v) } : v),
    2,
  ) + "\n";
export const parseJson = <T>(text: string): T =>
  JSON.parse(text, (_key, v: unknown) =>
    v && typeof v === "object" && "$bigint" in v
      ? BigInt(String(v.$bigint))
      : v,
  ) as T;
export function stableId(run: string, key: string) {
  const h = digest(`${run}\0${key}`);
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-5${h.slice(13, 16)}-a${h.slice(17, 20)}-${h.slice(20, 32)}`;
}
export async function saveJson(path: string, value: unknown) {
  await mkdir(dirname(path), { recursive: true, mode: 0o700 });
  const temp = `${path}.tmp`;
  await writeFile(temp, json(value), { mode: 0o600 });
  await rename(temp, path);
}
export function ignoredPath(path: string) {
  const full = resolve(path),
    rel = relative(resolve("data/research"), full);
  if (rel === ".." || rel.startsWith(`..${sep}`) || rel.startsWith(sep))
    throw new Error("Output/raw paths must be under ignored data/research");
  return full;
}
export function verifyDigest(
  bytes: Uint8Array,
  expected: string,
  label: string,
) {
  if (digest(bytes) !== expected) throw new Error(`${label} digest mismatch`);
}
export interface Source {
  id: string;
  url: string;
  version_identity: string;
  publication_time: string;
  raw_sha256: string;
  source_text_sha256: string;
  extraction: string;
  rights: string;
  raw_path: string;
  text_path: string;
  entities?: { surface: string; entityRef: string; semanticRole?: string }[];
}
export interface SemanticQuery {
  id: string;
  text: string;
  category: string;
  expected_sources: string[];
  forbidden_sources: string[];
  oracle_notes: string;
  variants: string[];
  knowledge_cut?: string;
  time_axis?: "occurred" | "observed" | "valid" | "formed" | "recorded";
  time_start?: string;
  time_end?: string;
  entity_ref?: string;
  concept_text?: string;
}
export interface Manifest {
  version: 1;
  pack_id: string;
  role: "calibration" | "sealed" | "reserve";
  sources: Source[];
  checkpoints: { id: string; sources: string[]; maintenance_grants: number }[];
  queries: SemanticQuery[];
  review_notes?: string[];
}
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("Expected object");
  return value as Record<string, unknown>;
}
function text(value: unknown, name: string): string {
  if (typeof value !== "string" || !value.trim())
    throw new Error(`Missing ${name}`);
  return value;
}
function array(value: unknown, name: string): unknown[] {
  if (!Array.isArray(value) || !value.length)
    throw new Error(`Missing ${name} array`);
  return value as unknown[];
}
function strings(value: unknown, name: string): string[] {
  if (!Array.isArray(value) || value.some((x) => typeof x !== "string"))
    throw new Error(`Invalid ${name} strings`);
  return value as string[];
}
export function validateManifest(value: unknown): Manifest {
  const m = record(value);
  if (
    m.version !== 1 ||
    !["calibration", "sealed", "reserve"].includes(String(m.role))
  )
    throw new Error("Invalid manifest version/role");
  text(m.pack_id, "pack_id");
  const sources = array(m.sources, "sources").map((v) => {
    const s = record(v);
    for (const k of [
      "id",
      "url",
      "version_identity",
      "publication_time",
      "extraction",
      "rights",
      "raw_path",
      "text_path",
    ])
      text(s[k], k);
    const url = new URL(String(s.url));
    if (url.protocol !== "https:" || url.username || url.password)
      throw new Error("Invalid source URL");
    if (!Number.isFinite(Date.parse(String(s.publication_time))))
      throw new Error("Invalid publication time");
    for (const k of ["raw_sha256", "source_text_sha256"])
      if (!/^[a-f0-9]{64}$/.test(String(s[k]))) throw new Error(`Invalid ${k}`);
    ignoredPath(String(s.raw_path));
    ignoredPath(String(s.text_path));
    return s as unknown as Source;
  });
  const known = new Set(sources.map((s) => s.id));
  if (known.size !== sources.length) throw new Error("duplicate source id");
  const seen = new Set<string>(),
    cuts = new Set<string>();
  for (const v of array(m.checkpoints, "checkpoints")) {
    const c = record(v),
      id = text(c.id, "checkpoint id");
    if (cuts.has(id)) throw new Error("duplicate checkpoint");
    cuts.add(id);
    if (
      !Number.isSafeInteger(c.maintenance_grants) ||
      Number(c.maintenance_grants) < 0 ||
      Number(c.maintenance_grants) > 16
    )
      throw new Error("Invalid maintenance bound");
    for (const source of strings(c.sources, "checkpoint sources")) {
      if (!known.has(source) || seen.has(source))
        throw new Error("Unknown/duplicate checkpoint source");
      seen.add(source);
    }
  }
  if (seen.size !== known.size)
    throw new Error("Every source must have one checkpoint");
  const queries = new Set<string>();
  for (const v of array(m.queries, "queries")) {
    const q = record(v),
      id = text(q.id, "query id");
    if (queries.has(id)) throw new Error("duplicate query id");
    queries.add(id);
    text(q.text, "query text");
    text(q.category, "category");
    text(q.oracle_notes, "oracle_notes");
    for (const k of ["expected_sources", "forbidden_sources"])
      for (const id of strings(q[k], k))
        if (!known.has(id)) throw new Error("Unknown query oracle source");
    for (const variant of strings(q.variants, "variants"))
      if (
        ![
          "plain",
          "time",
          "history",
          "entity",
          "context",
          "tag_direct",
          "explore",
          "existing_enrichment",
          "model_enrichment",
          "profiles",
        ].includes(variant)
      )
        throw new Error("Unknown query variant");
    if (q.knowledge_cut && !cuts.has(String(q.knowledge_cut)))
      throw new Error("Unknown knowledge cut");
  }
  return m as unknown as Manifest;
}
/** Simon remains referenced to its original metadata and oracle owner, not duplicated. */
export async function loadManifest(path: string) {
  const spec = record(JSON.parse(await readFile(path, "utf8")));
  if (spec.source_catalog) {
    const catalog = JSON.parse(
      await readFile(text(spec.source_catalog, "source_catalog"), "utf8"),
    ) as {
      sources: (Omit<Source, "publication_time" | "version_identity"> & {
        publication_date: string;
        topic: string;
      })[];
    };
    const paths = record(spec.source_paths);
    spec.sources = strings(spec.source_ids, "source_ids").map((id) => {
      const s = catalog.sources.find((s) => s.id === id);
      if (!s) throw new Error(`Unknown catalog source ${id}`);
      return {
        ...s,
        ...record(paths[id]),
        publication_time: s.publication_date + "T00:00:00Z",
        version_identity: `raw-sha256:${s.raw_sha256}`,
      };
    });
  }
  if (spec.query_catalog) {
    const catalog = JSON.parse(
      await readFile(text(spec.query_catalog, "query_catalog"), "utf8"),
    ) as {
      queries: {
        id: string;
        text: string;
        expected_units: string[];
        verification: string;
      }[];
      absence_control: { query: string; oracle: string };
    };
    const additions = (spec.queries ?? []) as SemanticQuery[];
    spec.queries = [
      ...catalog.queries.map((q) => ({
        id: q.id,
        text: q.text,
        category: q.expected_units.length > 1 ? "chronology" : "direct",
        expected_sources: q.expected_units,
        forbidden_sources: [],
        oracle_notes: q.verification,
        variants: ["plain"],
      })),
      {
        id: "salary-absence",
        text: catalog.absence_control.query,
        category: "absence",
        expected_sources: [],
        forbidden_sources: [],
        oracle_notes: catalog.absence_control.oracle,
        variants: ["plain"],
      },
      ...additions,
    ];
  }
  const manifest = validateManifest(spec);
  return {
    manifest,
    manifestDigest: digest(json(spec)),
    oracleDigest: digest(json(manifest.queries)),
  };
}
export type Identities = Record<string, string>;
export function assertResume(
  before: Identities,
  current: Identities,
  sealed: boolean,
) {
  for (const key of sealed
    ? new Set([...Object.keys(before), ...Object.keys(current)])
    : ["manifest", "oracle", "source", "model", "schema", "config"])
    if (before[key] !== current[key])
      throw new Error(
        `Resume ${key} identity changed; use a new qualification run`,
      );
}
type QueryResponse = Awaited<ReturnType<NousClient["cognition"]["query"]>>;
export function normalizeHit(
  hit: Pick<
    QueryResponse["hits"][number],
    "reference" | "revision" | "evidence" | "entityRefs"
  > &
    Partial<QueryResponse["hits"][number]>,
) {
  return {
    reference: hit.reference,
    revision: hit.revision,
    evidence: hit.evidence,
    entity_refs: hit.entityRefs,
    formed_at: hit.formedAt,
    recorded_at: hit.recordedAt,
    score: hit.score,
    evidence_families: hit.evidenceFamilies,
  };
}
export function sourceMetrics(
  matches: string[][],
  expected: string[],
  forbidden: string[],
) {
  const set = [...new Set(expected)],
    ranks = set.map((id) => matches.findIndex((row) => row.includes(id)) + 1);
  const recall = (k: number) =>
    set.length
      ? ranks.filter((r) => r > 0 && r <= k).length / set.length
      : null;
  const first =
    matches.findIndex((row) => row.some((id) => set.includes(id))) + 1;
  return {
    recall_at_1: recall(1),
    recall_at_5: recall(5),
    recall_at_10: recall(10),
    all_support_at_5: set.length ? recall(5) === 1 : null,
    all_support_at_10: set.length ? recall(10) === 1 : null,
    mrr: set.length ? (first ? 1 / first : 0) : null,
    expected_ranks: Object.fromEntries(set.map((s, i) => [s, ranks[i]])),
    forbidden_sources: [
      ...new Set(matches.flat().filter((id) => forbidden.includes(id))),
    ],
    empty_result: matches.length === 0,
  };
}
