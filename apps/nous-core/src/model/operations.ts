import { Code, ConnectError, type ServiceImpl } from "@connectrpc/connect";
import { ModelService } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { ModelRuntime } from "./runtime.js";
import { ModelMaterialPipeline } from "./material.js";
import { deriveMaterial } from "./derivation.js";
import { formObservation } from "./formation.js";

function failure(code: string, error: unknown) {
  return [
    {
      code,
      detail:
        error instanceof Error ? error.message : "Model operation unavailable",
    },
  ];
}
export function modelOperations(
  kernel: KernelClient,
  models: ModelRuntime,
): ServiceImpl<typeof ModelService> {
  const material = new ModelMaterialPipeline(kernel, models);
  return {
    prepareEmbeddings: async (r, c) => {
      if (r.limit < 1 || r.limit > 256)
        throw new ConnectError(
          "Embedding batch must be 1..256",
          Code.InvalidArgument,
        );
      try {
        return await material.prepare(r.subjectId, r.limit, {
          signal: c.signal,
          timeoutMs: c.timeoutMs(),
        });
      } catch (error) {
        if (c.signal.aborted) throw error;
        return {
          committed: 0,
          degradation: failure("embedding_preparation_failed", error),
        };
      }
    },
    formFromObservation: (r, c) =>
      formObservation(kernel, models, r, {
        signal: c.signal,
        timeoutMs: c.timeoutMs(),
      }),
    deriveMaterial: (r, c) =>
      deriveMaterial(kernel, models, r, {
        signal: c.signal,
        timeoutMs: c.timeoutMs(),
      }),
  };
}
