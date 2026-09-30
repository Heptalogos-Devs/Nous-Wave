import type { CallOptions } from "@connectrpc/connect";
import type { DeriveMaterialRequest } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import type { ModelRuntime } from "./runtime.js";
import type { ModelInvocationEvidence } from "./invocations.js";
import type { DerivedRepresentation } from "@nous-wave/protocol/nous/wave/v1alpha1/management_pb.js";

export async function deriveMaterial(
  kernel: KernelClient,
  models: ModelRuntime,
  request: DeriveMaterialRequest,
  options: CallOptions,
) {
  const strategy = request.strategy ?? models.materialStrategy;
  if (
    ![
      "description_only",
      "direct_structured",
      "describe_then_structure",
    ].includes(strategy)
  )
    throw new Error("Unknown material strategy");
  if (
    request.target &&
    !["description", "structured", "automatic"].includes(request.target)
  )
    throw new Error("Unknown derivation target");
  if (
    (request.target === "description" && strategy !== "description_only") ||
    (request.target === "structured" && strategy === "description_only")
  )
    throw new Error("Derivation target conflicts with strategy");
  const source = await kernel.authority.materializeEvidence(
    {
      subjectId: request.subjectId,
      reference: { kind: "source_region", value: request.sourceRegionId },
      maxBytes: 1048576n,
    },
    options,
  );
  if (source.partial)
    return {
      representations: [],
      degradation: [
        {
          code: "derivation_source_too_large",
          detail: "Select a bounded source region",
        },
      ],
    };
  const representations: DerivedRepresentation[] = [];
  const commit = async (
    text: string,
    kind: string,
    inputs: {
      ordinal: number;
      reference: { kind: string; value: string };
      role: string;
    }[],
    evidence?: ModelInvocationEvidence,
  ) => {
    const representation = await kernel.modelMaterial.commitInterpretation(
      {
        subjectId: request.subjectId,
        text,
        kind,
        inputs,
        strategy,
        supersedes:
          representations.length === 0 ? request.supersedes : undefined,
        producer: {
          providerClass: evidence?.protocol ?? "deterministic",
          operation:
            kind === "image_description"
              ? "image_interpretation"
              : kind === "transcript"
                ? "speech_transcription"
                : kind === "extracted_text"
                  ? "document_extraction"
                  : "text_interpretation",
          implementation: evidence
            ? "ai-sdk@7.0.102/openai@4.0.67"
            : "verified-utf8-decoding-v1",
          modelIdentity: evidence?.model,
          modelRevision: evidence?.modelRevision,
          preprocessingIdentity: `${evidence?.promptId ?? "verified-utf8"}/${strategy}`,
          preprocessingRevision: evidence?.promptDigest ?? "1",
          configDigest: evidence?.configDigest ?? "utf8-fatal-v1",
        },
      },
      options,
    );
    representations.push(representation);
    return representation;
  };
  const inputs = [
    {
      ordinal: 0,
      reference: { kind: "source_region", value: request.sourceRegionId },
      role: "source",
    },
  ];
  const mime = source.mediaType.split(";")[0]?.trim().toLowerCase() ?? "";
  const textual = mime.startsWith("text/") || mime === "application/json";
  const signal = options.signal ?? undefined;
  if (mime.startsWith("video/"))
    return {
      representations: [],
      degradation: [
        {
          code: "video_preprocessor_required",
          detail: "Bounded FFmpeg preprocessing has not been configured",
        },
      ],
    };
  if (!textual && !mime.startsWith("image/") && !mime.startsWith("audio/"))
    return {
      representations: [],
      degradation: [
        {
          code: "material_modality_unsupported",
          detail: "No standard derivation adapter for this media type",
        },
      ],
    };
  if (mime.startsWith("audio/") && strategy === "direct_structured")
    return {
      representations: [],
      degradation: [
        {
          code: "audio_direct_structured_unsupported",
          detail: "Speech transcription must precede textual structuring",
        },
      ],
    };
  try {
    if (strategy === "direct_structured") {
      const result = await models.structure(
        textual
          ? new TextDecoder("utf-8", { fatal: true }).decode(source.content)
          : { bytes: source.content, mediaType: mime },
        signal,
      );
      const structured = await commit(
        result.text,
        "structured_interpretation",
        inputs,
        result.evidence,
      );
      return {
        representations,
        selectedRepresentationId: structured.representationId,
        degradation: [],
      };
    }
    const result = textual
      ? {
          text: new TextDecoder("utf-8", { fatal: true }).decode(
            source.content,
          ),
          evidence: undefined,
        }
      : mime.startsWith("audio/")
        ? await models.invocations
            .transcription(source.content, signal)
            .then((r) => ({ text: r.value, evidence: r.evidence }))
        : await models.interpret(source.content, mime, signal);
    const description = await commit(
      result.text,
      textual
        ? "extracted_text"
        : mime.startsWith("audio/")
          ? "transcript"
          : "image_description",
      inputs,
      result.evidence,
    );
    if (strategy === "describe_then_structure") {
      const structured = await models.structure(description.text!, signal);
      await commit(
        structured.text,
        "structured_interpretation",
        [
          {
            ordinal: 0,
            reference: {
              kind: "derived_representation",
              value: description.representationId,
            },
            role: "description",
          },
        ],
        structured.evidence,
      );
    }
    return {
      representations,
      selectedRepresentationId: description.representationId,
      degradation: [],
    };
  } catch (error) {
    if (signal?.aborted) throw error;
    return {
      representations,
      selectedRepresentationId: representations[0]?.representationId,
      degradation: [
        {
          code: representations.length
            ? "material_structuring_failed"
            : "material_derivation_failed",
          detail:
            error instanceof Error ? error.message : "Derivation unavailable",
        },
      ],
    };
  }
}
