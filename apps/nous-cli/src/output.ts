// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? Object.fromEntries(Object.entries(value))
    : {};
}
function pick(value: unknown, keys: string[]) {
  const source = record(value);
  return Object.fromEntries(
    keys
      .filter((key) => source[key] !== undefined)
      .map((key) => [key, source[key]]),
  );
}
function contextSummary(value: unknown): unknown {
  const source = record(value);
  if (Array.isArray(source.items))
    return {
      items: source.items.map(contextSummary),
      nextPageToken: source.nextPageToken,
    };
  return pick(source.workContext ?? value, [
    "workContextId",
    "revision",
    "state",
    "purpose",
    "contextText",
    "cognitionAnchors",
    "entityAnchors",
    "tagAnchors",
    "unresolvedQuestions",
    "constraints",
    "resumeConditions",
    "budgetSummary",
  ]);
}
function cognitionSummary(value: unknown) {
  return pick(value, [
    "revisionId",
    "currentRevisionId",
    "memoryId",
    "schemaId",
    "episodeId",
    "journalId",
    "title",
    "text",
    "synopsis",
    "points",
    "boundaryExplanation",
    "cognitiveRole",
    "semanticRole",
    "formationMode",
    "acceptanceState",
    "integrityState",
    "suppressionState",
    "purgeState",
    "basis",
    "evidenceLinks",
    "aboutness",
    "tags",
    "formedAt",
    "recordedAt",
    "validTime",
    "temporalEvidence",
  ]);
}
export function semanticOutput(kind: string, input: unknown) {
  let data: unknown = input;
  const source = record(input);
  if (kind.startsWith("context.") && kind !== "context.foreground")
    data = contextSummary(input);
  else if (kind.startsWith("subject."))
    data = pick(input, ["subjectId", "status", "items", "nextPageToken"]);
  else if (kind.startsWith("session."))
    data = pick(input, [
      "sessionId",
      "runtimeRevision",
      "closed",
      "activeWorkContextId",
      "residentRefs",
    ]);
  else if (kind.startsWith("observe."))
    data = {
      ...pick(input, [
        "occurrenceId",
        "sourceRegionId",
        "memories",
        "degradation",
      ]),
      next: "form <occurrenceId>",
    };
  else if (kind === "show") data = cognitionSummary(input);
  else if (kind === "trace")
    data = { ...source, cognition: cognitionSummary(source.cognition) };
  return { schemaVersion: "nous.cli.v1", kind, data };
}
export function renderText(value: unknown, depth = 0): string {
  if (value === undefined || value === null || value === "") return "";
  if (typeof value === "string") return value;
  if (
    typeof value === "number" ||
    typeof value === "boolean" ||
    typeof value === "bigint"
  )
    return value.toString();
  if (typeof value !== "object") return "";
  if (Array.isArray(value))
    return value
      .map(
        (item, index) =>
          `${"  ".repeat(depth)}${index + 1}. ${renderText(item, depth + 1).trimStart()}`,
      )
      .join("\n");
  if ("schemaVersion" in value && "data" in value)
    return renderText(value.data, depth);
  if (
    "kind" in value &&
    "value" in value &&
    typeof value.kind === "string" &&
    typeof value.value === "string"
  )
    return `${value.kind}:${value.value}`;
  if ("case" in value && "value" in value && typeof value.case === "string")
    return `${value.case}: ${renderText(value.value, depth)}`;
  return Object.entries(value)
    .filter(
      ([key, item]) =>
        !key.startsWith("$") &&
        item !== undefined &&
        item !== "" &&
        (!Array.isArray(item) || item.length),
    )
    .map(([key, item]) =>
      typeof item === "object" && item !== null
        ? `${"  ".repeat(depth)}${key}:\n${renderText(item, depth + 1)}`
        : `${"  ".repeat(depth)}${key}: ${String(item)}`,
    )
    .join("\n");
}
