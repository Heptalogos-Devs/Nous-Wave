// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";
import { remoteEndpointSchema } from "../remote-endpoint.js";
import { structuredContractForRole } from "./schemas/contracts.js";

import { roleNames, type ModelRole } from "./roles.js";
export { roleNames, type ModelRole } from "./roles.js";
const nonempty = z.string().min(1).max(512);
const boundedTimeout = z.number().int().min(1).max(300_000);
const gatewaySchema = z.strictObject({
  base_url: remoteEndpointSchema,
  credential_env: z.string().regex(/^[A-Za-z_][A-Za-z0-9_]*$/),
  enabled: z.boolean().default(true),
  request_timeout_ms: boundedTimeout.default(30_000),
});
const embeddingSchema = z.strictObject({
  dimension: z.number().int().min(1).max(8192),
  max_batch_size: z.number().int().min(1).max(64).default(64),
  weights_revision: nonempty,
  task: nonempty,
  input_representation: nonempty,
  preprocessing_identity: nonempty,
  preprocessing_revision: nonempty,
  normalization: nonempty,
  output_semantics: nonempty,
});
const modelBaseSchema = z.strictObject({
  gateway: nonempty,
  model: z.string().max(512).default(""),
  capabilities: z
    .array(
      z.enum([
        "text",
        "image_input",
        "audio_input",
        "video_input",
        "structured_output",
        "embedding",
        "speech_transcription",
        "rerank",
      ]),
    )
    .min(1),
  model_revision: nonempty.optional(),
});
// The protocol/embedding contract is native Zod, including the exported JSON Schema.
const modelSchema = z.discriminatedUnion("protocol", [
  modelBaseSchema.extend({
    protocol: z.literal("openai-embeddings"),
    embedding: embeddingSchema,
  }),
  modelBaseSchema.extend({
    protocol: z.enum([
      "openai-chat",
      "openai-responses",
      "openai-audio-transcription",
      "rerank-v1",
    ]),
    embedding: z.never().optional(),
  }),
]);
const generationTokensSchema = z
  .number()
  .int()
  .min(1)
  .max(65_536)
  .default(4096);
const bindingSchema = z.strictObject({
  model: nonempty,
  prompt: nonempty.optional(),
  temperature: z.number().min(0).max(2).optional(),
  top_p: z.number().min(0).max(1).optional(),
  max_output_tokens: generationTokensSchema
    .unwrap()
    .optional()
    .meta({ default: generationTokensSchema.parse(undefined) }),
  timeout_ms: boundedTimeout.optional(),
  requirement: z
    .enum(["optional", "preferred", "required"])
    .default("optional"),
});
const generationBindingSchema = bindingSchema.extend({
  max_output_tokens: generationTokensSchema,
});
/** Apply per-role schema defaults only to generation protocols. */
export function resolveRoleBinding(
  binding: z.infer<typeof bindingSchema>,
  protocol: string,
) {
  return ["openai-chat", "openai-responses"].includes(protocol)
    ? generationBindingSchema.parse(binding)
    : binding;
}

export const modelConfigurationShape = {
  material_inputs: z
    .strictObject({
      formation_source_max_bytes: z
        .number()
        .int()
        .min(1)
        .max(1048576)
        .default(32768),
      derivation_source_max_bytes: z
        .number()
        .int()
        .min(1)
        .max(1048576)
        .default(1048576),
    })
    .prefault({}),
  video: z
    .strictObject({
      input_mode: z.enum(["direct", "frames"]).default("direct"),
      ffmpeg_executable: z.string().min(1).optional(),
      max_source_bytes: z
        .number()
        .int()
        .min(1024)
        .max(67108864)
        .default(16777216),
      max_video_seconds: z.number().positive().max(3600).default(60),
      max_frames: z.number().int().min(1).max(16).default(8),
      frame_end_margin_seconds: z.number().min(0).max(1).default(0.1),
      max_frame_bytes: z.number().int().min(1024).max(1048576).default(262144),
      max_audio_bytes: z.number().int().min(44).max(16777216).default(4194304),
      process_timeout_ms: boundedTimeout.default(30000),
      prompt: z.string().min(1).default("material/video-description.md"),
    })
    .prefault({}),
  audio: z
    .strictObject({
      input_mode: z.enum(["direct", "transcription"]).default("direct"),
      max_source_bytes: z
        .number()
        .int()
        .min(1024)
        .max(25165824)
        .default(16777216),
    })
    .prefault({}),
  material_strategy: z
    .enum(["description_only", "direct_structured", "describe_then_structure"])
    .default("description_only"),
  gateway_profiles: z.record(z.string(), gatewaySchema).default({}),
  model_profiles: z.record(z.string(), modelSchema).default({}),
  roles: z.partialRecord(z.enum(roleNames), bindingSchema).default({}),
};
export const modelConfigurationSchema = z.strictObject(modelConfigurationShape);

/** Cross-profile graph readiness is an owner diagnostic, not a second Catalog validator. */
export function modelRoleProblem(
  role: string,
  binding: z.infer<typeof bindingSchema>,
  model: z.infer<typeof modelSchema>,
): string | undefined {
  const protocol =
    role === "query_embedding"
      ? "openai-embeddings"
      : role === "query_rerank"
        ? "rerank-v1"
        : role === "speech_transcription"
          ? "openai-audio-transcription"
          : undefined;
  const capability =
    role === "query_embedding"
      ? "embedding"
      : role === "query_rerank"
        ? "rerank"
        : role === "speech_transcription"
          ? "speech_transcription"
          : "text";
  if (
    (protocol
      ? model.protocol !== protocol
      : !["openai-chat", "openai-responses"].includes(model.protocol)) ||
    !model.capabilities.includes(capability)
  )
    return "Role protocol/capability mismatch";
  if (
    structuredContractForRole(role as ModelRole) &&
    !model.capabilities.includes("structured_output")
  )
    return "Role requires structured_output";
  if (protocol && binding.prompt)
    return "Role does not consume a system prompt";
  if (
    role === "material_direct_structuring" &&
    (!model.capabilities.includes("image_input") ||
      !model.capabilities.includes("structured_output"))
  )
    return "Direct structuring requires image_input and structured_output";
  if (
    protocol &&
    (binding.temperature !== undefined ||
      binding.top_p !== undefined ||
      binding.max_output_tokens !== undefined)
  )
    return "Role does not consume generation parameters";
}
export type ModelConfiguration = z.infer<typeof modelConfigurationSchema>;
export type ModelProfile = z.infer<typeof modelSchema>;
export type RoleBinding = z.infer<typeof bindingSchema>;
