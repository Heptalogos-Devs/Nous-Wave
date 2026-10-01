import { z } from "zod";
import { remoteEndpointSchema } from "../remote-endpoint.js";

export const resourceProfilesSchema = z
  .record(
    z.string().regex(/^[a-z][a-z0-9_.-]{0,127}$/),
    z.strictObject({
      adapter_kind: z.literal("ragflow"),
      base_url: remoteEndpointSchema,
      credential_env: z.string().regex(/^[A-Za-z_][A-Za-z0-9_]*$/),
      enabled: z.boolean().default(true),
      timeout_ms: z.number().int().min(1).max(300000).default(30000),
      max_material_bytes: z.number().int().min(1).max(1048576).default(262144),
    }),
  )
  .default({});
export type ResourceProfiles = z.infer<typeof resourceProfilesSchema>;
