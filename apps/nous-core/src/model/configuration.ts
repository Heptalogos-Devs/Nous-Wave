import { z } from "zod";
import { remoteEndpointSchema } from "../remote-endpoint.js";

export const roleNames = [
  "projection_steward",
  "memory_formation",
  "episode_segmentation",
  "journal_synthesis",
  "memory_consolidation",
  "material_description",
  "material_structuring",
  "material_direct_structuring",
  "query_embedding",
  "query_rerank",
  "speech_transcription",
] as const;
export type ModelRole = (typeof roleNames)[number];
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
const modelSchema = z
  .strictObject({
    gateway: nonempty,
    protocol: z.enum([
      "openai-chat",
      "openai-responses",
      "openai-embeddings",
      "openai-audio-transcription",
      "rerank-v1",
    ]),
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
    embedding: embeddingSchema.optional(),
  })
  .superRefine((model, ctx) => {
    if ((model.protocol === "openai-embeddings") !== Boolean(model.embedding))
      ctx.addIssue({
        code: "custom",
        message:
          "Embedding protocol requires an explicit space profile; other protocols cannot declare one",
      });
  });
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
export const modelConfigurationSchema = z
  .strictObject(modelConfigurationShape)
  .superRefine((config, ctx) => {
    for (const [role, binding] of Object.entries(config.roles)) {
      const model = config.model_profiles[binding.model];
      if (!model) continue;
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
        ctx.addIssue({
          code: "custom",
          message: `Protocol/capability mismatch for role ${role}`,
        });
      if (
        [
          "episode_segmentation",
          "journal_synthesis",
          "memory_consolidation",
        ].includes(role) &&
        !model.capabilities.includes("structured_output")
      )
        ctx.addIssue({
          code: "custom",
          message: `Role ${role} requires structured_output`,
        });
      if (protocol && binding.prompt)
        ctx.addIssue({
          code: "custom",
          message: `Role ${role} does not consume a system prompt`,
        });
      if (
        role === "material_direct_structuring" &&
        (!model.capabilities.includes("image_input") ||
          !model.capabilities.includes("structured_output"))
      )
        ctx.addIssue({
          code: "custom",
          message:
            "Direct structuring requires image_input and structured_output",
        });
      if (
        protocol &&
        (binding.temperature !== undefined ||
          binding.top_p !== undefined ||
          binding.max_output_tokens !== undefined)
      )
        ctx.addIssue({
          code: "custom",
          message: `Role ${role} does not consume generation parameters`,
        });
    }
  });
export type ModelConfiguration = z.infer<typeof modelConfigurationSchema>;
export type ModelProfile = z.infer<typeof modelSchema>;
export type RoleBinding = z.infer<typeof bindingSchema>;
