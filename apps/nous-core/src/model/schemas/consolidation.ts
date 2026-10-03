import { z } from "zod";
const key = z
  .string()
  .min(1)
  .max(256)
  .describe("Exact invocation-local key from the relevant supplied catalog.");
const time = z.discriminatedUnion("kind", [
  z.strictObject({ kind: z.literal("unknown") }),
  z.strictObject({ kind: z.literal("instant"), at: z.iso.datetime() }),
  z.strictObject({
    kind: z.literal("interval"),
    start: z.iso.datetime().nullable(),
    end: z.iso.datetime().nullable(),
  }),
]);
const memory = z.strictObject({
  cognitiveRole: z.enum([
    "experiential",
    "declarative",
    "procedural_experience",
  ]),
  formationMode: z.enum(["grounded", "synthesized"]),
  groundingMemberKey: key.nullable(),
  semanticRole: z.string().min(1).max(128),
  text: z.string().min(1).max(32768),
  title: z.string().max(8192).nullable(),
  supportKeys: z.array(key).min(1).max(128),
  entityKeys: z.array(key).max(128),
  validTime: time.describe(
    "World-valid time of this claim, distinct from publication, observation and formation time.",
  ),
  epistemicClass: z.enum([
    "observed",
    "reported",
    "derived",
    "inferred",
    "narrative",
    "simulated",
  ]),
});
const schema = z.strictObject({
  title: z.string().max(8192).nullable(),
  structuralClaim: z.string().min(1).max(16384),
  applicability: z
    .string()
    .min(1)
    .max(16384)
    .describe(
      "Conditions under which the generalized structural claim applies.",
    ),
  boundaryDefinition: z
    .string()
    .min(1)
    .max(16384)
    .describe("Limits and exceptions of the proposed generalization."),
  formationKind: z.enum(["explicit_import", "synthesized"]),
  entityKeys: z.array(key).max(128),
  validTime: time.describe(
    "World-valid time of this claim, distinct from publication, observation and formation time.",
  ),
  evidence: z
    .array(
      z.strictObject({
        role: z.enum(["support", "counterexample", "boundary_case"]),
        supportKey: key,
      }),
    )
    .min(1)
    .max(128),
});
const intent = z.enum(["correct", "rephrase", "reinterpret"]);
const endpoint = z.discriminatedUnion("kind", [
  z.strictObject({ kind: z.literal("candidate"), key }),
  z.strictObject({
    kind: z.literal("action"),
    index: z.number().int().min(0).max(15),
  }),
]);
export const consolidationActionSchema = z.discriminatedUnion("action", [
  z.strictObject({
    action: z.literal("skip"),
    reason: z.string().min(1).max(8192),
  }),
  z.strictObject({ action: z.literal("create_memory"), content: memory }),
  z.strictObject({
    action: z.literal("revise_memory"),
    targetKey: key,
    intent,
    content: memory,
  }),
  z.strictObject({ action: z.literal("create_schema"), content: schema }),
  z.strictObject({
    action: z.literal("revise_schema"),
    targetKey: key,
    intent,
    content: schema,
  }),
  z.strictObject({
    action: z.literal("link_relation"),
    from: endpoint,
    to: endpoint,
    relation: z.enum([
      "derived_from",
      "contradicts",
      "temporal_successor",
      "replaces_basis",
      "elaborates",
    ]),
  }),
]);
export const consolidationSchema = z
  .strictObject({
    actions: z
      .array(consolidationActionSchema)
      .min(1)
      .max(16)
      .describe(
        "Ordered proposals; action endpoints use zero-based indices in this array.",
      ),
  })
  .meta({
    title: "Memory consolidation",
    description:
      "Source-grounded proposals to create or revise Memory and CognitiveSchema and relate legitimate objects.",
  });
export type ConsolidationProposal = z.infer<typeof consolidationSchema>;
