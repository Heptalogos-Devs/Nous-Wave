import { z } from "zod";

export const projectionStewardSchema = z
  .strictObject({
    selectedIds: z
      .array(z.string())
      .max(64)
      .describe("Distinct IDs selected from the supplied segment catalog."),
    summary: z
      .string()
      .max(8192)
      .nullable()
      .default(null)
      .describe(
        "Optional synthesis supported by the selected segments, or null to retain the selected segments directly.",
      ),
  })
  .meta({
    title: "Projection stewardship",
    description:
      "Selection and optional synthesis of an already policy-filtered segment catalog.",
  });
