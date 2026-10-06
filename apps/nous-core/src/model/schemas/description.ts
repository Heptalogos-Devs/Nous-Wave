// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";

// Local bound on free text; this is not sent as a Structured Output contract.
export const descriptionSchema = z.strictObject({
  text: z.string().min(1).max(65536),
});
