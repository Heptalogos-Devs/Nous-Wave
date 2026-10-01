import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import type { KernelClient } from "../kernel-client.js";
import { ModelRuntime } from "./runtime.js";
import { invocationSummary } from "./summary.js";
export class ModelMaterialPipeline {
  constructor(
    private readonly kernel: KernelClient,
    private readonly models: ModelRuntime,
  ) {}
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
    const batchSize =
      this.models.invocations.profile("query_embedding")?.embedding
        ?.max_batch_size ?? 64;
    for (let offset = 0; offset < needs.needs.length; offset += batchSize) {
      try {
        const batch = needs.needs.slice(offset, offset + batchSize);
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
