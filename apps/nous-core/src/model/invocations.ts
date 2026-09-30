import { createOpenAI } from "@ai-sdk/openai";
import { embed, generateText, Output, transcribe, type UserContent } from "ai";
import { z } from "zod";
import { performance } from "node:perf_hooks";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  roleNames,
  type ModelConfiguration,
  type ModelProfile,
  type ModelRole,
  type RoleBinding,
} from "./configuration.js";
import {
  canonicalDigest,
  PromptRegistry,
  type PromptAsset,
} from "./prompts.js";

export type ModelInvocationEvidence = {
  protocol: string;
  model: string;
  modelRevision?: string;
  profileDigest: string;
  promptId?: string;
  promptDigest?: string;
  configDigest: string;
  latencyMs: number;
  usage?: unknown;
};
type ReadyRole = {
  profile: ModelProfile;
  binding: RoleBinding;
  prompt?: PromptAsset;
  provider: ReturnType<typeof createOpenAI>;
  baseURL: string;
  credential: string;
  timeout: number;
  profileDigest: string;
  configDigest: string;
};
const rerankResponse = z
  .object({
    results: z
      .array(
        z
          .object({
            index: z.number().int().nonnegative(),
            relevance_score: z.number().finite(),
          })
          .passthrough(),
      )
      .max(64),
    usage: z.unknown().optional(),
  })
  .passthrough();
export function parseRerankResponse(input: unknown, count: number) {
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
  private readonly active = new Map<ModelRole, ReadyRole>();
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
  ) {
    const runtime = new ModelInvocations();
    const prompts = new PromptRegistry(promptRoot);
    for (const role of roleNames) {
      const binding = config.roles[role];
      if (!binding) {
        runtime.states.set(role, {
          state: "NOT_CONFIGURED",
          detail: "Role has no binding",
        });
        continue;
      }
      const profile = config.model_profiles[binding.model]!;
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
          binding,
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
          profile,
          binding,
          prompt,
          provider,
          credential,
          baseURL: gateway.base_url,
          timeout: binding.timeout_ms ?? gateway.request_timeout_ms,
          profileDigest,
          configDigest,
        });
        runtime.states.set(role, {
          state: "READY",
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
    return {
      protocol: role.profile.protocol,
      model: role.profile.model,
      modelRevision: role.profile.model_revision,
      profileDigest: role.profileDigest,
      promptId: role.prompt?.id,
      promptDigest: role.prompt?.digest,
      configDigest: role.configDigest,
      latencyMs: performance.now() - start,
      usage,
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
  ) {
    const role = this.require(name);
    if (
      role.profile.protocol !== "openai-chat" &&
      role.profile.protocol !== "openai-responses"
    )
      throw new Error("Role is not a text generation protocol");
    const start = performance.now();
    try {
      const result = await generateText({
        model:
          role.profile.protocol === "openai-chat"
            ? role.provider.chat(role.profile.model)
            : role.provider.responses(role.profile.model),
        system: role.prompt?.text,
        messages: [{ role: "user", content }],
        output: schema ? Output.object({ schema }) : undefined,
        temperature: role.binding.temperature,
        topP: role.binding.top_p,
        maxOutputTokens: role.binding.max_output_tokens ?? 4096,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      return {
        value: schema ? schema.parse(result.output) : result.text,
        evidence: this.evidence(role, start, result.usage),
      };
    } catch {
      if (signal?.aborted) throw signal.reason;
      // Provider error bodies may echo headers, input material, or credentials.
      throw new Error(`Model role ${name} invocation failed`);
    }
  }
  async embedding(text: string, model: string, signal?: AbortSignal) {
    const role = this.require("query_embedding");
    if (role.profile.model !== model || !role.profile.embedding)
      throw new Error("Embedding profile disagrees with Kernel space");
    const start = performance.now();
    try {
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
      return {
        value: result.embedding,
        evidence: this.evidence(role, start, result.usage),
      };
    } catch {
      if (signal?.aborted) throw signal.reason;
      throw new Error(
        "Embedding invocation failed or vector violated the configured space",
      );
    }
  }
  async transcription(audio: Uint8Array, signal?: AbortSignal) {
    const role = this.require("speech_transcription");
    const start = performance.now();
    try {
      const result = await transcribe({
        model: role.provider.transcription(role.profile.model),
        audio,
        maxRetries: 0,
        abortSignal: this.signal(role, signal),
      });
      if (!result.text.trim()) throw new Error("Empty transcript");
      return { value: result.text, evidence: this.evidence(role, start) };
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
    const start = performance.now();
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
        evidence: this.evidence(role, start, result.usage),
      };
    } catch {
      if (signal?.aborted) throw signal.reason;
      throw new Error(
        "Rerank invocation failed or response violated candidate mapping",
      );
    }
  }
}
