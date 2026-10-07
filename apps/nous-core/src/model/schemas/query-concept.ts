// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { z } from "zod";
export const queryConceptSchema = z
  .strictObject({
    existing_tags: z
      .array(
        z.strictObject({
          key: z.string().regex(/^c[0-9]{1,2}$/),
          strength: z.number().finite().min(0).max(1),
        }),
      )
      .max(8),
    novel_concepts: z
      .array(z.strictObject({ text: z.string().trim().min(1).max(512) }))
      .max(4),
  })
  .meta({
    title: "Query concept enrichment",
    description:
      "Ephemeral query activation: supplied catalog keys and bounded concept hypotheses, never durable identifiers or mutations.",
  });
