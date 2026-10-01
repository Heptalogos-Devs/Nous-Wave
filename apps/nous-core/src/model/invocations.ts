import { createOpenAI } from "@ai-sdk/openai";
import type { ModelBudget } from "./budget.js";
import {
  embed,
  embedMany,
  generateText,
  Output,
  transcribe,
  type UserContent,
} from "ai";
import { z } from "zod";
import { canonicalDigest } from "../digest.js";
import { structuredOutputContract } from "./schemas/provider.js";
import { performance } from "node:perf_hooks";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  roleNames,
  type ModelConfiguration,
  type ModelProfile,
  type ModelRole,
  type RoleBinding,
  modelConfigurationSchema,
} from "./configuration.js";
import { PromptRegistry, type PromptAsset } from "./prompts.js";

class MediaProtocolError extends Error {}

export type ModelInvocationEvidence = {
  implementation: string;
  role: ModelRole;
  protocol: string;
  model: string;
  modelRevision?: string;
  profileDigest: string;
  promptId?: string;
  promptDigest?: string;
  outputSchemaDigest?: string;
  configDigest: string;
  latencyMs: number;
  usage?: { inputTokens?: number; outputTokens?: number; totalTokens?: number };
  requestCount: number;
  status: "success";
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
  private budget?: ModelBudget;
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
    budget?: ModelBudget,
  ) {
    const runtime = new ModelInvocations();
    runtime.budget = budget;
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
      const binding = config.roles[role];
      runtime.requirements.set(role, binding?.requirement ?? "optional");
      if (!binding) {
        runtime.states.set(role, {
          state: "NOT_CONFIGURED",
          detail: "Role has no binding",
        });
        continue;
      }
      const profile = config.model_profiles[binding.model]!;
      if (!profile.model.trim()) {
        runtime.states.set(role, {
          state: "NOT_CONFIGURED",
          detail: "Model identifier is unset",
        });
        continue;
      }
      const gateway = config.gateway_profiles[profile.gateway]!;
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
        const profileDigest = canonicalDigest({
          profile,
          destination: gateway.base_url,
        });
        const configDigest = canonicalDigest({
          binding: {
            ...binding,
            max_output_tokens: binding.max_output_tokens ?? 4096,
            timeout_ms: binding.timeout_ms ?? gateway.request_timeout_ms,
          },
          adapter: "ai-sdk@7.0.102/openai@4.0.67",
          profileDigest,
          prompt: prompt ? { id: prompt.id, digest: prompt.digest } : undefined,
        });
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
          state: "UNAVAILABLE",
          detail: "Client configured; live conformance remains unverified",
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
  private evidence(
    role: ReadyRole,
    start: number,
    usage?: unknown,
  ): ModelInvocationEvidence {
    const raw =
      usage && typeof usage === "object"
        ? (usage as Record<string, unknown>)
        : {};
    const number = (value: unknown): number | undefined => {
      const item =
        value && typeof value === "object" && "total" in value
          ? (value as { total: unknown }).total
          : value;
      return typeof item === "number" && Number.isSafeInteger(item) && item >= 0
        ? item
        : undefined;
    };
    const tokens =
      role.name === "query_rerank"
        ? {
            inputTokens: number(raw.prompt_tokens),
            outputTokens: number(raw.completion_tokens),
            totalTokens: number(raw.total_tokens),
          }
        : {
            inputTokens: number(raw.inputTokens ?? raw.tokens),
            outputTokens: number(raw.outputTokens),
            totalTokens: number(raw.totalTokens ?? raw.tokens),
          };
    return {
      implementation: "ai-sdk@7.0.102/openai@4.0.67",
      role: role.name,
      protocol: role.profile.protocol,
      model: role.profile.model,
      modelRevision: role.profile.model_revision,
      profileDigest: role.profileDigest,
      promptId: role.prompt?.id,
      promptDigest: role.prompt?.digest,
      configDigest: role.configDigest,
      latencyMs: performance.now() - start,
      usage: tokens,
      requestCount: 1,
      status: "success",
    };
  }
  private signal(role: ReadyRole, signal?: AbortSignal) {
    return signal
      ? AbortSignal.any([signal, AbortSignal.timeout(role.timeout)])
      : AbortSignal.timeout(role.timeout);
  }
  async generate<T>(
    name: ModelRole,
    content: UserContent,
    schema?: z.ZodType<T>,
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
    const output = schema ? structuredOutputContract(schema) : undefined;
    if (output && !role.profile.capabilities.includes("structured_output"))
      throw new Error("Strict structured output capability is unavailable");
    const start = performance.now();
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
        await this.budget?.reserve();
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
            max_tokens: role.binding.max_output_tokens ?? 4096,
            ...(schema
              ? {
                  response_format: {
                    type: "json_schema",
                    json_schema: {
                      name: "material",
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
        const usage = result.usage as Record<string, unknown> | undefined;
        this.states.set(name, {
          state: "READY",
          detail: "Direct media invocation validated",
        });
        return {
          value,
          evidence: {
            ...this.evidence(role, start, {
              inputTokens: usage?.prompt_tokens,
              outputTokens: usage?.completion_tokens,
              totalTokens: usage?.total_tokens,
            }),
            implementation: "gateway-chat-media-v1",
            outputSchemaDigest: output?.digest,
          },
        };
      }
      await this.budget?.reserve();
      const result = await generateText({
        model:
          role.profile.protocol === "openai-chat"
            ? role.provider.chat(role.profile.model)
            : role.provider.responses(role.profile.model),
        system: role.prompt?.text,
        messages: [{ role: "user", content }],
        output: output
          ? Output.object({ schema: output.sdkSchema })
          : undefined,
        temperature: role.binding.temperature,
        topP: role.binding.top_p,
        maxOutputTokens: role.binding.max_output_tokens ?? 4096,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (!schema && !result.text.trim()) throw new Error("Empty model output");
      if (schema && result.finishReason !== "stop")
        throw new Error("Structured output did not complete");
      const value = schema ? schema.parse(result.output) : result.text;
      this.states.set(name, {
        state: "READY",
        detail: "Standard protocol invocation validated",
      });
      return {
        value,
        evidence: {
          ...this.evidence(role, start, result.usage),
          outputSchemaDigest: output?.digest,
        },
      };
    } catch (error) {
      this.states.set(name, {
        state: "UNAVAILABLE",
        detail: "Model invocation failed validation or transport",
      });
      if (signal?.aborted) throw signal.reason;
      if (error instanceof MediaProtocolError) throw error;
      if (media)
        throw new MediaProtocolError(
          `Direct media invocation failed at ${mediaStage}`,
        );
      // Provider error bodies may echo headers, input material, or credentials.
      // oxlint-disable-next-line preserve-caught-error -- Provider causes may contain credentials or raw media.
      throw new Error(`Model role ${name} invocation failed`);
    }
  }
  async embedding(text: string, model: string, signal?: AbortSignal) {
    const role = this.require("query_embedding");
    if (role.profile.model !== model || !role.profile.embedding)
      throw new Error("Embedding profile disagrees with Kernel space");
    const start = performance.now();
    try {
      await this.budget?.reserve();
      const result = await embed({
        model: role.provider.embeddingModel(model),
        value: text,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (
        result.embedding.length !== role.profile.embedding.dimension ||
        result.embedding.some((v) => !Number.isFinite(v))
      )
        throw new Error("Invalid vector");
      this.states.set("query_embedding", {
        state: "READY",
        detail: "Embedding invocation validated",
      });
      return {
        value: result.embedding,
        evidence: this.evidence(role, start, result.usage),
      };
    } catch {
      this.states.set("query_embedding", {
        state: "UNAVAILABLE",
        detail: "Embedding invocation unavailable",
      });
      if (signal?.aborted) throw signal.reason;
      throw new Error(
        "Embedding invocation failed or vector violated the configured space",
      );
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
    const start = performance.now();
    try {
      const embeddings: number[][] = [];
      const calls: ModelInvocationEvidence[] = [];
      for (
        let offset = 0;
        offset < texts.length;
        offset += role.profile.embedding.max_batch_size
      ) {
        await this.budget?.reserve();
        const callStart = performance.now();
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
        calls.push(this.evidence(role, callStart, result.usage));
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
      this.states.set("query_embedding", {
        state: "READY",
        detail: "Embedding batch validated",
      });
      return {
        value: embeddings,
        evidence: {
          ...this.evidence(role, start),
          requestCount: calls.length,
          usage: {
            inputTokens: calls.every(
              (call) => call.usage?.inputTokens !== undefined,
            )
              ? calls.reduce((sum, call) => sum + call.usage!.inputTokens!, 0)
              : undefined,
            outputTokens: calls.every(
              (call) => call.usage?.outputTokens !== undefined,
            )
              ? calls.reduce((sum, call) => sum + call.usage!.outputTokens!, 0)
              : undefined,
            totalTokens: calls.every(
              (call) => call.usage?.totalTokens !== undefined,
            )
              ? calls.reduce((sum, call) => sum + call.usage!.totalTokens!, 0)
              : undefined,
          },
        },
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
    const start = performance.now();
    try {
      await this.budget?.reserve();
      const result = await transcribe({
        model: role.provider.transcription(role.profile.model),
        audio,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (!result.text.trim()) throw new Error("Empty transcript");
      this.states.set("speech_transcription", {
        state: "READY",
        detail: "Transcription invocation validated",
      });
      return { value: result.text, evidence: this.evidence(role, start) };
    } catch {
      this.states.set("speech_transcription", {
        state: "UNAVAILABLE",
        detail: "Transcription invocation unavailable",
      });
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
    const start = performance.now();
    try {
      await this.budget?.reserve();
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
      this.states.set("query_rerank", {
        state: "READY",
        detail: "Rerank invocation validated",
      });
      return {
        value: result.results,
        evidence: this.evidence(role, start, result.usage),
      };
    } catch {
      this.states.set("query_rerank", {
        state: "UNAVAILABLE",
        detail: "Rerank invocation unavailable",
      });
      if (signal?.aborted) throw signal.reason;
      throw new Error(
        "Rerank invocation failed or response violated candidate mapping",
      );
    }
  }
}
