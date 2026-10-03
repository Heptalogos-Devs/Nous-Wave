import { z } from "zod";
import { modelConfigurationShape } from "./model/configuration.js";
import { resourceProfilesSchema } from "./resources/configuration.js";

export const CONFIG_REVISION = 2;
const requirement = z.enum(["REQUIRED", "PREFERRED", "OPTIONAL", "FORBIDDEN"]);
export const consumersSchema = z
  .array(
    z.strictObject({
      consumer_id: z.string().min(1).max(128),
      revision: z.string().min(1).default("1"),
      memory: requirement.default("PREFERRED"),
      runtime: requirement.default("OPTIONAL"),
      resource: requirement.default("OPTIONAL"),
      max_items: z.number().int().min(1).max(2048).default(32),
      max_text_bytes: z.number().int().min(1).max(1048576).default(32768),
      materialize: z.boolean().default(true),
    }),
  )
  .min(1)
  .max(64)
  .prefault([{ consumer_id: "default" }]);
export const hostSchema = z
  .strictObject({
    port: z.number().int().min(0).max(65535).default(9470),
    dotenv_file: z.string().min(1).default("gateway.env"),
    kernel_executable: z.string().min(1).optional(),
  })
  .prefault({});
export const databaseSchema = z
  .strictObject({
    mode: z.enum(["managed_private", "external"]).default("managed_private"),
    url_env: z
      .string()
      .regex(/^[A-Za-z_][A-Za-z0-9_]*$/)
      .optional(),
    max_connections: z.number().int().min(1).max(256).default(8),
    name: z
      .string()
      .regex(/^[a-z][a-z0-9_]{0,62}$/)
      .default("nous_wave"),
  })
  .prefault({});
export const bootstrapSchema = z.object({
  config_revision: z.literal(CONFIG_REVISION),
  host: hostSchema,
  database: databaseSchema,
});

const owners = [
  {
    path: "config_revision",
    schema: z.literal(CONFIG_REVISION),
    default: CONFIG_REVISION,
    owner: "core-host",
    exposure: "developer",
    deployment: true,
  },
  {
    path: "host",
    schema: hostSchema,
    default: hostSchema.parse(undefined),
    owner: "core-host",
    exposure: "standard",
    deployment: true,
  },
  {
    path: "database",
    schema: databaseSchema,
    default: databaseSchema.parse(undefined),
    owner: "core-host",
    exposure: "advanced",
    deployment: true,
  },
  {
    path: "gateway_profiles",
    schema: modelConfigurationShape.gateway_profiles,
    default: {},
    owner: "core-model",
    exposure: "standard",
  },
  {
    path: "model_profiles",
    schema: modelConfigurationShape.model_profiles,
    default: {},
    owner: "core-model",
    exposure: "standard",
  },
  {
    path: "roles",
    schema: modelConfigurationShape.roles,
    default: {},
    owner: "core-model",
    exposure: "advanced",
  },
  {
    path: "audio",
    schema: modelConfigurationShape.audio,
    default: modelConfigurationShape.audio.parse(undefined),
    owner: "core-model",
    exposure: "advanced",
  },
  {
    path: "video",
    schema: modelConfigurationShape.video,
    default: modelConfigurationShape.video.parse(undefined),
    owner: "core-model",
    exposure: "advanced",
  },
  {
    path: "material.strategy",
    schema: modelConfigurationShape.material_strategy,
    default: "description_only",
    owner: "core-material",
    exposure: "standard",
  },
  {
    path: "resource_profiles",
    schema: resourceProfilesSchema,
    default: {},
    owner: "core-resource",
    exposure: "advanced",
  },
  {
    path: "consumers",
    schema: consumersSchema,
    default: consumersSchema.parse(undefined),
    owner: "core-consumer",
    exposure: "advanced",
  },
] satisfies {
  path: string;
  schema: z.ZodType;
  default: unknown;
  owner: string;
  exposure: string;
  deployment?: boolean;
}[];

/** Owner Zod schemas are the sole TypeScript type/JSON Schema source. */
export function coreDescriptors() {
  return owners.map((owner) => ({
    path: owner.path,
    owner: owner.owner,
    title: owner.path,
    description: `Configuration for ${owner.path}.`,
    category: owner.owner,
    json_schema: z.toJSONSchema(owner.schema, {
      target: "draft-2020-12",
      io: "input",
    }),
    reference_default: owner.default,
    exposure: owner.exposure,
    scope_policy: "system_only",
    storage_policy: "deployment" in owner ? "deployment_only" : "overrideable",
    apply_mode: "restart_process",
    semantic_effect: "operational",
    unit: null,
    sensitivity: ["gateway_profiles", "resource_profiles", "database"].includes(
      owner.path,
    )
      ? "credential_reference"
      : "normal",
    reference_profile: null,
  }));
}
export function configurationBundle(deploymentDocument: unknown) {
  return {
    bundle_revision: 1,
    core_descriptors: coreDescriptors(),
    deployment_document: deploymentDocument,
  };
}
