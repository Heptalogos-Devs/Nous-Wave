// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { z } from "zod";
import {
  gatewaySchema,
  modelSchema,
  executionProfileSchema,
  profileNameSchema,
  executionTimeoutSchema,
  type ExecutionProfile,
} from "./profiles.js";
import { structuredContractForRole } from "./schemas/contracts.js";

import { roleNames, type ModelRole } from "./roles.js";
const rolePolicySchema = z.strictObject({
  routes: z
    .array(profileNameSchema)
    .min(1)
    .max(4)
    .refine(
      (routes) => new Set(routes).size === routes.length,
      "Duplicate execution route",
    ),
  prompt: profileNameSchema.optional(),
  requirement: z.enum(["optional", "required"]).default("optional"),
});
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
      process_timeout_ms: executionTimeoutSchema.default(30000),
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
  execution_profiles: z.record(z.string(), executionProfileSchema).default({}),
  roles: z.partialRecord(z.enum(roleNames), rolePolicySchema).default({}),
};
export const modelConfigurationSchema = z.strictObject(modelConfigurationShape);

/** Cross-profile graph readiness is an owner diagnostic, not a second Catalog validator. */
export function modelRoleProblem(
  role: string,
  binding: ExecutionProfile,
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
  if (
    role === "material_direct_structuring" &&
    !["image_input", "audio_input", "video_input"].some((inputCapability) =>
      model.capabilities.includes(
        inputCapability as "image_input" | "audio_input" | "video_input",
      ),
    )
  )
    return "Direct structuring requires a declared media input capability";
  if (!model.reasoning_levels.includes(binding.reasoning))
    return "Unsupported reasoning level";
  if (
    protocol &&
    (binding.reasoning !== "provider-default" ||
      binding.temperature !== undefined ||
      binding.top_p !== undefined ||
      binding.max_output_tokens !== undefined)
  )
    return "Role does not consume generation parameters";
}
export type ModelConfiguration = z.infer<typeof modelConfigurationSchema>;
export type RolePolicy = z.infer<typeof rolePolicySchema>;
