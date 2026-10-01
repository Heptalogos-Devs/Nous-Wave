import { createHash } from "node:crypto";
import { z } from "zod";
import type { UserContent } from "ai";
import {
  materialInterpretationSchema,
  structuredMaterialResult,
  type StructuredMaterialContext,
} from "./schemas/material-interpretation.js";
import type { Degradation, Segment } from "../domain.js";
import { ModelInvocations, type ModelRoleSnapshot } from "./invocations.js";
import {
  modelConfigurationSchema,
  type ModelConfiguration,
} from "./configuration.js";

const proposalSchema = z.strictObject({
  selectedIds: z.array(z.string()).max(64),
  summary: z.string().max(8192).optional(),
});
type StewardProposal = z.infer<typeof proposalSchema>;
export type ProposalGenerator = (
  segments: Segment[],
  signal?: AbortSignal,
) => Promise<StewardProposal>;
const formationSchema = z.strictObject({
  text: z.string().min(1).max(32768),
  semanticRole: z.string().min(1).max(128),
  title: z.string().max(256).optional(),
  selectedEntityKeys: z.array(z.string().max(128)).max(128).default([]),
});
const interpretationSchema = z.strictObject({
  text: z.string().min(1).max(65536),
});

export class ModelRuntime {
  constructor(
    private readonly generator?: ProposalGenerator,
    private readonly producer = "deterministic",
    readonly invocations = new ModelInvocations(),
    readonly materialStrategy: ModelConfiguration["material_strategy"] = "description_only",
    readonly video = modelConfigurationSchema.parse({}).video,
    readonly credentialEnvironments: readonly string[] = [],
    readonly audio = modelConfigurationSchema.parse({}).audio,
    readonly tempRoot?: string,
  ) {}
  static async fromConfig(
    config: ModelConfiguration,
    promptRoot?: string,
    overridePromptRoot?: string,
    tempRoot?: string,
  ) {
    const invocations = await ModelInvocations.create(
      config,
      promptRoot,
      overridePromptRoot,
    );
    const steward = invocations.profile("projection_steward");
    const generator: ProposalGenerator | undefined = steward
      ? async (segments, signal) => {
          const result = await invocations.generate(
            "projection_steward",
            JSON.stringify(
              segments.map((s) => ({
                id: s.segmentId,
                role: s.semanticRole,
                text: s.text,
              })),
            ),
            proposalSchema,
            signal,
          );
          return proposalSchema.parse(result.value);
        }
      : undefined;
    return new ModelRuntime(
      generator,
      invocations.identity("projection_steward") ?? "deterministic",
      invocations,
      config.material_strategy,
      config.video,
      Object.values(config.gateway_profiles).map(
        (gateway) => gateway.credential_env,
      ),
      config.audio,
      tempRoot,
    );
  }
  get embeddingModel() {
    return this.invocations.profile("query_embedding")?.model;
  }
  async form(text: string, signal?: AbortSignal, snapshot?: ModelRoleSnapshot) {
    const result = await this.invocations.generate(
      "memory_formation",
      text,
      formationSchema,
      signal,
      undefined,
      snapshot,
    );
    return {
      ...formationSchema.parse(result.value),
      evidence: result.evidence,
    };
  }
  async interpret(
    bytes: Uint8Array,
    mediaType: string,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
  ) {
    if (!mediaType.startsWith("image/"))
      throw new Error("Image description requires image material");
    if (
      !this.invocations
        .profile("material_description")
        ?.capabilities.includes("image_input")
    )
      throw new Error("Image model capability is unavailable");
    const content = [{ type: "file" as const, data: bytes, mediaType }];
    const result = await this.invocations.generate(
      "material_description",
      content,
      undefined,
      signal,
      undefined,
      fixed,
    );
    return {
      text: interpretationSchema.parse({ text: result.value }).text,
      evidence: result.evidence,
    };
  }
  async structure(
    input: string | { bytes: Uint8Array; mediaType: string },
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
    context?: StructuredMaterialContext,
  ) {
    if (!context)
      throw new Error("Structured material requires a stable support catalog");
    const role =
      typeof input === "string"
        ? "material_structuring"
        : "material_direct_structuring";
    if (
      typeof input !== "string" &&
      !this.invocations.profile(role)?.capabilities.includes("image_input")
    )
      throw new Error("Direct structured image capability is unavailable");
    const content: UserContent =
      typeof input === "string"
        ? JSON.stringify({
            source_text: input,
            support_catalog: Object.keys(context.catalog),
            modalities: { visual: context.visual, audio: context.audio },
          })
        : [
            {
              type: "file" as const,
              data: input.bytes,
              mediaType: input.mediaType,
            },
          ];
    if (Array.isArray(content))
      content.unshift({
        type: "text",
        text: JSON.stringify({ support_catalog: Object.keys(context.catalog) }),
      });
    const result = await this.invocations.generate(
      role,
      content,
      materialInterpretationSchema,
      signal,
      undefined,
      fixed,
    );
    return {
      ...structuredMaterialResult(result.value, context),
      evidence: result.evidence,
    };
  }
  async embedding(text: string, model: string, signal?: AbortSignal) {
    if (!this.embeddingModel || model !== this.embeddingModel)
      throw new Error("Embedding model is not configured for the Kernel space");
    return this.invocations.embedding(text, model, signal);
  }
  async describeMedia(
    bytes: Uint8Array,
    mediaType: string,
    structured: boolean,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
    context?: StructuredMaterialContext,
  ) {
    if (structured && !context)
      throw new Error("Structured material requires a stable support catalog");
    const result = await this.invocations.generate(
      structured ? "material_direct_structuring" : "material_description",
      `Input media type: ${mediaType}. The attached material is the input evidence. Support catalog: ${JSON.stringify(Object.keys(context?.catalog ?? {}))}.`,
      structured ? materialInterpretationSchema : undefined,
      signal,
      undefined,
      fixed,
      { bytes, mediaType },
    );
    return {
      ...(structured
        ? structuredMaterialResult(result.value, context!)
        : { text: interpretationSchema.parse({ text: result.value }).text }),
      evidence: result.evidence,
    };
  }
  async describeScene(
    frames: { bytes: Uint8Array; timestamp: number }[],
    transcript: string | undefined,
    structured: boolean,
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
    context?: StructuredMaterialContext,
  ) {
    if (
      !this.invocations
        .profile(
          structured ? "material_direct_structuring" : "material_description",
        )
        ?.capabilities.includes("image_input")
    )
      throw new Error("Video frame model capability is unavailable");
    if (structured && !context)
      throw new Error("Structured material requires a stable support catalog");
    const content = [
      {
        type: "text" as const,
        text: JSON.stringify({
          sampled_timestamps: frames.map((f) => f.timestamp),
          transcript,
          support_catalog: Object.keys(context?.catalog ?? {}),
        }),
      },
      ...frames.map((f) => ({
        type: "file" as const,
        data: f.bytes,
        mediaType: "image/jpeg",
      })),
    ];
    const result = await this.invocations.generate(
      structured ? "material_direct_structuring" : "material_description",
      content,
      structured ? materialInterpretationSchema : undefined,
      signal,
      fixed
        ? undefined
        : structured
          ? undefined
          : { role: "material_description", path: this.video.prompt },
      fixed,
    );
    return {
      ...(structured
        ? structuredMaterialResult(result.value, context!)
        : { text: interpretationSchema.parse({ text: result.value }).text }),
      evidence: result.evidence,
    };
  }

  async refine(
    segments: Segment[],
    signal?: AbortSignal,
  ): Promise<{ segments: Segment[]; degradation: Degradation[] }> {
    if (!this.generator)
      return {
        segments,
        degradation: [
          {
            code: "steward_not_configured",
            detail: "Deterministic projection",
          },
        ],
      };
    try {
      const output = proposalSchema.parse(
        await this.generator(segments, signal),
      );
      const ids = new Set(segments.map((s) => s.segmentId));
      if (
        new Set(output.selectedIds).size !== output.selectedIds.length ||
        output.selectedIds.some((id) => !ids.has(id))
      )
        throw new Error("Steward proposed invalid source IDs");
      const selected = new Set(output.selectedIds);
      const sources = segments.filter((s) => selected.has(s.segmentId));
      if (output.summary) {
        if (!sources.length)
          throw new Error("A Steward summary requires selected sources");
        const revision = createHash("sha256")
          .update(JSON.stringify([this.producer, sources, output.summary]))
          .digest("hex");
        return {
          segments: [
            {
              segmentId: `steward:${revision}`,
              text: output.summary,
              semanticRole: "steward_synthesis",
              sourceRefs: sources.flatMap((s) => s.sourceRefs),
              evidence: sources.flatMap((s) => s.evidence),
              authority: "interpretation",
              stability: "EPOCH_STABLE",
              sourceRevision: revision,
            },
          ],
          degradation: [],
        };
      }
      return { segments: sources, degradation: [] };
    } catch (error) {
      if (signal?.aborted) throw signal.reason;
      return {
        segments,
        degradation: [
          {
            code: "steward_rejected",
            detail: error instanceof Error ? error.message : "Invalid proposal",
          },
        ],
      };
    }
  }
}
