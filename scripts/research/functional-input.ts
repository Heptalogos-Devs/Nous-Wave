import { parseArgs } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { loadFunctionalCorpus } from "./functional-plan.js";

// Resolve authored intents against the actual formation receipts/catalog. The
// oracle and natural-language surface examples never enter CognitiveQuery.
const { values } = parseArgs({
  options: {
    corpus: { type: "string", default: "docs/research/corpus/functional" },
    root: { type: "string" },
    "query-ids": { type: "string" },
    "concept-bindings": { type: "string" },
    output: { type: "string" },
  },
});
if (!values.root || !values.output)
  throw new Error("--root and --output required");
const root = resolve(values.root);
const output = resolve(values.output);
if (
  ![root, output].every((path) =>
    path.startsWith(resolve("data/research") + "/"),
  )
)
  throw new Error("Research-owned paths required");
const corpus = await loadFunctionalCorpus(
  values.corpus!,
  values["query-ids"]?.split(","),
);
const state = JSON.parse(
  await readFile(resolve(root, "formation-state.json"), "utf8"),
) as {
  digest: string;
  subjects: Record<
    string,
    {
      id: string;
      workContext?: string;
      events: Record<string, { occurrenceId: string }>;
    }
  >;
};
if (state.digest !== corpus.sourceDigest)
  throw new Error("Source receipts differ from corpus");
const inspection = JSON.parse(
  await readFile(resolve(root, "formation-inspection.json"), "utf8"),
) as {
  tags: { tag_id: string; subject_id: string; status: string }[];
  episodes: {
    episode_revision_id: string;
    subject_id: string;
    members: { ref_kind: string; ref_value: string }[];
  }[];
};
const concepts: Record<string, string> = values["concept-bindings"]
  ? (JSON.parse(await readFile(values["concept-bindings"], "utf8")) as Record<
      string,
      string
    >)
  : {};
const unresolved: { key: string; reason: string }[] = [];
const queries = corpus.queries.flatMap((intent) => {
  try {
    const subject = state.subjects[intent.scenario_key];
    if (!subject) throw new Error("Scenario has no formation receipts");
    const prepared = intent.prepared_query;
    const cues: Record<string, unknown>[] = [
      { kind: "text", text: prepared.canonical_text },
    ];
    for (const key of prepared.entity_keys ?? [])
      cues.push({
        kind: "entity",
        entity_ref: `entity:functional:${intent.scenario_key}:${key}`,
      });
    for (const key of prepared.concept_keys ?? []) {
      const tag = concepts[`${intent.scenario_key}:${key}`];
      if (
        !inspection.tags.some(
          (candidate) =>
            candidate.tag_id === tag &&
            candidate.subject_id === subject.id &&
            candidate.status === "active",
        )
      )
        throw new Error(`Concept ${key} lacks a selected active owner Tag`);
      cues.push({ kind: "tag", tag });
    }
    const targets: Record<string, unknown>[] = [
      { kind: "any_relevant_cognition" },
    ];
    for (const key of prepared.exact_event_keys ?? []) {
      const occurrence = subject.events[key]?.occurrenceId;
      const episodes = inspection.episodes.filter(
        (episode) =>
          episode.subject_id === subject.id &&
          episode.members.some(
            (member) =>
              member.ref_kind === "occurrence" &&
              member.ref_value === occurrence,
          ),
      );
      // A composed Episode can legitimately retain the same occurrence as its
      // atomic source Episode. An exact event intent binds the unique smallest
      // owner scope; equal-specificity alternatives remain unresolved.
      const width = Math.min(
        ...episodes.map((episode) => episode.members.length),
      );
      const exact = episodes.filter(
        (episode) => episode.members.length === width,
      );
      if (exact.length !== 1)
        throw new Error(
          `Event ${key} has ${exact.length} most-specific Episode bindings`,
        );
      targets.push({
        kind: "exact",
        reference: {
          kind: "episode_revision",
          id: exact[0]!.episode_revision_id,
        },
      });
    }
    const constraints: Record<string, unknown> = {};
    if (prepared.time_axis) {
      const axis = { valid_time: "valid", occurred_time: "occurred" }[
        prepared.time_axis
      ];
      if (!axis) throw new Error("Unsupported time axis");
      constraints[axis] = {
        start: prepared.time_start ?? null,
        end: prepared.time_end ?? null,
      };
    }
    if (prepared.work_context_key && !subject.workContext)
      throw new Error("WorkContext receipt missing");
    return [
      {
        key: intent.key,
        as_of: prepared.as_of,
        query: {
          api_version: 1,
          subject: subject.id,
          work_context: prepared.work_context_key ? subject.workContext : null,
          session: null,
          situation: {},
          text_only_compatibility: false,
          expression: { operation: "atom", targets, cues, constraints },
          exploration:
            prepared.exploration === "associative"
              ? "bounded_associative"
              : (prepared.exploration ?? "none"),
          resources: {},
          result_need: { limit: 16 },
          effort: "deep",
          capabilities: {
            text_embedding: "preferred",
            rerank: "forbidden",
            residual_sensing: "forbidden",
            multimodal_interpretation: "forbidden",
          },
          diagnostics: "full",
        },
      },
    ];
  } catch (error) {
    unresolved.push({ key: intent.key, reason: (error as Error).message });
    return [];
  }
});
await writeFile(
  output,
  JSON.stringify({ queries, unresolved }, null, 2) + "\n",
  { mode: 0o600 },
);
console.log(
  JSON.stringify({ resolved: queries.length, unresolved, providerCalls: 0 }),
);
