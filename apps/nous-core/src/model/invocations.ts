// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

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
import { resolvedEmbeddingRoute } from "./embedding-profile.js";
import { canonicalDigest } from "../digest.js";
import {
  providerContractForRole,
  type ModelGenerationOutput,
} from "./schemas/contracts.js";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  roleNames,
  resolveExecutionProfile,
  modelRoleProblem,
  type ModelConfiguration,
  type ModelProfile,
  type ModelRole,
  type ExecutionProfile,
  type RolePolicy,
  modelConfigurationSchema,
} from "./configuration.js";
import { PromptRegistry, type PromptAsset } from "./prompts.js";
import { modelRoleIdentity, invocationConfigDigest } from "./identity.js";

class MediaProtocolError extends Error {}
class ModelOutputError extends Error {
  constructor(
    message: string,
    readonly usage?: unknown,
  ) {
    super(message);
  }
}
export class GenerationFailure extends Error {
  constructor(
    role: ModelRole,
    readonly reason: string,
    readonly execution?: ExecutionTelemetry,
    readonly usage?: unknown,
  ) {
    super(`Model role ${role} invocation failed: ${reason}`);
  }
}

const interruptedExecutions = new WeakMap<object, ExecutionTelemetry>();
export function failedExecutionTelemetry(
  error: unknown,
): ExecutionTelemetry | undefined {
  return error instanceof GenerationFailure
    ? error.execution
    : error && typeof error === "object"
      ? interruptedExecutions.get(error)
      : undefined;
}
function usageCounts(input: unknown) {
  if (!input || typeof input !== "object") return undefined;
  const counters = Object.fromEntries(Object.entries(input));
  const count = (entry: unknown) => {
    const value =
      entry && typeof entry === "object" && "total" in entry
        ? entry.total
        : entry;
    return typeof value === "number" &&
      Number.isSafeInteger(value) &&
      value >= 0
      ? value
      : undefined;
  };
  const usage = {
    inputTokens: count(counters.inputTokens ?? counters.prompt_tokens),
    outputTokens: count(counters.outputTokens ?? counters.completion_tokens),
    totalTokens: count(counters.totalTokens ?? counters.total_tokens),
  };
  return Object.values(usage).some((value) => value !== undefined)
    ? usage
    : undefined;
}
function safeGenerationFailure(error: unknown) {
  if (error instanceof GenerationFailure) return error.reason;
  if (error instanceof MediaProtocolError) return error.message;
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
  modelRole: ModelRole;
  modelProfile: string;
  executionProfile: string;
  inferenceControlsDigest: string;
  rolePolicyDigest: string;
};
type ExecutionAttempt = {
  executionProfile: string;
  modelProfile: string;
  status: "succeeded" | "failed";
  failureClass?: string;
  latencyMs: number;
  usage?: unknown;
};
export type ExecutionTelemetry = {
  attempts: ExecutionAttempt[];
  successfulExecutionProfile?: string;
};
type ReadyRole = {
  name: ModelRole;
  profile: ModelProfile;
  binding: ExecutionProfile;
  executionName: string;
  policy: RolePolicy;
  policyDigest: string;
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
  private readonly active = new Map<ModelRole, ReadyRole[]>();
  private configuration!: ModelConfiguration;
  private readonly credentialOrigins = new Set<string>();
  private readonly requirements = new Map<
    ModelRole,
    RolePolicy["requirement"]
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
    runtime.configuration = modelConfigurationSchema.parse(config);
    for (const gateway of Object.values(config.gateway_profiles))
      if (gateway.enabled)
        runtime.credentialOrigins.add(
          `${gateway.credential_env}@${new URL(gateway.base_url).origin}`,
        );
    const prompts = new PromptRegistry(promptRoot, overridePromptRoot);
    runtime.prompts = prompts;
    for (const name of roleNames) {
      const policy = config.roles[name];
      runtime.requirements.set(name, policy?.requirement ?? "optional");
      if (!policy) {
        runtime.states.set(name, {
          state: "NOT_CONFIGURED",
          detail: "Role has no policy",
        });
        continue;
      }
      if (policy.prompt) runtime.promptPaths[name] = policy.prompt;
      const routes: ReadyRole[] = [];
      let problem = "Execution route unavailable";
      let prompt: PromptAsset | undefined;
      try {
        prompt = await prompts.load(name, policy.prompt);
      } catch {
        problem = "Prompt asset unavailable or invalid";
      }
      const policyDigest = canonicalDigest(policy);
      for (const executionName of policy.routes) {
        const configured = config.execution_profiles[executionName];
        const profile = configured && config.model_profiles[configured.model];
        if (!configured || !profile || !profile.model.trim()) {
          problem = "Execution/model profile is absent or unset";
          continue;
        }
        const binding = resolveExecutionProfile(configured, profile.protocol);
        const mismatch = modelRoleProblem(name, binding, profile);
        if (mismatch) {
          problem = mismatch;
          continue;
        }
        const gateway = config.gateway_profiles[profile.gateway];
        const credential = gateway && process.env[gateway.credential_env];
        if (!gateway?.enabled || !credential) {
          problem = "Gateway disabled or credential unavailable";
          continue;
        }
        if (!prompt && providerContractForRole(name)) {
          problem = "Prompt asset unavailable or invalid";
          continue;
        }
        const identity = modelRoleIdentity(profile, gateway, binding, prompt);
        routes.push({
          name,
          profile,
          binding,
          executionName,
          policy,
          policyDigest,
          prompt,
          provider: createOpenAI({
            baseURL: gateway.base_url,
            apiKey: credential,
            name: "standard-gateway",
            fetch: (destination, init) =>
              fetch(destination, { ...init, redirect: "error" }),
          }),
          credential,
          credentialEnv: gateway.credential_env,
          baseURL: gateway.base_url,
          timeout: binding.timeout_ms ?? gateway.request_timeout_ms,
          profileDigest: identity.profileDigest,
          configDigest: canonicalDigest({
            execution: identity.configDigest,
            policyDigest,
            executionName,
          }),
        });
      }
      if (name === "query_embedding" && routes.length > 1) {
        const signature = (route: ReadyRole) =>
          canonicalDigest({
            model: route.profile.model,
            embedding: route.profile.embedding,
          });
        if (routes.some((route) => signature(route) !== signature(routes[0]!)))
          throw new Error(
            "Embedding routes require the identical full embedding space signature",
          );
      }
      runtime.active.set(name, routes);
      runtime.states.set(name, {
        state: routes.length ? "READY" : "UNAVAILABLE",
        detail: routes.length ? "Executable ordered routes available" : problem,
      });
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
    return this.active.get(role)?.[0]?.profile;
  }
  requirement(role: ModelRole) {
    return this.requirements.get(role) ?? "optional";
  }
  snapshot(name: ModelRole): ModelRoleSnapshot {
    const role = this.require(name);
    return snapshotSchema.parse({
      role: name,
      configuration: this.configuration,
      prompt: role.prompt,
      profileDigest: role.profileDigest,
      configDigest: canonicalDigest({
        policy: role.policy,
        configuration: this.configuration,
        prompt: role.prompt && {
          id: role.prompt.id,
          digest: role.prompt.digest,
        },
        outputSchemaDigest: providerContractForRole(name)?.digest,
      }),
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
    }
    snapshot.configDigest = invocationConfigDigest(
      snapshot.configDigest,
      path ? snapshot.prompt : undefined,
      adapter,
    );
    return snapshot;
  }
  private hydrate(input: ModelRoleSnapshot, executionName: string): ReadyRole {
    const snapshot = snapshotSchema.parse(input);
    const policy = snapshot.configuration.roles[snapshot.role];
    if (!policy?.routes.includes(executionName))
      throw new Error("Reserved execution route is absent");
    const configured = snapshot.configuration.execution_profiles[executionName];
    const profile =
      configured && snapshot.configuration.model_profiles[configured.model];
    const gateway =
      profile && snapshot.configuration.gateway_profiles[profile.gateway];
    if (!configured || !profile || !gateway)
      throw new Error("Reserved execution resources unavailable");
    const binding = resolveExecutionProfile(configured, profile.protocol);
    const problem = modelRoleProblem(snapshot.role, binding, profile);
    if (problem) throw new Error(problem);
    const credential = process.env[gateway.credential_env];
    if (
      !this.credentialOrigins.has(
        `${gateway.credential_env}@${new URL(gateway.base_url).origin}`,
      )
    )
      throw new Error(
        "Reserved gateway credential destination is no longer authorized by current configuration",
      );
    if (!gateway.enabled || !credential)
      throw new Error("Reserved model credential reference unavailable");
    const identity = modelRoleIdentity(
      profile,
      gateway,
      binding,
      snapshot.prompt,
    );
    const policyDigest = canonicalDigest(policy);
    return {
      name: snapshot.role,
      profile,
      binding,
      executionName,
      policy,
      policyDigest,
      prompt: snapshot.prompt,
      baseURL: gateway.base_url,
      credential,
      credentialEnv: gateway.credential_env,
      timeout: binding.timeout_ms ?? gateway.request_timeout_ms,
      profileDigest: identity.profileDigest,
      configDigest: canonicalDigest({
        snapshot: snapshot.configDigest,
        execution: identity.configDigest,
        policyDigest,
        executionName,
      }),
      provider: createOpenAI({
        baseURL: gateway.base_url,
        apiKey: credential,
        name: "standard-gateway",
        fetch: (destination, init) =>
          fetch(destination, { ...init, redirect: "error" }),
      }),
    };
  }
  private async withRoutes<
    T extends {
      value: unknown;
      producerMetadata: ModelProducerMetadata;
      usage?: unknown;
    },
  >(
    name: ModelRole,
    invoke: (role: ReadyRole) => Promise<T>,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
    beforeAttempt?: () => void,
  ) {
    const snapshot = fixed ?? this.snapshot(name);
    if (snapshot.role !== name) throw new Error("Reserved model role mismatch");
    const policy = snapshot.configuration.roles[name];
    const attempts: ExecutionAttempt[] = [];
    for (const executionName of policy?.routes ?? []) {
      if (signal?.aborted) throw signal.reason;
      let role: ReadyRole;
      try {
        role = this.hydrate(snapshot, executionName);
      } catch {
        attempts.push({
          executionProfile: executionName,
          modelProfile:
            snapshot.configuration.execution_profiles[executionName]?.model ??
            "",
          status: "failed",
          failureClass: "route_unavailable",
          latencyMs: 0,
        });
        continue;
      }
      // Budget admission is outside fallback handling. Exhaustion never tries another route.
      try {
        beforeAttempt?.();
      } catch (error) {
        if (error && typeof error === "object")
          interruptedExecutions.set(error, { attempts: [...attempts] });
        throw error;
      }
      const started = performance.now();
      try {
        const result = await invoke(role);
        attempts.push({
          executionProfile: executionName,
          modelProfile: role.binding.model,
          status: "succeeded",
          latencyMs: Math.round(performance.now() - started),
          usage: "usage" in result ? usageCounts(result.usage) : undefined,
        });
        const { usage: _usage, ...record } = result;
        return {
          ...record,
          execution: {
            attempts,
            successfulExecutionProfile: executionName,
          } satisfies ExecutionTelemetry,
        };
      } catch (error) {
        if (signal?.aborted) throw signal.reason;
        attempts.push({
          executionProfile: executionName,
          modelProfile: role.binding.model,
          status: "failed",
          failureClass: safeGenerationFailure(error),
          usage:
            error instanceof GenerationFailure
              ? usageCounts(error.usage)
              : undefined,
          latencyMs: Math.round(performance.now() - started),
        });
      }
    }
    throw new GenerationFailure(
      name,
      `all_execution_routes_failed:${attempts.at(-1)?.failureClass ?? "route_unavailable"}`,
      { attempts },
    );
  }
  identity(role: ModelRole) {
    return this.active.get(role)?.[0]?.configDigest;
  }
  private require(role: ModelRole): ReadyRole {
    const value = this.active.get(role)?.[0];
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
      modelRole: role.name,
      modelProfile: role.binding.model,
      executionProfile: role.executionName,
      inferenceControlsDigest: canonicalDigest(role.binding),
      rolePolicyDigest: role.policyDigest,
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
    beforeAttempt?: () => void,
  ) {
    let snapshot = fixed ?? this.snapshot(name);
    if (promptRole) {
      const selected =
        typeof promptRole === "string" ? promptRole : promptRole.role;
      const prompt = await this.prompts?.load(
        selected,
        typeof promptRole === "string"
          ? this.promptPaths[selected]
          : promptRole.path,
      );
      if (!prompt) throw new Error("Strategy prompt unavailable");
      snapshot = {
        ...snapshot,
        prompt,
        configDigest: invocationConfigDigest(snapshot.configDigest, prompt),
      };
    }
    return this.withRoutes(
      name,
      (role) => this.generateOnce(name, content, signal, role, media),
      signal,
      snapshot,
      beforeAttempt,
    );
  }
  private async generateOnce<R extends ModelRole>(
    name: R,
    content: UserContent,
    signal: AbortSignal | undefined,
    role: ReadyRole,
    media?: { bytes: Uint8Array; mediaType: string },
  ) {
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
            ...(role.binding.reasoning === "provider-default"
              ? {}
              : { reasoning_effort: role.binding.reasoning }),
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
          usage: usageCounts(result.usage),
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
        instructions: role.prompt?.text,
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
        reasoning: role.binding.reasoning,
        providerOptions: role.binding.provider_options,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (!schema && !result.text.trim()) throw new Error("Empty model output");
      if (schema && result.finishReason !== "stop")
        throw new ModelOutputError(
          `output_incomplete_${result.finishReason}`,
          result.totalUsage,
        );
      const value = schema ? schema.parse(result.output) : result.text;
      return {
        value: value as ModelGenerationOutput<R>,
        usage: result.totalUsage,
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
      throw new GenerationFailure(
        name,
        safeGenerationFailure(error),
        undefined,
        NoObjectGeneratedError.isInstance(error) ||
          error instanceof ModelOutputError
          ? usageCounts(error.usage)
          : undefined,
      );
    }
  }
  async embeddingBatch(texts: string[], model: string, signal?: AbortSignal) {
    const result = await this.withRoutes(
      "query_embedding",
      (role) => this.embeddingBatchOnce(texts, model, signal, role),
      signal,
    );
    const producer = resolvedEmbeddingRoute(
      this.configuration,
      result.producerMetadata.executionProfile,
    )?.producer;
    if (!producer)
      throw new Error(
        "Successful embedding execution has no canonical producer",
      );
    return { ...result, producer };
  }
  private async embeddingBatchOnce(
    texts: string[],
    model: string,
    signal: AbortSignal | undefined,
    role: ReadyRole,
  ) {
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
    return this.withRoutes(
      "speech_transcription",
      (role) => this.transcriptionOnce(audio, signal, role),
      signal,
      fixed,
    );
  }
  private async transcriptionOnce(
    audio: Uint8Array,
    signal: AbortSignal | undefined,
    role: ReadyRole,
  ) {
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
    return this.withRoutes(
      "query_rerank",
      (role) => this.rerankOnce(query, documents, topN, signal, role),
      signal,
    );
  }
  private async rerankOnce(
    query: string,
    documents: string[],
    topN: number,
    signal: AbortSignal | undefined,
    role: ReadyRole,
  ) {
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
