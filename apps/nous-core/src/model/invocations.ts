// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import type { UserContent } from "ai";
import {
  generateProvider,
  embedProvider,
  transcribeProvider,
  rerankProvider,
  safeProviderFailure,
  usageCounts,
  ProviderFailure,
} from "./protocols.js";
import { z } from "zod";
import { resolvedEmbeddingRoute } from "./embedding-profile.js";
import { canonicalDigest } from "../digest.js";
import {
  providerContractForRole,
  type ModelGenerationOutput,
} from "./schemas/contracts.js";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { type ModelProfile, type ExecutionProfile } from "./profiles.js";
import { roleNames, type ModelRole } from "./roles.js";
import {
  modelRoleProblem,
  type ModelConfiguration,
  type RolePolicy,
  modelConfigurationSchema,
  modelExecutionSchema,
  modelRoleGraph,
} from "./configuration.js";
import { PromptRegistry, type PromptAsset } from "./prompts.js";
import { modelRoleIdentity, invocationConfigDigest } from "./identity.js";
import { modelImplementations } from "./implementation.js";
import {
  generationInput,
  InputUnavailable,
  type InputModalities,
  type DirectMedia,
} from "./input.js";

type GenerationInput = {
  content: UserContent;
  media?: DirectMedia;
  context?: (input: InputModalities) => Record<string, unknown>;
};
type GenerationOptions<R extends ModelRole> = {
  signal?: AbortSignal;
  snapshot?: ModelRoleSnapshot;
  prompt?: ModelRole | { role: ModelRole; path: string };
  beforeAttempt?: () => void;
  validateOutput?: (
    value: ModelGenerationOutput<R>,
    input: InputModalities,
  ) => void;
};

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
    ? (error.execution ?? interruptedExecutions.get(error))
    : error && typeof error === "object"
      ? interruptedExecutions.get(error)
      : undefined;
}
function interrupt(
  role: ModelRole,
  reason: unknown,
  attempts: ExecutionAttempt[],
): never {
  const error =
    reason && typeof reason === "object"
      ? reason
      : new GenerationFailure(role, "caller_cancelled", {
          attempts: [...attempts],
        });
  interruptedExecutions.set(error, { attempts: [...attempts] });
  throw error;
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
  status: "succeeded" | "failed" | "skipped" | "unknown";
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
  baseURL: string;
  credential: string;
  credentialEnv: string;
  timeout: number;
  profileDigest: string;
  configDigest: string;
};
const snapshotSchema = z.strictObject({
  format: z.literal("nous.model.execution"),
  implementationDigest: z.string().regex(/^[a-f0-9]{64}$/),
  outputSchemaDigest: z
    .string()
    .regex(/^[a-f0-9]{64}$/)
    .optional(),
  configuration: modelExecutionSchema,
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
    config = modelConfigurationSchema.parse(config);
    runtime.configuration = config;
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
        const binding = configured;
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
      format: "nous.model.execution",
      implementationDigest: modelImplementations.provider,
      outputSchemaDigest: providerContractForRole(name)?.digest,
      role: name,
      configuration: modelRoleGraph(this.configuration, name),
      prompt: role.prompt,
      profileDigest: role.profileDigest,
      configDigest: canonicalDigest({
        policy: role.policy,
        configuration: modelRoleGraph(this.configuration, name),
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
    if (snapshot.implementationDigest !== modelImplementations.provider)
      throw new Error("Reserved model implementation unavailable");
    if (
      snapshot.outputSchemaDigest !==
      providerContractForRole(snapshot.role)?.digest
    )
      throw new Error("Reserved model output contract unavailable");
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
    const binding = configured;
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
    };
  }
  private async withRoutes<
    T extends {
      value: unknown;
      producerMetadata: ModelProducerMetadata;
      usage?: unknown;
    },
    Input = undefined,
  >(
    name: ModelRole,
    invoke: (role: ReadyRole, input: Input) => Promise<T>,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
    beforeAttempt?: () => void,
    prepare?: (role: ReadyRole) => Input,
  ) {
    const snapshot = fixed ?? this.snapshot(name);
    if (snapshot.role !== name) throw new Error("Reserved model role mismatch");
    const policy = snapshot.configuration.roles[name];
    const attempts: ExecutionAttempt[] = [];
    for (const executionName of policy?.routes ?? []) {
      if (signal?.aborted) interrupt(name, signal.reason, attempts);
      let role: ReadyRole;
      let prepared: Input;
      try {
        role = this.hydrate(snapshot, executionName);
        prepared = prepare ? prepare(role) : (undefined as Input);
      } catch (error) {
        attempts.push({
          executionProfile: executionName,
          modelProfile:
            snapshot.configuration.execution_profiles[executionName]?.model ??
            "",
          status: "skipped",
          failureClass:
            error instanceof InputUnavailable
              ? error.message
              : "route_unavailable",
          latencyMs: 0,
        });
        continue;
      }
      // Budget admission is outside fallback handling. Exhaustion never tries another route.
      try {
        beforeAttempt?.();
      } catch (error) {
        interrupt(name, error, attempts);
      }
      const started = performance.now();
      try {
        const result = await invoke(role, prepared);
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
        const failureClass = signal?.aborted
          ? "caller_cancelled"
          : error instanceof GenerationFailure
            ? error.reason
            : safeProviderFailure(error);
        attempts.push({
          executionProfile: executionName,
          modelProfile: role.binding.model,
          status:
            signal?.aborted ||
            (error instanceof ProviderFailure && error.outcome === "unknown") ||
            [
              "validation_or_transport",
              "timeout_or_cancellation",
              "embedding_validation_or_transport",
            ].includes(failureClass)
              ? "unknown"
              : "failed",
          failureClass,
          usage:
            error instanceof GenerationFailure ||
            error instanceof ProviderFailure
              ? usageCounts(error.usage)
              : undefined,
          latencyMs: Math.round(performance.now() - started),
        });
        if (signal?.aborted) interrupt(name, signal.reason, attempts);
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
  private metadata(role: ReadyRole, adapter: string): ModelProducerMetadata {
    return {
      implementation: `${adapter}:${modelImplementations.provider}`,
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
  async generate<R extends ModelRole>(
    name: R,
    request: GenerationInput,
    options: GenerationOptions<R> = {},
  ) {
    const {
      signal,
      prompt: promptRole,
      beforeAttempt,
      validateOutput,
    } = options;
    const { content, media } = request;
    let snapshot = options.snapshot ?? this.snapshot(name);
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
    return this.withRoutes<
      {
        value: ModelGenerationOutput<R>;
        producerMetadata: ModelProducerMetadata;
        usage?: unknown;
        input: InputModalities;
      },
      { input: InputModalities; content: UserContent }
    >(
      name,
      async (role, { input, content: supplied }) => {
        const outputContract = providerContractForRole(name);
        const { adapter, ...output } = await generateProvider(
          role,
          supplied,
          outputContract,
          signal,
          media,
        );
        const result = {
          ...output,
          value: output.value as ModelGenerationOutput<R>,
          producerMetadata: {
            ...this.metadata(role, adapter),
            outputSchemaDigest: outputContract?.digest,
          },
        };
        try {
          validateOutput?.(result.value, input);
        } catch {
          // Owner validation errors may contain source content; keep the public classification bounded.
          throw new GenerationFailure(
            name,
            "output_semantics_invalid",
            undefined,
            result.usage,
          );
        }
        return { ...result, input };
      },
      signal,
      snapshot,
      beforeAttempt,
      (role) => {
        const input = generationInput(role.profile, content, media);
        const context = request.context?.(input);
        const supplied = context
          ? typeof content === "string"
            ? JSON.stringify({ input_context: context, content })
            : [
                {
                  type: "text" as const,
                  text: JSON.stringify({ input_context: context }),
                },
                ...content,
              ]
          : content;
        return {
          input: context ? { ...input, text: true } : input,
          content: supplied,
        };
      },
    );
  }
  async embeddingBatch(texts: string[], model: string, signal?: AbortSignal) {
    const result = await this.withRoutes(
      "query_embedding",
      async (role) => {
        const { adapter, ...output } = await embedProvider(
          role,
          texts,
          model,
          signal,
        );
        return { ...output, producerMetadata: this.metadata(role, adapter) };
      },
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
  async transcription(
    audio: Uint8Array,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
  ) {
    return this.withRoutes(
      "speech_transcription",
      async (role) => {
        const { adapter, ...result } = await transcribeProvider(
          role,
          audio,
          signal,
        );
        return { ...result, producerMetadata: this.metadata(role, adapter) };
      },
      signal,
      fixed,
    );
  }
  async rerank(
    query: string,
    documents: string[],
    topN: number,
    signal?: AbortSignal,
  ) {
    return this.withRoutes(
      "query_rerank",
      async (role) => {
        const { adapter, ...result } = await rerankProvider(
          role,
          query,
          documents,
          topN,
          signal,
        );
        return { ...result, producerMetadata: this.metadata(role, adapter) };
      },
      signal,
    );
  }
}
