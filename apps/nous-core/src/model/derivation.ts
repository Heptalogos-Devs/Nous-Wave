import type { CallOptions } from "@connectrpc/connect";
import type { DeriveMaterialRequest } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import type { ModelRuntime } from "./runtime.js";
import type { ModelInvocationEvidence } from "./invocations.js";
import type { DerivedRepresentation } from "@nous-wave/protocol/nous/wave/v1alpha1/management_pb.js";
import { sampleVideo } from "./video.js";
import { canonicalDigest } from "./prompts.js";
import { invocationSummary } from "./summary.js";

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
  const region = await kernel.authority.getSourceRegion(
    { subjectId: request.subjectId, id: request.sourceRegionId },
    options,
  );
  const artifact = await kernel.authority.getArtifact(
    { subjectId: request.subjectId, id: region.artifactId },
    options,
  );
  const isVideo = artifact.mediaType.toLowerCase().startsWith("video/");
  let source;
  if (isVideo && region.coordinateKind === "whole_artifact") {
    if (artifact.byteLength > BigInt(models.video.max_source_bytes))
      return {
        representations: [],
        degradation: [
          {
            code: "video_source_too_large",
            detail: "Video exceeds configured preprocessing byte bound",
          },
        ],
      };
    const chunks: Uint8Array[] = [];
    let length = 0;
    for await (const chunk of kernel.artifacts.downloadArtifact(
      { subjectId: request.subjectId, artifactId: region.artifactId },
      options,
    )) {
      length += chunk.content.length;
      if (length > models.video.max_source_bytes)
        throw new Error("Video stream exceeds configured byte bound");
      chunks.push(chunk.content);
    }
    source = {
      content: Buffer.concat(chunks),
      mediaType: artifact.mediaType,
      partial: false,
    };
  } else
    source = await kernel.authority.materializeEvidence(
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
  const invocations: ReturnType<typeof invocationSummary>[] = [];
  const commit = async (
    text: string,
    kind: string,
    inputs: {
      ordinal: number;
      reference: { kind: string; value: string };
      role: string;
    }[],
    evidence?: ModelInvocationEvidence,
    quality: Record<string, unknown> = {},
    preprocessingDigest?: string,
  ) => {
    if (evidence) invocations.push(invocationSummary(evidence));
    const representation = await kernel.modelMaterial.commitInterpretation(
      {
        subjectId: request.subjectId,
        text,
        kind,
        inputs,
        strategy,
        quality: JSON.parse(
          JSON.stringify({
            ...quality,
            ...(evidence
              ? {
                  model_profile_digest: evidence.profileDigest,
                  role_config_digest: evidence.configDigest,
                  latency_ms: evidence.latencyMs,
                }
              : {}),
          }),
        ) as import("@bufbuild/protobuf").JsonObject,
        supersedes:
          representations.length === 0 ? request.supersedes : undefined,
        producer: {
          providerClass: evidence?.protocol ?? "deterministic",
          operation:
            kind === "image_description" || kind === "scene_description"
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
          preprocessingIdentity: `${evidence ? (evidence.promptId ?? "standard-audio-transcription") : "verified-utf8"}/${strategy}`,
          preprocessingRevision: evidence?.promptDigest ?? "1",
          configDigest: preprocessingDigest
            ? canonicalDigest({
                role: evidence?.configDigest,
                preprocessing: preprocessingDigest,
              })
            : (evidence?.configDigest ?? "utf8-fatal-v1"),
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
  if (
    !textual &&
    !mime.startsWith("image/") &&
    !mime.startsWith("audio/") &&
    !mime.startsWith("video/")
  )
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
    if (mime.startsWith("video/")) {
      const samples = await sampleVideo(
        source.content,
        models.video,
        models.credentialEnvironments,
        Boolean(models.invocations.profile("speech_transcription")),
        signal,
      );
      let transcript: string | undefined;
      const sceneInputs = [...inputs];
      const degradation: { code: string; detail: string }[] = [];
      if (samples.audio) {
        try {
          const result = await models.invocations.transcription(
            samples.audio,
            signal,
          );
          const representation = await commit(
            result.value,
            "transcript",
            inputs,
            result.evidence,
            samples.quality,
            samples.preprocessingDigest,
          );
          transcript = representation.text;
          sceneInputs.push({
            ordinal: 1,
            reference: {
              kind: "derived_representation",
              value: representation.representationId,
            },
            role: "transcript",
          });
        } catch {
          if (signal?.aborted) throw signal.reason;
          degradation.push({
            code: "video_transcription_unavailable",
            detail: "Scene description uses frames without a transcript",
          });
        }
      } else if (samples.hasAudio)
        degradation.push({
          code: "video_audio_not_interpreted",
          detail: "Audio is not included in the scene description",
        });
      const description = await models.describeScene(
        samples.frames,
        transcript,
        strategy === "direct_structured",
        signal,
      );
      const selected = await commit(
        description.text,
        strategy === "direct_structured"
          ? "structured_interpretation"
          : "scene_description",
        sceneInputs,
        description.evidence,
        samples.quality,
        samples.preprocessingDigest,
      );
      if (strategy === "describe_then_structure") {
        try {
          const result = await models.structure(selected.text!, signal);
          await commit(
            result.text,
            "structured_interpretation",
            [
              {
                ordinal: 0,
                reference: {
                  kind: "derived_representation",
                  value: selected.representationId,
                },
                role: "description",
              },
            ],
            result.evidence,
          );
        } catch {
          if (signal?.aborted) throw signal.reason;
          degradation.push({
            code: "material_structuring_failed",
            detail: "The committed scene description is retained",
          });
        }
      }
      return {
        representations,
        invocations,
        selectedRepresentationId: selected.representationId,
        degradation,
      };
    }
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
        invocations,
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
      invocations,
      selectedRepresentationId: description.representationId,
      degradation: [],
    };
  } catch (error) {
    if (signal?.aborted) throw error;
    return {
      representations,
      invocations,
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
