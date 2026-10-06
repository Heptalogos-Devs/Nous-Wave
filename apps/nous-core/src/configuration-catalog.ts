// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

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
const executionTimeout = z.number().int().min(1).max(600000);
const hostFields = {
  port: z.number().int().min(0).max(65535).default(9470),
  dotenv_file: z.string().min(1).default("gateway.env"),
  kernel_executable: z.string().min(1).nullable().default(null),
  kernel_startup_timeout_ms: executionTimeout.default(120000),
  kernel_shutdown_timeout_ms: executionTimeout.default(5000),
  runtime_download_timeout_ms: executionTimeout.default(300000),
};
export const hostSchema = z.strictObject(hostFields).prefault({});
export const coreExecutionSchema = z
  .strictObject({
    kernel_rpc_timeout_ms: executionTimeout.default(30000),
    maintenance_rpc_timeout_ms: executionTimeout.default(10000),
    workflow_ack_timeout_ms: executionTimeout.default(5000),
    context_track_limit: z.number().int().min(1).max(65536).default(256),
    query_embedding_cache_entries: z
      .number()
      .int()
      .min(1)
      .max(65536)
      .default(128),
    query_rerank_candidate_limit: z.number().int().min(2).max(64).default(64),
    public_rpc_response_max_bytes: z
      .number()
      .int()
      .min(1024)
      .max(33554432)
      .default(4194304),
    http_body_limit_bytes: z
      .number()
      .int()
      .min(1024)
      .max(8388608)
      .default(2097152),
  })
  .prefault({});
export type CoreExecutionPolicy = z.infer<typeof coreExecutionSchema>;

const databaseSchema = z
  .strictObject({
    mode: z.enum(["managed_private", "external"]).default("managed_private"),
    url_env: z
      .string()
      .regex(/^[A-Za-z_][A-Za-z0-9_]*$/)
      .optional(),
    max_connections: z.number().int().min(1).max(256).default(8),
    acquire_timeout_ms: executionTimeout.default(15000),
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
  ...Object.entries(hostFields).map(([name, schema]) => ({
    path: `host.${name}`,
    schema,
    default: schema.parse(undefined),
    owner: "core-host",
    exposure:
      name === "port" || name === "dotenv_file" ? "standard" : "developer",
    deployment: true,
  })),
  {
    path: "core_execution",
    schema: coreExecutionSchema,
    default: coreExecutionSchema.parse(undefined),
    owner: "core-execution",
    exposure: "developer",
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
    path: "material.inputs",
    schema: modelConfigurationShape.material_inputs,
    default: modelConfigurationShape.material_inputs.parse(undefined),
    owner: "core-material",
    exposure: "developer",
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
function coreDescriptors() {
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
