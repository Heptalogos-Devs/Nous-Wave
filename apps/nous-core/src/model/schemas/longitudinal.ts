import { z } from "zod";

const catalogKeySchema = z
  .string()
  .min(1)
  .max(256)
  .describe("Exact invocation-local key from the supplied source catalog.");
const segmentSchema = z.strictObject({
  memberKeys: z
    .array(catalogKeySchema)
    .min(1)
    .max(256)
    .describe(
      "A contiguous subsequence of the ordered member catalog, preserving its order.",
    ),
  title: z.string().max(8192).nullable(),
  boundaryExplanation: z
    .string()
    .min(1)
    .max(16384)
    .describe("Evidence for the event boundary represented by this segment."),
});
export const episodePartitionSchema = z
  .discriminatedUnion("action", [
    z.strictObject({
      action: z.literal("no_change"),
      reason: z.string().min(1).max(8192),
    }),
    z.strictObject({
      action: z.literal("partition"),
      segments: z.array(segmentSchema).min(1).max(16),
    }),
  ])
  .meta({
    title: "Episode partition",
    description:
      "An unchanged decision or an exact ordered partition of the supplied episode members.",
  });
export type EpisodePartitionProposal = z.infer<typeof episodePartitionSchema>;

export const journalPointSchema = z.strictObject({
  role: z.enum([
    "summary",
    "outcome",
    "change",
    "decision",
    "open_question",
    "salient_event",
    "reflection",
  ]),
  text: z.string().min(1).max(8192),
  supportKeys: z
    .array(catalogKeySchema)
    .min(1)
    .max(16)
    .describe("Distinct exact support catalog keys grounding this point."),
});
export const journalSynthesisSchema = z
  .discriminatedUnion("action", [
    z.strictObject({
      action: z.literal("no_change"),
      reason: z.string().min(1).max(8192),
    }),
    z.strictObject({
      action: z.literal("withdraw"),
      reason: z.string().min(1).max(8192),
    }),
    z.strictObject({
      action: z.literal("commit"),
      title: z.string().max(8192).nullable(),
      narrative: z
        .string()
        .min(1)
        .max(131072)
        .describe(
          "A coherent narrative whose factual claims are covered by the supported points.",
        ),
      points: z.array(journalPointSchema).min(1).max(128),
    }),
  ])
  .meta({
    title: "Journal synthesis",
    description:
      "A supported narrative proposal, an unchanged decision or withdrawal of the current Journal.",
  });
export type JournalSynthesisProposal = z.infer<typeof journalSynthesisSchema>;

export function partitionIndices(
  proposal: EpisodePartitionProposal,
  orderedKeys: readonly string[],
) {
  if (proposal.action === "no_change") return undefined;
  const flattened = proposal.segments.flatMap((segment) => segment.memberKeys);
  if (
    flattened.length !== orderedKeys.length ||
    flattened.some((value, index) => value !== orderedKeys[index])
  )
    throw new Error(
      "Episode proposal must partition the exact ordered member catalog",
    );
  const indices = new Map(orderedKeys.map((value, index) => [value, index]));
  if (indices.size !== orderedKeys.length)
    throw new Error("Episode member catalog contains duplicate keys");
  return proposal.segments.map((segment) => ({
    memberIndices: segment.memberKeys.map((key) => indices.get(key)!),
    title: segment.title ?? undefined,
    boundaryExplanation: segment.boundaryExplanation,
  }));
}

export function journalSupportKeys(
  proposal: JournalSynthesisProposal,
  allowedKeys: ReadonlySet<string>,
) {
  if (proposal.action !== "commit") return;
  for (const point of proposal.points) {
    if (
      new Set(point.supportKeys).size !== point.supportKeys.length ||
      point.supportKeys.some((key) => !allowedKeys.has(key))
    )
      throw new Error(
        "Journal point support must select distinct exact catalog keys",
      );
  }
}
