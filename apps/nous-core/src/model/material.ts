import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import {
  QueryEmbeddingSchema,
  type QueryEmbedding,
} from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/model_pb.js";
import type {
  QueryExpr,
  QueryRequest,
  QueryResponse,
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
    const intent = input.expression ? positiveIntent(input.expression) : "";
    const profile = this.models.invocations.profile("query_rerank");
    if (
      intent &&
      !profile &&
      this.models.invocations.requirement("query_rerank") === "required"
    )
      throw new ConnectError(
        "Required query rerank role unavailable",
        Code.FailedPrecondition,
      );
    const prepared = await this.kernel.authority.query(
      {
        query: input,
        embeddings: material,
        validatedCandidateLimit: intent && profile ? 64 : undefined,
      },
      options,
    );
    if (!prepared.response)
      throw new ConnectError("Kernel query response missing", Code.Internal);
    let result = prepared.response;
    const ticket = prepared.validationTicket;
    if (ticket) {
      const candidates = result.hits
        .filter((hit) => hit.text?.trim())
        .slice(0, 64);
      let order: {
        reference: NonNullable<(typeof candidates)[number]["reference"]>;
        score: number;
      }[] = [];
      let status = "NOT_RUN",
        reason = "fewer_than_two_textual_candidates",
        providerResults = 0;
      try {
        if (candidates.length >= 2) {
          try {
            const ranking = await this.models.invocations.rerank(
              intent,
              candidates.map((hit) => hit.text!),
              candidates.length,
              options.signal ?? undefined,
            );
            invocations.push(invocationSummary(ranking.evidence));
            order = ranking.value.map((item) => {
              const reference = candidates[item.index]!.reference;
              if (!reference)
                throw new ConnectError(
                  "Kernel candidate reference missing",
                  Code.Internal,
                );
              return { reference, score: item.relevance_score };
            });
            status = "PASS";
            reason = "";
            providerResults = order.length;
          } catch (error) {
            if (options.signal?.aborted) throw error;
            if (
              this.models.invocations.requirement("query_rerank") === "required"
            )
              throw new ConnectError(
                "Required rerank invocation unavailable",
                Code.FailedPrecondition,
              );
            status = "FAIL";
            reason = "validated_baseline_fallback";
          }
        }
        result = await this.kernel.authority.finalizeQuery(
          { subjectId: input.subjectId, validationTicket: ticket, order },
          options,
        );
        result.rerank = {
          $typeName: "nous.wave.v1alpha1.RerankMechanismSummary",
          mechanism: "model-order-with-baseline-tail-v1",
          status,
          reason,
          candidateCount: candidates.length,
          providerResults,
          protocol: profile?.protocol ?? "",
          model: profile?.model ?? "",
        };
        if (status === "FAIL") {
          result.degradation.push({
            $typeName: "nous.wave.v1alpha1.Degradation",
            code: "query_rerank_unavailable",
            detail: "Validated baseline retained after model rerank failure",
          });
          if (result.status === "complete") result.status = "degraded";
        }
      } finally {
        await this.kernel.authority
          .releaseQuery(
            { subjectId: input.subjectId, validationTicket: ticket },
            { timeoutMs: 5000 },
          )
          .catch(() => {});
      }
    } else
      result.rerank = skippedRerank(
        intent ? "role_not_configured" : "no_positive_textual_intent",
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

function positiveIntent(node: QueryExpr): string {
  const own = node.cues
    .map((cue) =>
      cue.cue.case === "text" || cue.cue.case === "concept"
        ? cue.cue.value.trim()
        : "",
    )
    .filter(Boolean)
    .join(" ");
  if (node.operation === "atom") return own;
  const children = node.children.map(positiveIntent).filter(Boolean);
  if (!children.length) return "";
  return `${node.operation === "all" ? "ALL OF" : "ANY OF"}: ${children.map((text) => `(${text})`).join("; ")}`;
}
function skippedRerank(reason: string): NonNullable<QueryResponse["rerank"]> {
  return {
    $typeName: "nous.wave.v1alpha1.RerankMechanismSummary",
    mechanism: "model-order-with-baseline-tail-v1",
    status: "NOT_RUN",
    reason,
    candidateCount: 0,
    providerResults: 0,
    protocol: "",
    model: "",
  };
}
