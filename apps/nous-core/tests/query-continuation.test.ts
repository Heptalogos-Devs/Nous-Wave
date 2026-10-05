import { coreExecutionSchema } from "../src/configuration-catalog.js";
import { expect, it, vi } from "vitest";
import { create } from "@bufbuild/protobuf";
import {
  QueryExprSchema,
  QueryRequestSchema,
  QueryResponseSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { QueryOrchestrator } from "../src/query/orchestrator.js";
import type { KernelClient } from "../src/kernel-client.js";
import type { ModelRuntime } from "../src/model/runtime.js";
import type { ResourceRegistry } from "../src/resources/registry.js";

it("finishes a Resource ticket without invoking required rerank for identity-only intent", async () => {
  const response = create(QueryResponseSchema, {
    status: "complete",
    hits: [{ text: "first" }, { text: "second" }],
  });
  const finalize = vi.fn(async () => response);
  const release = vi.fn(async () => ({}));
  const rerank = vi.fn(async () => {
    throw new Error("Empty intent must not enter the reranker");
  });
  const kernel = {
    execution: coreExecutionSchema.parse(undefined),
    queryWorkflow: {
      prepareQuery: async () => ({
        preparationToken: "prepared-token",
        boundQuery: "{}",
        embeddingRequired: false,
        embeddingText: "",
      }),
      query: async () => ({ response, validationTicket: "resource-ticket" }),
      finalizeQuery: finalize,
      releaseQuery: release,
    },
  } as unknown as KernelClient;
  const models = {
    invocations: {
      profile: () => ({ protocol: "rerank-v1", model: "required-model" }),
      requirement: () => "required",
      rerank,
    },
  } as unknown as ModelRuntime;
  await new QueryOrchestrator(kernel, models, {} as ResourceRegistry).execute(
    create(QueryRequestSchema, {
      subjectId: "subject",
      expression: create(QueryExprSchema, { operation: "atom" }),
    }),
  );
  expect(rerank).not.toHaveBeenCalled();
  expect(finalize).toHaveBeenCalledOnce();
  expect(release).toHaveBeenCalledTimes(2);
});

it("prepares before embedding and sends one complete representation for an expression tree", async () => {
  const representation =
    "Intent:\nALL OF: Alice's release; Nous Wave tasks\n\nCurrent work:\nRelease stabilization";
  const embedding = vi.fn(async (texts: string[]) => ({
    value: texts.map(() => [1, 0]),
  }));
  const execute = vi.fn(async () => ({
    response: create(QueryResponseSchema, { status: "complete" }),
  }));
  const release = vi.fn(async () => ({}));
  const prepare = vi.fn(async () => ({
    preparationToken: "prepared",
    boundQuery: '{"prepared":true}',
    embeddingText: representation,
    embeddingRequired: true,
    rerankRequirement: "forbidden",
  }));
  const kernel = {
    execution: coreExecutionSchema.parse(undefined),
    materialWorkflow: {
      getEmbeddingConfig: async () => ({
        spaceHash: "space",
        producerHash: "producer",
        model: "fake",
      }),
    },
    queryWorkflow: {
      prepareQuery: prepare,
      query: execute,
      releaseQuery: release,
    },
  } as unknown as KernelClient;
  const models = {
    embeddingModel: {},
    invocations: {
      embeddingBatch: embedding,
      profile: () => ({ model: "configured-reranker" }),
      requirement: (role: string) =>
        role === "query_rerank" ? "required" : "optional",
      rerank: vi.fn(async () => {
        throw new Error("forbidden rerank invoked");
      }),
    },
  } as unknown as ModelRuntime;
  const expression = create(QueryExprSchema, {
    operation: "all",
    children: [
      {
        operation: "atom",
        cues: [{ cue: { case: "text", value: "Alice's release" } }],
      },
      {
        operation: "atom",
        cues: [{ cue: { case: "text", value: "Nous Wave tasks" } }],
      },
    ],
  });
  const result = await new QueryOrchestrator(
    kernel,
    models,
    {} as ResourceRegistry,
  ).execute(
    create(QueryRequestSchema, {
      subjectId: "subject",
      expression,
      capabilities: { rerank: "forbidden" },
    }),
  );
  expect(prepare).toHaveBeenCalledOnce();
  expect(embedding).toHaveBeenCalledOnce();
  expect(embedding.mock.calls[0]?.[0]).toEqual([representation]);
  expect(execute).toHaveBeenCalledWith(
    expect.objectContaining({
      subjectId: "subject",
      preparationToken: "prepared",
      embeddings: [expect.objectContaining({ text: representation })],
    }),
    {},
  );
  expect(result.boundQuery).toBe('{"prepared":true}');
  expect(release).toHaveBeenCalledOnce();
});
