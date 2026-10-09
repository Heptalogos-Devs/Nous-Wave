// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { createHash } from "node:crypto";
import type { UserContent } from "ai";
import {
  structuredMaterialResult,
  type StructuredMaterialContext,
} from "./schemas/material-interpretation.js";
import type { Degradation, Segment } from "../domain.js";
import { ModelInvocations, type ModelRoleSnapshot } from "./invocations.js";
import {
  modelConfigurationSchema,
  type ModelConfiguration,
} from "./configuration.js";

import { formationSchema } from "./schemas/formation.js";
import { projectionStewardSchema as proposalSchema } from "./schemas/projection.js";
import { descriptionSchema as interpretationSchema } from "./schemas/description.js";

export class ModelRuntime {
  constructor(
    readonly invocations = new ModelInvocations(),
    readonly materialStrategy: ModelConfiguration["material_strategy"] = "description_only",
    readonly video = modelConfigurationSchema.parse({}).video,
    readonly credentialEnvironments: readonly string[] = [],
    readonly audio = modelConfigurationSchema.parse({}).audio,
    readonly tempRoot?: string,
    readonly materialInputs = modelConfigurationSchema.parse({})
      .material_inputs,
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
    return new ModelRuntime(
      invocations,
      config.material_strategy,
      config.video,
      Object.values(config.gateway_profiles).map(
        (gateway) => gateway.credential_env,
      ),
      config.audio,
      tempRoot,
      config.material_inputs,
    );
  }
  get embeddingModel() {
    return this.invocations.profile("query_embedding")?.model;
  }
  async segmentEpisode(
    input: string,
    signal?: AbortSignal,
    snapshot?: ModelRoleSnapshot,
    beforeAttempt?: () => void,
  ) {
    return this.invocations.generate(
      "episode_segmentation",
      input,
      signal,
      undefined,
      snapshot,
      undefined,
      beforeAttempt,
    );
  }
  async synthesizeJournal(
    input: string,
    signal?: AbortSignal,
    snapshot?: ModelRoleSnapshot,
    beforeAttempt?: () => void,
  ) {
    return this.invocations.generate(
      "journal_synthesis",
      input,
      signal,
      undefined,
      snapshot,
      undefined,
      beforeAttempt,
    );
  }
  async maintainConcepts(
    input: string,
    signal?: AbortSignal,
    snapshot?: ModelRoleSnapshot,
    beforeAttempt?: () => void,
  ) {
    return this.invocations.generate(
      "concept_maintenance",
      input,
      signal,
      undefined,
      snapshot,
      undefined,
      beforeAttempt,
    );
  }
  async consolidate(
    input: string,
    signal?: AbortSignal,
    snapshot?: ModelRoleSnapshot,
    beforeAttempt?: () => void,
  ) {
    return this.invocations.generate(
      "memory_consolidation",
      input,
      signal,
      undefined,
      snapshot,
      undefined,
      beforeAttempt,
    );
  }
  async form(text: string, signal?: AbortSignal, snapshot?: ModelRoleSnapshot) {
    const result = await this.invocations.generate(
      "memory_formation",
      text,
      signal,
      undefined,
      snapshot,
    );
    return {
      ...formationSchema.parse(result.value),
      producerMetadata: result.producerMetadata,
      execution: result.execution,
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
      signal,
      undefined,
      fixed,
    );
    return {
      text: interpretationSchema.parse({ text: result.value }).text,
      producerMetadata: result.producerMetadata,
      execution: result.execution,
    };
  }
  async structure(
    input: string | { bytes: Uint8Array; mediaType: string },
    signal?: AbortSignal,
    fixed?: ModelRoleSnapshot,
    context?: StructuredMaterialContext,
  ) {
    if (!context)
      throw new Error(
        "Structured material requires a stable formation basis catalog",
      );
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
            evidence_text: input,
            evidence_kind: context.sourceText
              ? "original_text"
              : "committed_representation",
            evidence_access: context.evidenceAccess,
            basis_catalog: Object.keys(context.catalog),
            modalities: {
              visual: context.visual,
              audio: context.audio,
              source_text: context.sourceText,
            },
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
        text: JSON.stringify({ basis_catalog: Object.keys(context.catalog) }),
      });
    const result = await this.invocations.generate(
      role,
      content,
      signal,
      undefined,
      fixed,
      undefined,
      undefined,
      (value) => {
        structuredMaterialResult(value, context);
      },
    );
    return {
      ...structuredMaterialResult(result.value, context),
      producerMetadata: result.producerMetadata,
      execution: result.execution,
    };
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
      throw new Error(
        "Structured material requires a stable formation basis catalog",
      );
    const result = await this.invocations.generate(
      structured ? "material_direct_structuring" : "material_description",
      `Input media type: ${mediaType}. The attached material is the input evidence. Formation basis catalog: ${JSON.stringify(Object.keys(context?.catalog ?? {}))}.`,
      signal,
      undefined,
      fixed,
      { bytes, mediaType },
      undefined,
      structured
        ? (value) => {
            structuredMaterialResult(value, context!);
          }
        : undefined,
    );
    return {
      ...(structured
        ? structuredMaterialResult(result.value, context!)
        : { text: interpretationSchema.parse({ text: result.value }).text }),
      producerMetadata: result.producerMetadata,
      execution: result.execution,
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
      throw new Error(
        "Structured material requires a stable formation basis catalog",
      );
    const content = [
      {
        type: "text" as const,
        text: JSON.stringify({
          sampled_timestamps: frames.map((f) => f.timestamp),
          transcript,
          basis_catalog: Object.keys(context?.catalog ?? {}),
        }),
      },
      ...frames.flatMap((frame, index) => [
        {
          type: "text" as const,
          text: JSON.stringify({
            frame_index: index,
            timestamp_seconds: frame.timestamp,
            applies_to: "the immediately following image",
          }),
        },
        {
          type: "file" as const,
          data: frame.bytes,
          mediaType: "image/jpeg",
        },
      ]),
    ];
    const result = await this.invocations.generate(
      structured ? "material_direct_structuring" : "material_description",
      content,
      signal,
      fixed
        ? undefined
        : structured
          ? undefined
          : { role: "material_description", path: this.video.prompt },
      fixed,
      undefined,
      undefined,
      structured
        ? (value) => {
            structuredMaterialResult(value, context!);
          }
        : undefined,
    );
    return {
      ...(structured
        ? structuredMaterialResult(result.value, context!)
        : { text: interpretationSchema.parse({ text: result.value }).text }),
      producerMetadata: result.producerMetadata,
      execution: result.execution,
    };
  }

  async refine(
    segments: Segment[],
    signal?: AbortSignal,
  ): Promise<{ segments: Segment[]; degradation: Degradation[] }> {
    if (!this.invocations.profile("projection_steward"))
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
        (
          await this.invocations.generate(
            "projection_steward",
            JSON.stringify(
              segments.map((s) => ({
                id: s.segmentId,
                role: s.semanticRole,
                text: s.text,
              })),
            ),
            signal,
          )
        ).value,
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
          .update(
            JSON.stringify([
              this.invocations.identity("projection_steward"),
              sources,
              output.summary,
            ]),
          )
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
