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
  expect(release).toHaveBeenCalledOnce();
});
