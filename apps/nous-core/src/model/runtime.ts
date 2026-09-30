import { createHash } from "node:crypto";
import { z } from "zod";
import type { Degradation, Segment } from "../domain.js";
import { ModelInvocations } from "./invocations.js";
import type { ModelConfiguration } from "./configuration.js";

const proposalSchema = z
  .object({
    selectedIds: z.array(z.string()).max(64),
    summary: z.string().max(8192).optional(),
  })
  .strict();
type StewardProposal = z.infer<typeof proposalSchema>;
export type ProposalGenerator = (
  segments: Segment[],
  signal?: AbortSignal,
) => Promise<StewardProposal>;
const formationSchema = z
  .object({
    text: z.string().min(1).max(32768),
    semanticRole: z.string().min(1).max(128),
    title: z.string().max(256).optional(),
  })
  .strict();
const interpretationSchema = z
  .object({ text: z.string().min(1).max(65536) })
  .strict();

export class ModelRuntime {
  constructor(
    private readonly generator?: ProposalGenerator,
    private readonly producer = "deterministic",
    readonly invocations = new ModelInvocations(),
  ) {}
  static async fromConfig(config: ModelConfiguration) {
    const invocations = await ModelInvocations.create(config);
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
    );
  }
  get embeddingModel() {
    return this.invocations.profile("query_embedding")?.model;
  }
  async form(text: string, signal?: AbortSignal) {
    const result = await this.invocations.generate(
      "memory_formation",
      text,
      formationSchema,
      signal,
    );
    return {
      ...formationSchema.parse(result.value),
      evidence: result.evidence,
    };
  }
  async interpret(bytes: Uint8Array, mediaType: string, signal?: AbortSignal) {
    const content = mediaType.startsWith("text/")
      ? [
          {
            type: "text" as const,
            text: new TextDecoder("utf-8", { fatal: true }).decode(bytes),
          },
        ]
      : [{ type: "image" as const, image: bytes, mediaType }];
    const result = await this.invocations.generate(
      "material_description",
      content,
      undefined,
      signal,
    );
    return {
      text: interpretationSchema.parse({ text: result.value }).text,
      evidence: result.evidence,
    };
  }
  async embedding(
    text: string,
    model: string,
    signal?: AbortSignal,
  ): Promise<number[]> {
    if (!this.embeddingModel || model !== this.embeddingModel)
      throw new Error("Embedding model is not configured for the Kernel space");
    return (await this.invocations.embedding(text, model, signal)).value;
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
