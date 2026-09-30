import { Code, ConnectError, type ServiceImpl } from "@connectrpc/connect";
import { ModelService } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { ModelRuntime } from "./runtime.js";
import { ModelMaterialPipeline } from "./material.js";
import { deriveMaterial } from "./derivation.js";

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
        { subjectId: r.subjectId, id: r.occurrenceId },
        opts,
      );
      let representationId = r.representationId;
      if (!representationId && occurrence.artifactId) {
        const artifact = await kernel.authority.getArtifact(
          { subjectId: r.subjectId, id: occurrence.artifactId },
          opts,
        );
        const mime =
          artifact.mediaType.split(";")[0]?.trim().toLowerCase() ?? "";
        const kind = mime.startsWith("image/")
          ? "image_description"
          : mime.startsWith("audio/")
            ? "transcript"
            : mime.startsWith("video/")
              ? "scene_description"
              : "extracted_text";
        const available = await kernel.authority.listDerivedRepresentations(
          {
            subjectId: r.subjectId,
            artifactId: occurrence.artifactId,
            kind,
            limit: 1,
          },
          opts,
        );
        representationId = available.items[0]?.representationId;
        if (
          !representationId &&
          !mime.startsWith("text/") &&
          mime !== "application/json"
        )
          return {
            degradation: [
              {
                code: "material_representation_required",
                detail: "Derive a textual representation before formation",
              },
            ],
          };
      }
      const source = await kernel.authority.materializeEvidence(
        {
          subjectId: r.subjectId,
          reference: representationId
            ? { kind: "derived_representation", value: representationId }
            : { kind: "occurrence", value: r.occurrenceId },
          maxBytes: 32768n,
        },
        opts,
      );
      if (source.partial)
        return {
          degradation: [
            {
              code: "formation_source_too_large",
              detail: "Select a bounded textual representation",
            },
          ],
        };
      if (
        !source.mediaType.toLowerCase().startsWith("text/") &&
        source.mediaType.split(";")[0]?.trim().toLowerCase() !==
          "application/json"
      )
        return {
          degradation: [
            {
              code: "material_representation_required",
              detail: "Formation requires textual material",
            },
          ],
        };
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
            groundingOccurrenceId: r.occurrenceId,
            semanticRole: proposal.semanticRole,
            text: proposal.text,
            title: proposal.title,
            epistemicClass: "derived",
            producer: {
              providerClass: proposal.evidence.protocol,
              operation: "memory_formation_text",
              implementation: "ai-sdk@7.0.102/openai@4.0.67",
              modelIdentity: proposal.evidence.model,
              modelRevision: proposal.evidence.modelRevision,
              preprocessingIdentity: proposal.evidence.promptId!,
              preprocessingRevision: proposal.evidence.promptDigest!,
              configDigest: proposal.evidence.configDigest,
            },
            formedAt,
            validTime: {},
            supports: [
              {
                support: {
                  case: "evidence",
                  value: {
                    occurrenceId: r.occurrenceId,
                    locator: representationId
                      ? {
                          case: "derivedRepresentationId",
                          value: representationId,
                        }
                      : { case: "wholeOccurrence", value: true },
                    supportRole: "interpretation",
                  },
                },
              },
            ],
            aboutness: [],
          },
        },
        opts,
      );
      return { memory, degradation: [] };
    },
    deriveMaterial: (r, c) =>
      deriveMaterial(kernel, models, r, {
        signal: c.signal,
        timeoutMs: c.timeoutMs(),
      }),
  };
}
