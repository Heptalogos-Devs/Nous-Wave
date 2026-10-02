import { z } from "zod";
import { structuredOutputContract } from "./provider.js";
import type { JsonObject } from "@bufbuild/protobuf";

const coverage = z.enum(["not_available", "observed", "limited", "uncertain"]);
const supports = z.array(z.string());
const time = {
  start_ms: z.number().int().nullable(),
  end_ms: z.number().int().nullable(),
};
export const materialInterpretationSchema = z.strictObject({
  summary: z.string(),
  coverage: z.strictObject({
    visual: coverage,
    audio: coverage,
    embedded_text: coverage,
  }),
  observations: z.array(
    z.strictObject({
      kind: z.enum([
        "object",
        "action",
        "state",
        "scene",
        "spatial_relation",
        "sound",
        "music",
        "text",
        "speech",
        "event",
        "other",
      ]),
      content: z.string(),
      basis: z.enum(["direct", "inferred", "uncertain"]),
      ...time,
      support_keys: supports,
    }),
  ),
  mentions: z.array(
    z.strictObject({
      surface: z.string(),
      category: z.enum([
        "person",
        "organization",
        "place",
        "object",
        "concept",
        "work",
        "other",
      ]),
      role: z.string().nullable(),
      support_keys: supports,
    }),
  ),
  embedded_text: z.array(
    z.strictObject({
      text: z.string(),
      fidelity: z.enum(["verbatim", "approximate", "uncertain"]),
      ...time,
      support_keys: supports,
    }),
  ),
  speech: z.array(
    z.strictObject({
      text: z.string(),
      fidelity: z.enum(["verbatim", "semantic", "uncertain"]),
      speaker_hint: z.string().nullable(),
      ...time,
      support_keys: supports,
    }),
  ),
  interpretations: z.array(
    z.strictObject({
      content: z.string(),
      status: z.enum(["supported", "tentative"]),
      support_keys: supports,
    }),
  ),
  uncertainties: z.array(
    z.strictObject({
      issue: z.string(),
      alternatives: z.array(z.string()),
      support_keys: supports,
    }),
  ),
});
export type MaterialInterpretation = z.infer<
  typeof materialInterpretationSchema
>;
const contract = structuredOutputContract(materialInterpretationSchema);
export const materialInterpretationSchemaDigest = contract.digest;
export const materialProjectionIdentity = "material-interpretation-text-v1";
type InterpretationInput = {
  visual: boolean;
  audio: boolean;
  supportKeys: ReadonlySet<string>;
  durationMs?: number;
};
export type StructuredMaterialContext = Omit<
  InterpretationInput,
  "supportKeys"
> & {
  catalog: Readonly<Record<string, { kind: string; value: string }>>;
};
export function structuredMaterialResult(
  value: unknown,
  context: StructuredMaterialContext,
) {
  const output = validateMaterialInterpretation(value, {
    ...context,
    supportKeys: new Set(Object.keys(context.catalog)),
  });
  const payload: Record<string, unknown> = { ...output };
  for (const group of [
    "observations",
    "mentions",
    "embedded_text",
    "speech",
    "interpretations",
    "uncertainties",
  ] as const)
    payload[group] = output[group].map(({ support_keys, ...item }) => ({
      ...item,
      supports: support_keys.map((key) => context.catalog[key]!),
    }));
  return {
    text: projectMaterialInterpretation(output),
    structuredPayload: JSON.parse(JSON.stringify(payload)) as JsonObject,
  };
}

// Provider schema stays conservative; the Material owner enforces these local bounds.
function validateMaterialInterpretation(
  value: unknown,
  input: InterpretationInput,
): MaterialInterpretation {
  const output = materialInterpretationSchema.parse(value);
  if (
    Buffer.byteLength(JSON.stringify(output)) > 262144 ||
    Buffer.byteLength(output.summary) > 8192
  )
    throw new Error("Structured material exceeds payload bounds");
  const groups = [
    output.observations,
    output.mentions,
    output.embedded_text,
    output.speech,
    output.interpretations,
    output.uncertainties,
  ];
  for (const group of groups) {
    if (group.length > 64)
      throw new Error("Structured material exceeds item bound");
    for (const item of group) {
      for (const field of Object.values(item)) {
        if (typeof field === "string" && Buffer.byteLength(field) > 4096)
          throw new Error("Structured material exceeds field byte bound");
      }
      if (
        item.support_keys.length > 16 ||
        new Set(item.support_keys).size !== item.support_keys.length
      )
        throw new Error("Structured material support count invalid");
      for (const key of item.support_keys)
        if (!input.supportKeys.has(key))
          throw new Error("Unknown structured material support key");
      if (
        "basis" in item &&
        item.basis === "direct" &&
        !item.support_keys.length
      )
        throw new Error("Direct observation requires source support");
      if ("start_ms" in item) {
        const { start_ms: start, end_ms: end } = item;
        for (const timestamp of [start, end])
          if (
            timestamp !== null &&
            (!Number.isSafeInteger(timestamp) ||
              timestamp < 0 ||
              (input.durationMs !== undefined && timestamp > input.durationMs))
          )
            throw new Error(
              "Structured material timestamp out of source bounds",
            );
        if (start !== null && end !== null && start > end)
          throw new Error("Structured material timestamp order invalid");
      }
      if (
        "alternatives" in item &&
        (item.alternatives.length > 8 ||
          item.alternatives.some((entry) => Buffer.byteLength(entry) > 4096))
      )
        throw new Error("Structured material alternatives exceed bounds");
    }
  }
  if (
    !input.visual &&
    (output.coverage.visual !== "not_available" ||
      output.embedded_text.length ||
      output.observations.some(
        (item) =>
          item.basis === "direct" &&
          ["object", "action", "state", "scene", "spatial_relation"].includes(
            item.kind,
          ),
      ))
  )
    throw new Error("Structured material invents unavailable visual input");
  if (
    !input.audio &&
    (output.coverage.audio !== "not_available" ||
      output.speech.length ||
      output.observations.some((item) =>
        ["sound", "music", "speech"].includes(item.kind),
      ))
  )
    throw new Error("Structured material invents unavailable audio input");
  return output;
}

function projectMaterialInterpretation(output: MaterialInterpretation): string {
  const section = (title: string, lines: string[]) =>
    lines.length ? [`${title}:`, ...lines.map((line) => `- ${line}`)] : [];
  return [
    `Summary: ${output.summary}`,
    `Coverage: visual=${output.coverage.visual}; audio=${output.coverage.audio}; embedded_text=${output.coverage.embedded_text}`,
    ...section(
      "Observations",
      output.observations.map(
        (item) => `[${item.basis}/${item.kind}] ${item.content}`,
      ),
    ),
    ...section(
      "Mentions",
      output.mentions.map(
        (item) =>
          `${item.surface} [${item.category}]${item.role === null ? "" : ` (${item.role})`}`,
      ),
    ),
    ...section(
      "Embedded text",
      output.embedded_text.map((item) => `[${item.fidelity}] ${item.text}`),
    ),
    ...section(
      "Speech",
      output.speech.map(
        (item) =>
          `[${item.fidelity}] ${item.speaker_hint ?? "unknown speaker"}: ${item.text}`,
      ),
    ),
    ...section(
      "Interpretations",
      output.interpretations.map((item) => `[${item.status}] ${item.content}`),
    ),
    ...section(
      "Uncertainties",
      output.uncertainties.map(
        (item) =>
          `${item.issue}${item.alternatives.length ? `; alternatives: ${item.alternatives.join(" | ")}` : ""}`,
      ),
    ),
  ].join("\n");
}
