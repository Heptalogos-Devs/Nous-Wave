// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { jsonSchema } from "ai";
import { z } from "zod";
import { canonicalDigest } from "../../digest.js";

export function structuredOutputContract<T>(owner: z.ZodType<T>) {
  const providerSchema = z.toJSONSchema(owner, { target: "draft-7" });
  return {
    providerSchema,
    digest: canonicalDigest(providerSchema),
    sdkSchema: jsonSchema<T>(providerSchema, {
      validate(value) {
        const result = owner.safeParse(value);
        return result.success
          ? { success: true, value: result.data }
          : { success: false, error: result.error };
      },
    }),
  };
}
