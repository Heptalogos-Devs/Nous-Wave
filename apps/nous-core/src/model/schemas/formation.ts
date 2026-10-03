import { z } from "zod";

export const formationSchema = z
  .strictObject({
    text: z
      .string()
      .min(1)
      .max(32768)
      .describe("Source-grounded content to retain as one Memory."),
    semanticRole: z
      .string()
      .min(1)
      .max(128)
      .describe(
        "A short semantic label, at most 128 characters, such as reported_fact or working_practice.",
      ),
    title: z
      .string()
      .max(256)
      .nullable()
      .default(null)
      .describe(
        "A short descriptive title, or null when no useful title is needed.",
      ),
    selectedEntityKeys: z
      .array(z.string().max(128))
      .max(128)
      .default([])
      .describe(
        "Invocation-local keys selecting only resolved entities that this Memory is about.",
      ),
  })
  .meta({
    title: "Memory formation",
    description:
      "A source-grounded Memory proposal and its selection from the supplied entity catalog.",
  });
