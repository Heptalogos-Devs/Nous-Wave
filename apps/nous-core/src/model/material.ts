import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import {
  QueryEmbeddingSchema,
  type QueryEmbedding,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/model_pb.js";
import type {
  QueryExpr,
  QueryRequest,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { ModelRuntime } from "./runtime.js";
import { canonicalDigest } from "./prompts.js";
import { invocationSummary } from "./summary.js";

export class ModelMaterialPipeline {
  private readonly queryVectors = new Map<string, number[]>();
  constructor(
    private readonly kernel: KernelClient,
    private readonly models: ModelRuntime,
  ) {}
  async query(input: QueryRequest, options: CallOptions = {}) {
    const material: QueryEmbedding[] = [];
    const invocations: ReturnType<typeof invocationSummary>[] = [];
    let failure: string | undefined;
    const texts = new Set<string>();
    const collect = (node: QueryExpr) => {
      const text = node.cues
        .map((c) =>
          c.cue.case === "text" || c.cue.case === "concept" ? c.cue.value : "",
        )
        .filter(Boolean)
        .join(" ")
        .trim();
      if (text) texts.add(text);
      node.children.forEach(collect);
    };
    if (input.expression) collect(input.expression);
    if (texts.size > 64)
      throw new ConnectError(
        "Query material bound exceeded",
        Code.ResourceExhausted,
      );
    if (texts.size) {
      try {
        if (!this.models.embeddingModel)
          throw new ConnectError(
            "Query embedding role unavailable",
            Code.FailedPrecondition,
          );
        const config = await this.kernel.modelMaterial.getEmbeddingConfig(
          {},
          options,
        );
        const keys = new Map(
          [...texts].map((text) => [
            text,
            canonicalDigest({
              text,
              space: config.spaceHash,
              producer: config.producerHash,
            }),
          ]),
        );
        const missing = [...texts].filter(
          (text) => !this.queryVectors.has(keys.get(text)!),
        );
        if (missing.length) {
          const vectors = await this.models.invocations.embeddingBatch(
            missing,
            config.model,
            options.signal ?? undefined,
          );
          invocations.push(invocationSummary(vectors.evidence));
          missing.forEach((text, index) =>
            this.queryVectors.set(keys.get(text)!, vectors.value[index]!),
          );
        }
        for (const text of texts)
          material.push(
            create(QueryEmbeddingSchema, {
              text,
              spaceHash: config.spaceHash,
              producerHash: config.producerHash,
              vector: this.queryVectors.get(keys.get(text)!)!,
            }),
          );
        while (this.queryVectors.size > 128)
          this.queryVectors.delete(this.queryVectors.keys().next().value!);
      } catch (error) {
        if (options.signal?.aborted) throw error;
        if (
          this.models.invocations.requirement("query_embedding") === "required"
        )
          throw new ConnectError(
            "Required query embedding unavailable",
            Code.FailedPrecondition,
          );
        failure =
          error instanceof Error ? error.message : "Embedding unavailable";
      }
    }
    const result = await this.kernel.authority.query(
      { query: input, embeddings: material },
      options,
    );
    result.invocations.push(
      ...invocations.map((summary) => ({
        $typeName: "nous.wave.v1alpha1.ModelInvocationSummary" as const,
        ...summary,
      })),
    );
    if (failure) {
      result.degradation.push({
        $typeName: "nous.wave.v1alpha1.Degradation",
        code: "query_embedding_unavailable",
        detail: failure,
      });
      if (result.status === "complete") result.status = "degraded";
    }
    return result;
  }
  async prepare(subjectId: string, limit: number, options: CallOptions = {}) {
    const needs = await this.kernel.modelMaterial.listEmbeddingNeeds(
      { subjectId, limit },
      options,
    );
    if (!needs.config)
      throw new ConnectError(
        "Embedding configuration unavailable",
        Code.FailedPrecondition,
      );
    let committed = 0;
    const invocations: ReturnType<typeof invocationSummary>[] = [];
    for (let offset = 0; offset < needs.needs.length; offset += 64) {
      try {
        const batch = needs.needs.slice(offset, offset + 64);
        const vectors = await this.models.invocations.embeddingBatch(
          batch.map((need) => need.text),
          needs.config.model,
          options.signal ?? undefined,
        );
        invocations.push(invocationSummary(vectors.evidence));
        for (const [index, need] of batch.entries()) {
          await this.kernel.modelMaterial.commitEmbedding(
            {
              subjectId,
              reference: need.reference,
              material: {
                text: need.text,
                spaceHash: needs.config.spaceHash,
                producerHash: needs.config.producerHash,
                vector: vectors.value[index]!,
              },
            },
            options,
          );
          committed++;
        }
      } catch (error) {
        if (options.signal?.aborted) throw error;
        return {
          committed,
          invocations,
          degradation: [
            {
              code: "embedding_batch_incomplete",
              detail:
                error instanceof Error
                  ? error.message
                  : "Embedding material failed",
            },
          ],
        };
      }
    }
    return { committed, degradation: [], invocations };
  }
}
