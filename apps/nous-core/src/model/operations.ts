import { Code, ConnectError, type ServiceImpl } from "@connectrpc/connect";
import { ModelService } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { ModelRuntime } from "./runtime.js";
import { ModelMaterialPipeline } from "./material.js";

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
    formFromObservation: async (r, c) => {
      const opts = { signal: c.signal, timeoutMs: c.timeoutMs() };
      const occurrence = await kernel.authority.getOccurrence(
        { subjectId: r.subjectId, id: r.sourceId },
        opts,
      );
      const source = await kernel.authority.materializeEvidence(
        {
          subjectId: r.subjectId,
          reference: { kind: "occurrence", value: r.sourceId },
          maxBytes: 32768n,
        },
        opts,
      );
      let proposal;
      try {
        proposal = await models.form(
          new TextDecoder("utf-8", { fatal: true }).decode(source.content),
          c.signal,
        );
      } catch (error) {
        if (c.signal.aborted) throw error;
        return { degradation: failure("memory_formation_unavailable", error) };
      }
      // The model does not supply identity or evidence fields. Source authority is fixed by the operation.
      const now = new Date();
      const formedAt = {
        seconds: BigInt(Math.floor(now.getTime() / 1000)),
        nanos: (now.getTime() % 1000) * 1_000_000,
      };
      const memory = await kernel.authority.formMemory(
        {
          operationId: crypto.randomUUID(),
          subjectId: r.subjectId,
          input: {
            cognitiveRole: "declarative",
            formationMode: "grounded",
            groundingOccurrenceId: r.sourceId,
            semanticRole: proposal.semanticRole,
            text: proposal.text,
            title: proposal.title,
            epistemicClass: "derived",
            formedAt,
            validTime: {},
            supports: [
              {
                support: {
                  case: "evidence",
                  value: {
                    occurrenceId: r.sourceId,
                    locator: { case: "wholeOccurrence", value: true },
                    supportRole: "interpretation",
                  },
                },
              },
            ],
            aboutness: occurrence.actorEntityRef
              ? [occurrence.actorEntityRef]
              : [],
          },
        },
        opts,
      );
      return { memory, degradation: [] };
    },
    interpretSource: async (r, c) => {
      const opts = { signal: c.signal, timeoutMs: c.timeoutMs() };
      const region = await kernel.authority.getSourceRegion(
        { subjectId: r.subjectId, id: r.sourceId },
        opts,
      );
      const source = await kernel.authority.materializeEvidence(
        {
          subjectId: r.subjectId,
          reference: { kind: "source_region", value: region.sourceRegionId },
          maxBytes: 1048576n,
        },
        opts,
      );
      if (source.partial)
        return {
          degradation: [
            {
              code: "interpretation_source_too_large",
              detail:
                "Select a bounded source region before model interpretation",
            },
          ],
        };
      let text;
      try {
        text = await models.interpret(
          source.content,
          source.mediaType,
          c.signal,
        );
      } catch (error) {
        if (c.signal.aborted) throw error;
        return { degradation: failure("interpretation_unavailable", error) };
      }
      const kind = source.mediaType.startsWith("image/")
        ? "image_description"
        : source.mediaType.startsWith("audio/") ||
            source.mediaType.startsWith("video/")
          ? "transcript"
          : "extracted_text";
      const representation = await kernel.modelMaterial.commitInterpretation(
        {
          subjectId: r.subjectId,
          sourceRegionId: r.sourceId,
          text,
          model: models.invocations.profile("material_description")!.model,
          modelRevision: "configured",
          implementation: "ai-sdk-7",
          kind,
        },
        opts,
      );
      return { representation, degradation: [] };
    },
  };
}
