import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

type Source = {
  id: string;
  url: string;
  topic: string;
  entity_ref: string;
  cluster: string;
  revision?: string;
  publication_date?: string;
  raw_sha256: string;
};
type Unit = {
  id: string;
  source: string;
  ordinal: number;
  anchor: string | null;
  sha256: string;
};
type AuthoredQuery = {
  id: string;
  text: string;
  category: string;
  expected_units: string[];
  acceptable_units: string[];
  harmful_stale_units: string[];
  oracle_status: string;
  verification_note: string;
  judged_absence?: boolean;
  source_locators: {
    unit?: string;
    url?: string;
    sha256?: string;
    ordinal?: number;
    manifest_sha256?: string;
    absent_literal_entity?: string;
  }[];
};
const sha = (text: string | Buffer) =>
  createHash("sha256").update(text).digest("hex");

export async function prepareHardText(
  manifestPath: string,
  queryPath: string,
  textsPath: string,
  output: string,
) {
  const root = resolve(output);
  if (!root.startsWith(resolve("data/research") + "/"))
    throw new Error(
      "Prepared third-party text must remain in ignored data/research",
    );
  const manifestBytes = await readFile(manifestPath);
  const manifest = JSON.parse(manifestBytes.toString("utf8")) as {
    sources: Source[];
    units: Unit[];
  };
  const queryBytes = await readFile(queryPath);
  const authored = JSON.parse(queryBytes.toString("utf8")) as {
    status: string;
    queries: AuthoredQuery[];
  };
  const texts = JSON.parse(await readFile(textsPath, "utf8")) as Record<
    string,
    string
  >;
  const sources = new Map(manifest.sources.map((s) => [s.id, s]));
  const units = new Map(manifest.units.map((u) => [u.id, u]));
  if (
    sources.size !== manifest.sources.length ||
    units.size !== manifest.units.length
  )
    throw new Error("Duplicate hard-text source/unit identity");
  const subject = "hard-text";
  const ingestedAt = "2026-10-04T00:00:00.000Z";
  const queryAt = "2026-10-04T00:00:01.000Z";
  const sourceTags = (s: Source) =>
    s.revision
      ? [
          `${s.cluster === "evolving_python" ? "Python" : "PostgreSQL"} ${s.revision}`,
        ]
      : ["Simon Willison"];
  const documentHeader = (source: Source) =>
    `Document: ${source.topic}\nSource: ${source.url}\nRevision: ${source.revision ?? source.publication_date}`;
  const sourceDocuments = Object.fromEntries(
    manifest.sources.map((source) => [
      source.id,
      {
        url: source.url,
        raw_sha256: source.raw_sha256,
        text:
          documentHeader(source) +
          "\n\n" +
          manifest.units
            .filter((u) => u.source === source.id)
            .map((u) => texts[u.id])
            .join("\n\n"),
      },
    ]),
  );
  const events = manifest.units.map((unit) => {
    const source = sources.get(unit.source);
    const text = texts[unit.id];
    if (!source || text === undefined || sha(text) !== unit.sha256)
      throw new Error(`Frozen hard-text unit mismatch: ${unit.id}`);
    return {
      event_id: `${subject}-${unit.id}`,
      subject,
      source_document: source.id,
      occurred_at: source.publication_date
        ? `${source.publication_date}T00:00:00.000Z`
        : ingestedAt,
      observed_at: ingestedAt,
      formed_at: ingestedAt,
      recorded_at: ingestedAt,
      valid_time: { start: ingestedAt, end: null },
      session: source.id,
      entities: [source.entity_ref.replace(/^entity:/, "")],
      tags: sourceTags(source),
      relations: [],
      renders: [
        {
          text: `${documentHeader(source)}\nUnit: ${unit.id}\n${text}`,
        },
      ],
      source_locator: {
        url: source.url,
        anchor: unit.anchor,
        ordinal: unit.ordinal,
        sha256: unit.sha256,
      },
    };
  });
  const queryIds = new Set<string>();
  const queries = authored.queries.map((q) => {
    if (queryIds.has(q.id))
      throw new Error("Duplicate hard-text query identity");
    queryIds.add(q.id);
    for (const proof of q.source_locators) {
      if (proof.unit) {
        const unit = units.get(proof.unit);
        if (
          !unit ||
          proof.sha256 !== unit.sha256 ||
          proof.ordinal !== unit.ordinal ||
          proof.url !== sources.get(unit.source)!.url
        )
          throw new Error(
            `Query source locator differs from frozen unit: ${q.id}`,
          );
      } else if (q.judged_absence) {
        const term = proof.absent_literal_entity;
        if (
          proof.manifest_sha256 !== sha(manifestBytes) ||
          !term ||
          !q.text.includes(term) ||
          Object.values(texts).some((text) =>
            text.toLowerCase().includes(term.toLowerCase()),
          )
        )
          throw new Error(
            `Scoped absence proof differs from source corpus: ${q.id}`,
          );
      } else throw new Error(`Query lacks a source unit locator: ${q.id}`);
    }
    if (
      !q.source_locators.length ||
      q.expected_units.some(
        (id) => !q.source_locators.some((proof) => proof.unit === id),
      )
    )
      throw new Error(`Query positive units lack source locators: ${q.id}`);
    const oracle: Record<
      string,
      { grade: number; reason: string; harmful_kind?: string }
    > = {};
    for (const [ids, grade, reason] of [
      [q.expected_units, 3, "direct_fact"],
      [q.acceptable_units, 1, "acceptable_background"],
      [q.harmful_stale_units, -1, "stale"],
    ] as const) {
      for (const id of ids) {
        if (!units.has(id) || `${subject}-${id}` in oracle)
          throw new Error(`Invalid or conflicting query relevance unit: ${id}`);
        oracle[`${subject}-${id}`] = {
          grade,
          reason,
          ...(grade < 0 ? { harmful_kind: "stale" } : {}),
        };
      }
    }
    return {
      query_id: `${subject}-${q.id}`,
      suite: "hard-text-ir",
      subject,
      category: q.category,
      text: q.text,
      as_of: queryAt,
      oracle,
      oracle_status: q.oracle_status,
      judged_absence: q.judged_absence ?? false,
      verification_note: q.verification_note,
      time_axis: null,
      session_oracle: [
        ...new Set(q.expected_units.map((id) => units.get(id)!.source)),
      ],
      input_context: {
        tag_labels: [...new Set(manifest.sources.flatMap(sourceTags))].filter(
          (tag) => q.text.includes(tag),
        ),
        current_event_ids: [],
        source: "literal source-revision or author cue in authored question",
      },
    };
  });
  await mkdir(resolve(root, "scenarios"), { recursive: true });
  const scenarioText =
    JSON.stringify({
      scenario_id: subject,
      suite: "hard-text-ir",
      source_documents: sourceDocuments,
      events,
    }) + "\n";
  const queriesText = JSON.stringify({ queries }) + "\n";
  await writeFile(resolve(root, "scenarios/hard-text.json"), scenarioText, {
    mode: 0o600,
  });
  await writeFile(resolve(root, "queries.json"), queriesText, { mode: 0o600 });
  const prepared = {
    schema_version: 1,
    suite: "hard-text-ir",
    adapter_version: "hard-text-shared-document-v2",
    source_sha256: sha(manifestBytes),
    authored_queries_sha256: sha(queryBytes),
    scenario_files: [
      {
        path: "scenarios/hard-text.json",
        sha256: sha(scenarioText),
        events: events.length,
      },
    ],
    queries_sha256: sha(queriesText),
    event_count: events.length,
    query_count: queries.length,
    oracle_policy: "explicit_qrels_unjudged_others",
    source_set_unit: "document_session",
    query_audit_status: authored.status,
    time_mapping:
      "controlled ingestion/formation at frozen archive time; original publication day only where source metadata supplies it; version questions remain textual revision IR",
    status: "prepared_not_imported_not_benchmarked",
  };
  await writeFile(
    resolve(root, "manifest.json"),
    JSON.stringify(prepared, null, 2) + "\n",
    { mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      suite: prepared.suite,
      events: events.length,
      queries: queries.length,
      query_audit_status: authored.status,
    }),
  );
}
