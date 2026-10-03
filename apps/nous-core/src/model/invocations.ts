import { createOpenAI } from "@ai-sdk/openai";
import {
  embedMany,
  generateText,
  Output,
  NoObjectGeneratedError,
  APICallError,
  transcribe,
  type UserContent,
} from "ai";
import { z } from "zod";
import { canonicalDigest } from "../digest.js";
import {
  providerContractForRole,
  type ModelGenerationOutput,
} from "./schemas/contracts.js";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  roleNames,
  resolveRoleBinding,
  modelRoleProblem,
  type ModelConfiguration,
  type ModelProfile,
  type ModelRole,
  type RoleBinding,
  modelConfigurationSchema,
} from "./configuration.js";
import { PromptRegistry, type PromptAsset } from "./prompts.js";
import { modelRoleIdentity } from "./identity.js";

class MediaProtocolError extends Error {}
class ModelOutputError extends Error {}
export class GenerationFailure extends Error {
  constructor(role: ModelRole, reason: string) {
    super(`Model role ${role} invocation failed: ${reason}`);
  }
}

function safeGenerationFailure(error: unknown) {
  if (error instanceof ModelOutputError) return error.message;
  if (NoObjectGeneratedError.isInstance(error))
    return error.finishReason && error.finishReason !== "stop"
      ? `output_incomplete_${error.finishReason}`
      : "output_schema_invalid";
  if (error instanceof z.ZodError) return "output_schema_invalid";
  if (error instanceof SyntaxError) return "output_json_invalid";
  if (
    APICallError.isInstance(error) &&
    error.statusCode &&
    Number.isInteger(error.statusCode) &&
    error.statusCode >= 100 &&
    error.statusCode <= 599
  )
    return `gateway_http_${error.statusCode}`;
  if (
    error instanceof Error &&
    ["AbortError", "TimeoutError"].includes(error.name)
  )
    return "timeout_or_cancellation";
  return "validation_or_transport";
}

export type ModelProducerMetadata = {
  implementation: string;
  protocol: string;
  model: string;
  modelRevision?: string;
  profileDigest: string;
  promptId?: string;
  promptDigest?: string;
  outputSchemaDigest?: string;
  configDigest: string;
};
type ReadyRole = {
  name: ModelRole;
  profile: ModelProfile;
  binding: RoleBinding;
  prompt?: PromptAsset;
  provider: ReturnType<typeof createOpenAI>;
  baseURL: string;
  credential: string;
  credentialEnv: string;
  timeout: number;
  profileDigest: string;
  configDigest: string;
};
const snapshotSchema = z.strictObject({
  configuration: modelConfigurationSchema,
  role: z.enum(roleNames),
  prompt: z
    .strictObject({
      id: z.string().max(1024),
      digest: z.string().regex(/^[a-f0-9]{64}$/),
      text: z.string().max(131072),
    })
    .optional(),
  profileDigest: z.string().regex(/^[a-f0-9]{64}$/),
  configDigest: z.string().regex(/^[a-f0-9]{64}$/),
});
export type ModelRoleSnapshot = z.infer<typeof snapshotSchema>;
const rerankResponse = z.object({
  results: z
    .array(
      z.object({
        index: z.number().int().nonnegative(),
        relevance_score: z.number().finite(),
      }),
    )
    .max(64),
  usage: z.unknown().optional(),
});
function parseRerankResponse(input: unknown, count: number) {
  const result = rerankResponse.parse(input);
  const indices = new Set<number>();
  for (const item of result.results) {
    if (item.index >= count || indices.has(item.index))
      throw new Error("Invalid rerank candidate index");
    indices.add(item.index);
  }
  return result;
}
export class ModelInvocations {
  private prompts?: PromptRegistry;
  private promptPaths: Partial<Record<ModelRole, string>> = {};
  private readonly active = new Map<ModelRole, ReadyRole>();
  private readonly credentialOrigins = new Set<string>();
  private readonly requirements = new Map<
    ModelRole,
    RoleBinding["requirement"]
  >();
  private readonly states = new Map<
    ModelRole,
    { state: string; detail: string }
  >();
  static async create(
    config: ModelConfiguration,
    promptRoot = resolve(
      dirname(fileURLToPath(import.meta.url)),
      "../../../../prompts",
    ),
    overridePromptRoot?: string,
  ) {
    const runtime = new ModelInvocations();
    for (const gateway of Object.values(config.gateway_profiles))
      if (gateway.enabled)
        runtime.credentialOrigins.add(
          `${gateway.credential_env}@${new URL(gateway.base_url).origin}`,
        );
    const prompts = new PromptRegistry(promptRoot, overridePromptRoot);
    runtime.prompts = prompts;
    for (const role of roleNames) {
      const path = config.roles[role]?.prompt;
      if (path) runtime.promptPaths[role] = path;
    }
    for (const role of roleNames) {
      const configuredBinding = config.roles[role];
      const binding = configuredBinding
        ? resolveRoleBinding(
            configuredBinding,
            config.model_profiles[configuredBinding.model]?.protocol ?? "",
          )
        : undefined;
      runtime.requirements.set(role, binding?.requirement ?? "optional");
      if (!binding) {
        runtime.states.set(role, {
          state: "NOT_CONFIGURED",
          detail: "Role has no binding",
        });
        continue;
      }
      const profile = config.model_profiles[binding.model]!;
      if (!profile || !profile.model.trim()) {
        runtime.states.set(role, {
          state: "NOT_CONFIGURED",
          detail: "Model identifier is unset",
        });
        continue;
      }
      const problem = modelRoleProblem(role, configuredBinding!, profile);
      if (problem) {
        runtime.states.set(role, { state: "UNAVAILABLE", detail: problem });
        continue;
      }
      const gateway = config.gateway_profiles[profile.gateway]!;
      if (!gateway) {
        runtime.states.set(role, {
          state: "NOT_CONFIGURED",
          detail: "Gateway profile is absent",
        });
        continue;
      }
      const credential = process.env[gateway.credential_env];
      if (!gateway.enabled || !credential) {
        runtime.states.set(role, {
          state: "UNAVAILABLE",
          detail: gateway.enabled
            ? "Credential environment is unavailable"
            : "Gateway disabled",
        });
        continue;
      }
      try {
        const prompt = await prompts.load(role, binding.prompt);
        const { profileDigest, configDigest } = modelRoleIdentity(
          profile,
          gateway,
          binding,
          prompt,
        );
        const provider = createOpenAI({
          baseURL: gateway.base_url,
          apiKey: credential,
          name: "standard-gateway",
          fetch: (input, init) => fetch(input, { ...init, redirect: "error" }),
        });
        runtime.active.set(role, {
          name: role,
          profile,
          binding,
          prompt,
          provider,
          credential,
          credentialEnv: gateway.credential_env,
          baseURL: gateway.base_url,
          timeout: binding.timeout_ms ?? gateway.request_timeout_ms,
          profileDigest,
          configDigest,
        });
        runtime.states.set(role, {
          state: "READY",
          detail: "Executable model configuration available",
        });
      } catch {
        runtime.states.set(role, {
          state: "UNAVAILABLE",
          detail: "Prompt asset unavailable or invalid",
        });
      }
    }
    return runtime;
  }
  get capabilities() {
    return roleNames.map((role) => ({
      name: `model.${role}`,
      ...(this.states.get(role) ?? {
        state: "NOT_CONFIGURED",
        detail: "Role has no binding",
      }),
    }));
  }
  profile(role: ModelRole) {
    return this.active.get(role)?.profile;
  }
  requirement(role: ModelRole) {
    return this.requirements.get(role) ?? "optional";
  }
  snapshot(name: ModelRole): ModelRoleSnapshot {
    const role = this.require(name);
    return snapshotSchema.parse({
      role: name,
      configuration: {
        gateway_profiles: {
          [role.profile.gateway]: {
            base_url: role.baseURL,
            credential_env: role.credentialEnv,
            enabled: true,
            request_timeout_ms: role.timeout,
          },
        },
        model_profiles: { [role.binding.model]: role.profile },
        roles: { [name]: role.binding },
      },
      prompt: role.prompt,
      profileDigest: role.profileDigest,
      configDigest: role.configDigest,
    });
  }
  async snapshotPrompt(
    name: ModelRole,
    path?: string,
    adapter?: string,
  ): Promise<ModelRoleSnapshot> {
    const snapshot = this.snapshot(name);
    if (path) {
      const prompt = await this.prompts?.load(name, path);
      if (!prompt) throw new Error("Strategy prompt unavailable");
      snapshot.prompt = prompt;
      snapshot.configDigest = canonicalDigest({
        binding: snapshot.configDigest,
        promptId: prompt.id,
        promptDigest: prompt.digest,
      });
    }
    if (adapter)
      snapshot.configDigest = canonicalDigest({
        binding: snapshot.configDigest,
        adapter,
      });
    return snapshot;
  }
  private hydrate(input: ModelRoleSnapshot): ReadyRole {
    const snapshot = snapshotSchema.parse(input),
      binding = snapshot.configuration.roles[snapshot.role]!;
    const profile = snapshot.configuration.model_profiles[binding.model]!,
      gateway = snapshot.configuration.gateway_profiles[profile.gateway]!;
    const credential = process.env[gateway.credential_env];
    if (
      !this.credentialOrigins.has(
        `${gateway.credential_env}@${new URL(gateway.base_url).origin}`,
      )
    )
      throw new Error(
        "Reserved gateway credential destination is no longer authorized by current configuration",
      );
    if (!credential)
      throw new Error("Reserved model credential reference unavailable");
    return {
      name: snapshot.role,
      profile,
      binding,
      prompt: snapshot.prompt,
      baseURL: gateway.base_url,
      credential,
      credentialEnv: gateway.credential_env,
      timeout: gateway.request_timeout_ms,
      profileDigest: snapshot.profileDigest,
      configDigest: snapshot.configDigest,
      provider: createOpenAI({
        baseURL: gateway.base_url,
        apiKey: credential,
        name: "standard-gateway",
        fetch: (request, options) =>
          fetch(request, { ...options, redirect: "error" }),
      }),
    };
  }
  identity(role: ModelRole) {
    return this.active.get(role)?.configDigest;
  }
  private require(role: ModelRole): ReadyRole {
    const value = this.active.get(role);
    if (!value) throw new Error(`Model role ${role} unavailable`);
    return value;
  }
  private metadata(role: ReadyRole): ModelProducerMetadata {
    return {
      implementation: "ai-sdk@7.0.102/openai@4.0.67",
      protocol: role.profile.protocol,
      model: role.profile.model,
      modelRevision: role.profile.model_revision,
      profileDigest: role.profileDigest,
      promptId: role.prompt?.id,
      promptDigest: role.prompt?.digest,
      configDigest: role.configDigest,
    };
  }
  private signal(role: ReadyRole, signal?: AbortSignal) {
    return signal
      ? AbortSignal.any([signal, AbortSignal.timeout(role.timeout)])
      : AbortSignal.timeout(role.timeout);
  }
  async generate<R extends ModelRole>(
    name: R,
    content: UserContent,
    signal?: AbortSignal,
    promptRole?: ModelRole | { role: ModelRole; path: string },
    fixed?: ModelRoleSnapshot,
    media?: { bytes: Uint8Array; mediaType: string },
  ) {
    let role = fixed ? this.hydrate(fixed) : this.require(name);
    if (role.name !== name) throw new Error("Reserved model role mismatch");
    if (promptRole) {
      const selectedRole =
        typeof promptRole === "string" ? promptRole : promptRole.role;
      const prompt = await this.prompts?.load(
        selectedRole,
        typeof promptRole === "string"
          ? this.promptPaths[selectedRole]
          : promptRole.path,
      );
      if (!prompt) throw new Error("Strategy prompt unavailable");
      role = {
        ...role,
        prompt,
        configDigest: canonicalDigest({
          binding: role.configDigest,
          promptId: prompt.id,
          promptDigest: prompt.digest,
        }),
      };
    }
    if (
      role.profile.protocol !== "openai-chat" &&
      role.profile.protocol !== "openai-responses"
    )
      throw new Error("Role is not a text generation protocol");
    const output = providerContractForRole(name);
    const schema = output?.owner;
    if (output && !role.profile.capabilities.includes("structured_output"))
      throw new Error("Strict structured output capability is unavailable");
    let mediaStage = "admission";
    try {
      if (media) {
        const audio = media.mediaType.startsWith("audio/");
        const formats: Record<string, string> = {
          "audio/mpeg": "mp3",
          "audio/mp3": "mp3",
          "audio/wav": "wav",
          "audio/x-wav": "wav",
          "audio/aac": "aac",
          "audio/mp4": "m4a",
        };
        if (
          role.profile.protocol !== "openai-chat" ||
          !role.profile.capabilities.includes(
            audio ? "audio_input" : "video_input",
          ) ||
          (!audio && !media.mediaType.startsWith("video/")) ||
          (audio && !formats[media.mediaType])
        )
          throw new Error(
            "Direct media protocol/capability or format unavailable",
          );
        if (
          !media.bytes.length ||
          media.bytes.length > (audio ? 25165824 : 67108864)
        )
          throw new Error("Direct media exceeds input bounds");
        const encoded = Buffer.from(media.bytes).toString("base64");
        const part = audio
          ? {
              type: "input_audio",
              input_audio: { data: encoded, format: formats[media.mediaType] },
            }
          : {
              type: "video_url",
              video_url: { url: `data:${media.mediaType};base64,${encoded}` },
            };
        mediaStage = "transport";
        const response = await fetch(`${role.baseURL}/chat/completions`, {
          method: "POST",
          redirect: "error",
          signal: this.signal(role, signal),
          headers: {
            Authorization: `Bearer ${role.credential}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            model: role.profile.model,
            stream: false,
            messages: [
              { role: "system", content: role.prompt?.text ?? "" },
              {
                role: "user",
                content: [{ type: "text", text: content }, part],
              },
            ],
            temperature: role.binding.temperature,
            top_p: role.binding.top_p,
            max_tokens: role.binding.max_output_tokens,
            ...(schema
              ? {
                  response_format: {
                    type: "json_schema",
                    json_schema: {
                      name: output!.providerName,
                      strict: true,
                      schema: output!.providerSchema,
                    },
                  },
                }
              : {}),
          }),
        });
        if (!response.ok || !response.body) {
          await response.body?.cancel();
          throw new MediaProtocolError(
            `Direct media gateway HTTP ${response.status}`,
          );
        }
        const reader = response.body.getReader(),
          chunks: Uint8Array[] = [];
        mediaStage = "response_read";
        let size = 0;
        try {
          while (true) {
            const next = await reader.read();
            if (next.done) break;
            size += next.value.byteLength;
            if (size > 1048576) {
              await reader.cancel();
              throw new Error("Media response exceeds bounds");
            }
            chunks.push(next.value);
          }
        } finally {
          reader.releaseLock();
        }
        mediaStage = "response_validation";
        const result = z
          .object({
            choices: z
              .array(
                z.object({
                  finish_reason: z.literal("stop"),
                  message: z.object({ content: z.string().min(1).max(65536) }),
                }),
              )
              .min(1)
              .max(1),
            usage: z.unknown().optional(),
          })
          .parse(JSON.parse(Buffer.concat(chunks).toString("utf8")));
        const text = result.choices[0]!.message.content;
        const value = schema ? schema.parse(JSON.parse(text)) : text;
        return {
          value: value as ModelGenerationOutput<R>,
          producerMetadata: {
            ...this.metadata(role),
            implementation: "gateway-chat-media-v1",
            outputSchemaDigest: output?.digest,
          },
        };
      }
      const result = await generateText({
        model:
          role.profile.protocol === "openai-chat"
            ? role.provider.chat(role.profile.model)
            : role.provider.responses(role.profile.model),
        system: role.prompt?.text,
        messages: [{ role: "user", content }],
        output: output
          ? Output.object({
              schema: output.sdkSchema,
              name: output.providerName,
            })
          : undefined,
        temperature: role.binding.temperature,
        topP: role.binding.top_p,
        maxOutputTokens: role.binding.max_output_tokens,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (!schema && !result.text.trim()) throw new Error("Empty model output");
      if (schema && result.finishReason !== "stop")
        throw new ModelOutputError(`output_incomplete_${result.finishReason}`);
      const value = schema ? schema.parse(result.output) : result.text;
      return {
        value: value as ModelGenerationOutput<R>,
        producerMetadata: {
          ...this.metadata(role),
          outputSchemaDigest: output?.digest,
        },
      };
    } catch (error) {
      if (signal?.aborted) throw signal.reason;
      if (error instanceof MediaProtocolError) throw error;
      if (media)
        throw new MediaProtocolError(
          `Direct media invocation failed at ${mediaStage}`,
        );
      // Provider error bodies may echo headers, input material, or credentials.
      throw new GenerationFailure(name, safeGenerationFailure(error));
    }
  }
  async embeddingBatch(texts: string[], model: string, signal?: AbortSignal) {
    const role = this.require("query_embedding");
    if (
      role.profile.model !== model ||
      !role.profile.embedding ||
      !texts.length ||
      texts.length > 64 ||
      texts.reduce((bytes, text) => bytes + Buffer.byteLength(text), 0) >
        1048576
    )
      throw new Error("Embedding batch violates configured profile/bounds");
    try {
      const embeddings: number[][] = [];
      for (
        let offset = 0;
        offset < texts.length;
        offset += role.profile.embedding.max_batch_size
      ) {
        const result = await embedMany({
          model: role.provider.embeddingModel(model),
          values: texts.slice(
            offset,
            offset + role.profile.embedding.max_batch_size,
          ),
          maxRetries: 0,
          maxParallelCalls: 1,
          abortSignal: this.signal(role, signal),
        });
        embeddings.push(...result.embeddings);
      }
      if (
        embeddings.length !== texts.length ||
        embeddings.some(
          (vector) =>
            vector.length !== role.profile.embedding!.dimension ||
            vector.some((value) => !Number.isFinite(value)),
        )
      )
        throw new Error("Invalid embedding batch output");
      return {
        value: embeddings,
        producerMetadata: this.metadata(role),
      };
    } catch (error) {
      if (signal?.aborted) throw signal.reason;
      const status =
        error &&
        typeof error === "object" &&
        "statusCode" in error &&
        typeof error.statusCode === "number" &&
        Number.isInteger(error.statusCode)
          ? ` (HTTP ${error.statusCode})`
          : "";
      // oxlint-disable-next-line preserve-caught-error -- Provider causes can expose credentials or corpus text; only numeric status is public.
      throw new Error(
        `Embedding batch invocation failed validation or transport${status}`,
      );
    }
  }
  async transcription(
    audio: Uint8Array,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
  ) {
    const role = fixed
      ? this.hydrate(fixed)
      : this.require("speech_transcription");
    try {
      const result = await transcribe({
        model: role.provider.transcription(role.profile.model),
        audio,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (!result.text.trim()) throw new Error("Empty transcript");
      return { value: result.text, producerMetadata: this.metadata(role) };
    } catch {
      if (signal?.aborted) throw signal.reason;
      throw new Error("Speech transcription invocation failed");
    }
  }
  async rerank(
    query: string,
    documents: string[],
    topN: number,
    signal?: AbortSignal,
  ) {
    const role = this.require("query_rerank");
    if (
      !query.trim() ||
      !documents.length ||
      documents.length > 64 ||
      topN < 1 ||
      topN > documents.length ||
      Buffer.byteLength(JSON.stringify(documents)) > 2 * 1024 * 1024
    )
      throw new Error("Rerank request exceeds bounds");
    try {
      const response = await fetch(`${role.baseURL}/rerank`, {
        method: "POST",
        redirect: "error",
        signal: this.signal(role, signal),
        headers: {
          Authorization: `Bearer ${role.credential}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model: role.profile.model,
          query,
          documents,
          top_n: topN,
          return_documents: false,
        }),
      });
      if (!response.ok || !response.body) {
        await response.body?.cancel();
        throw new Error("Rerank gateway failure");
      }
      const reader = response.body.getReader();
      const chunks: Uint8Array[] = [];
      let size = 0;
      try {
        while (true) {
          const next = await reader.read();
          if (next.done) break;
          size += next.value.byteLength;
          if (size > 1024 * 1024) {
            await reader.cancel();
            throw new Error("Rerank response exceeds bounds");
          }
          chunks.push(next.value);
        }
      } finally {
        reader.releaseLock();
      }
      const result = parseRerankResponse(
        JSON.parse(
          new TextDecoder("utf-8", { fatal: true }).decode(
            Buffer.concat(chunks),
          ),
        ),
        documents.length,
      );
      return {
        value: result.results,
        producerMetadata: this.metadata(role),
      };
    } catch {
      if (signal?.aborted) throw signal.reason;
      throw new Error(
        "Rerank invocation failed or response violated candidate mapping",
      );
    }
  }
}
