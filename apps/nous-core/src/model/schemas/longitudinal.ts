import { z } from "zod";

const catalogKeySchema = z.string().min(1).max(256);
const segmentSchema = z.strictObject({
  memberKeys: z.array(catalogKeySchema).min(1).max(256),
  title: z.string().max(8192).nullable(),
  boundaryExplanation: z.string().min(1).max(16384),
});
export const episodePartitionSchema = z.discriminatedUnion("action", [
  z.strictObject({
    action: z.literal("no_change"),
    reason: z.string().min(1).max(8192),
  }),
  z.strictObject({
    action: z.literal("partition"),
    segments: z.array(segmentSchema).min(1).max(16),
  }),
]);
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
  supportKeys: z.array(catalogKeySchema).min(1).max(16),
});
export const journalSynthesisSchema = z.discriminatedUnion("action", [
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
    narrative: z.string().min(1).max(131072),
    points: z.array(journalPointSchema).min(1).max(128),
  }),
]);
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
