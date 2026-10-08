// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import type { CliEnvironment } from "./runtime.js";

const uuid = /^[0-9a-f]{8}-[0-9a-f-]{27}$/i;
const idKinds: Record<string, string> = {
  subjectId: "subject",
  sessionId: "session",
  workContextId: "work_context",
  activeWorkContextId: "work_context",
  memoryId: "memory",
  memoryRevisionId: "memory_revision",
  schemaId: "cognitive_schema",
  schemaRevisionId: "cognitive_schema_revision",
  episodeId: "episode",
  episodeRevisionId: "episode_revision",
  journalId: "journal",
  journalRevisionId: "journal_revision",
  seedVersionId: "cognitive_seed_version",
  tagId: "tag",
  canonicalTagId: "tag",
  artifactId: "artifact",
  occurrenceId: "occurrence",
  groundingOccurrenceId: "occurrence",
  sourceRegionId: "source_region",
  parentSourceRegionId: "source_region",
  representationId: "derived_representation",
  derivedRepresentationId: "derived_representation",
  selectedRepresentationId: "derived_representation",
  derivedRegionId: "derived_region",
  associationId: "association",
};
const kinds = new Set([
  ...Object.values(idKinds),
  "memory_revision",
  "cognitive_schema_revision",
  "episode_revision",
  "journal_revision",
  "cognitive_seed_version",
  "entity",
  "resource",
]);
function directoryLabel(text: string) {
  let value = "";
  for (const character of text) {
    if (Buffer.byteLength(value + character) > 256) break;
    value += character;
  }
  return value;
}

/** Display addresses come from the existing Authority directory, never truncated IDs. */
export async function friendlyOutput(
  input: unknown,
  env: CliEnvironment,
): Promise<unknown> {
  const cache = new Map<string, Promise<string>>();
  const top =
    input && typeof input === "object"
      ? (input as Record<string, unknown>)
      : {};
  const subjectId =
    typeof top.subjectId === "string" ? top.subjectId : env.subjectId;
  const address = (kind: string, value: string, label = "") => {
    const key = `${subjectId}:${kind}:${value}`;
    if (!cache.has(key))
      cache.set(
        key,
        env.client.identity
          .bind({
            subjectId: kind === "subject" ? value : subjectId,
            canonical: { kind, value },
            displayName: directoryLabel(label),
            aliases: [],
            addressOnly: true,
          })
          .then((binding) => binding.lexicalRef),
      );
    return cache.get(key)!;
  };
  async function visit(
    value: unknown,
    key = "",
    parent: Record<string, unknown> = {},
  ): Promise<unknown> {
    // Detailed wire traces remain available through --developer / --json.
    if (key === "diagnostics") return undefined;
    if (Array.isArray(value))
      return Promise.all(value.map((item) => visit(item, key, parent)));
    if (typeof value === "string") {
      if (
        (key === "aboutness" ||
          key === "actorEntityRef" ||
          key === "entityAnchors") &&
        value.startsWith("entity:")
      )
        return address("entity", value);
      const ref = /^([a-z_]+):([0-9a-f-]{36})$/i.exec(value);
      if (ref && kinds.has(ref[1]!)) return address(ref[1]!, ref[2]!);
      if (uuid.test(value)) {
        let kind =
          key === "tags" || key === "tagAnchors" ? "tag" : idKinds[key];
        if (["revisionId", "currentRevisionId"].includes(key))
          kind = parent.memoryId
            ? "memory_revision"
            : parent.schemaId
              ? "cognitive_schema_revision"
              : parent.episodeId
                ? "episode_revision"
                : parent.journalId
                  ? "journal_revision"
                  : undefined;
        if (
          ["value", "id"].includes(key) &&
          typeof parent.kind === "string" &&
          kinds.has(parent.kind)
        )
          kind = parent.kind;
        if (key === "value" && typeof parent.case === "string")
          kind = idKinds[parent.case] ?? kind;
        if (kind)
          return address(
            kind,
            value,
            typeof parent.title === "string"
              ? parent.title
              : typeof parent.purpose === "string"
                ? parent.purpose
                : "",
          );
        return undefined;
      }
      if (/digest|hash|signature/i.test(key) && /^[0-9a-f]{64}$/i.test(value))
        return undefined;
      return value;
    }
    if (!value || typeof value !== "object" || value instanceof Uint8Array)
      return value;
    const object = value as Record<string, unknown>;
    const entries = await Promise.all(
      Object.entries(object).map(
        async ([name, item]) =>
          [name, await visit(item, name, object)] as const,
      ),
    );
    return Object.fromEntries(entries.filter(([, item]) => item !== undefined));
  }
  return visit(input);
}
