// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { Code, ConnectError } from "@connectrpc/connect";
import { executionOptions, type ExecutionOptions } from "../execution.js";
import { create, fromJson, toJson, type JsonValue } from "@bufbuild/protobuf";
import { FormMemoryRequestSchema } from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { type FormationRequest } from "@nous-wave/protocol/nous/wave/v1alpha1/model_pb.js";
import type { KernelClient } from "../kernel-client.js";
import type { ModelRuntime } from "./runtime.js";
import {
  failedExecutionTelemetry,
  type ModelRoleSnapshot,
} from "./invocations.js";
import { canonicalDigest } from "../digest.js";
import { z } from "zod";
import { modelProducer } from "./producer.js";

const snapshotSchema = z.strictObject({
  cognitive_formed_at: z.string(),
  model: z.unknown(),
  sourceMaxBytes: z.number().int().positive(),
  representationId: z.string().optional(),
  candidates: z
    .array(
      z.strictObject({
        key: z.string(),
        surface: z.string(),
        entityRef: z.string(),
      }),
    )
    .max(128),
});
const proposalSchema = z.strictObject({
  request: z.unknown(),
});
const outcomeSchema = z.union([
  z.strictObject({ purged: z.literal(true) }),
  z.strictObject({
    revisionId: z.string(),
  }),
]);

export async function formObservation(
  kernel: KernelClient,
  models: ModelRuntime,
  r: FormationRequest,
  options: ExecutionOptions,
) {
  const calls = executionOptions(kernel.execution.opportunity, options);
  options = calls;
  const { opportunity } = calls;
  if (
    !z.string().uuid().safeParse(r.operationId).success ||
    r.operationId === "00000000-0000-0000-0000-000000000000"
  )
    throw new ConnectError(
      "Caller-stable operation_id is required",
      Code.InvalidArgument,
    );
  r.operationId = r.operationId.toLowerCase();
  r.subjectId = r.subjectId.toLowerCase();
  r.occurrenceId = r.occurrenceId.toLowerCase();
  if (r.representationId) r.representationId = r.representationId.toLowerCase();
  const mode = r.aboutnessMode || "select_from_resolved_mentions";
  if (
    !["explicit", "select_from_resolved_mentions", "none"].includes(mode) ||
    (mode !== "explicit" && r.explicitAboutness.length)
  )
    throw new ConnectError("Invalid aboutness policy", Code.InvalidArgument);
  if (
    new Set(r.explicitAboutness).size !== r.explicitAboutness.length ||
    r.explicitAboutness.length > 128
  )
    throw new ConnectError(
      "Duplicate/oversized aboutness",
      Code.InvalidArgument,
    );
  if (
    r.explicitTags.length > 128 ||
    r.explicitTags.some((tag) => !z.string().uuid().safeParse(tag).success)
  )
    throw new ConnectError(
      "Invalid explicit Tag identities",
      Code.InvalidArgument,
    );
  r.explicitTags = [
    ...new Set(r.explicitTags.map((tag) => tag.toLowerCase())),
  ].sort();
  const identity = {
    subjectId: r.subjectId,
    owner: "memory",
    operationKey: r.operationId,
    semanticDigest: canonicalDigest({
      subject: r.subjectId,
      occurrence: r.occurrenceId,
      representation: r.representationId,
      mode,
      aboutness: [...r.explicitAboutness].sort(),
      explicitTags: r.explicitTags,
    }),
  };
  const replay = async (text: string) => {
    const outcome = outcomeSchema.parse(JSON.parse(text));
    if ("purged" in outcome)
      throw new ConnectError("Formation outcome was purged", Code.NotFound);
    const memory = await kernel.memory.getMemoryRevision(
      { subjectId: r.subjectId, id: outcome.revisionId },
      options,
    );
    return {
      memory,
      degradation: [],
    };
  };
  const previous = await kernel.modelWorkflow.findWorkflow(identity, options);
  if (previous.outcomeJson) return replay(previous.outcomeJson);
  let snapshotText = previous.snapshotJson;
  if (!snapshotText) {
    const model = models.invocations.snapshot("memory_formation");
    const occurrence = await kernel.material.getOccurrence(
      { subjectId: r.subjectId, id: r.occurrenceId },
      options,
    );
    let representationId = r.representationId;
    if (!representationId && occurrence.artifactId) {
      const artifact = await kernel.material.getArtifact(
        { subjectId: r.subjectId, id: occurrence.artifactId },
        options,
      );
      const mime = artifact.mediaType.split(";")[0]!.trim().toLowerCase();
      const kind = mime.startsWith("image/")
        ? "image_description"
        : mime.startsWith("audio/")
          ? models.audio.input_mode === "direct"
            ? "audio_description"
            : "transcript"
          : mime.startsWith("video/")
            ? "scene_description"
            : "extracted_text";
      const available = await kernel.material.listDerivedRepresentations(
        {
          subjectId: r.subjectId,
          artifactId: occurrence.artifactId,
          kind,
          limit: 1,
        },
        options,
      );
      representationId = available.items[0]?.representationId;
    }
    const candidates =
      mode === "select_from_resolved_mentions"
        ? (
            await kernel.materialWorkflow.getResolvedMentions(
              { subjectId: r.subjectId, occurrenceId: r.occurrenceId },
              options,
            )
          ).candidates.map((candidate) => ({
            key: candidate.key,
            surface: candidate.surface,
            entityRef: candidate.entityRef,
          }))
        : [];
    snapshotText = JSON.stringify({
      model,
      representationId,
      candidates,
      sourceMaxBytes: models.materialInputs.formation_source_max_bytes,
    });
  }
  const reservation = await kernel.modelWorkflow.reserveWorkflow(
    {
      ...identity,
      snapshotJson: snapshotText,
      leaseSeconds: opportunity.leaseSeconds,
    },
    options,
  );
  if (reservation.outcomeJson) return replay(reservation.outcomeJson);
  if (reservation.busy || !reservation.leaseToken)
    throw new ConnectError(
      "Formation workflow is busy; retry the same operation ID",
      Code.Aborted,
    );
  const lease = {
    subjectId: r.subjectId,
    owner: "memory",
    operationKey: r.operationId,
    leaseToken: reservation.leaseToken,
  };
  try {
    let proposed = reservation.proposalJson
      ? proposalSchema.parse(JSON.parse(reservation.proposalJson))
      : undefined;
    if (!proposed) {
      const snapshot = snapshotSchema.parse(
        JSON.parse(reservation.snapshotJson),
      );
      const source = await kernel.material.materializeEvidence(
        {
          subjectId: r.subjectId,
          reference: snapshot.representationId
            ? {
                kind: "derived_representation",
                value: snapshot.representationId,
              }
            : { kind: "occurrence", value: r.occurrenceId },
          maxBytes: BigInt(snapshot.sourceMaxBytes),
        },
        options,
      );
      if (source.partial)
        throw new ConnectError(
          "Formation source is too large; select a bounded representation",
          Code.ResourceExhausted,
        );
      const mime = source.mediaType.split(";")[0]!.toLowerCase();
      if (!mime.startsWith("text/") && mime !== "application/json")
        throw new ConnectError(
          "material_representation_required",
          Code.FailedPrecondition,
        );
      const text = new TextDecoder("utf-8", { fatal: true }).decode(
        source.content,
      );
      const result = await models.form(
        JSON.stringify({
          evidenceText: text,
          resolvedEntityCandidates: snapshot.candidates,
          aboutnessMode: mode,
        }),
        options.signal ?? undefined,
        snapshot.model as ModelRoleSnapshot,
      );
      await kernel.modelWorkflow.saveWorkflow(
        { ...lease, executionTelemetryJson: JSON.stringify(result.execution) },
        opportunity.cleanup(),
      );
      if (
        new Set(result.selectedEntityKeys).size !==
          result.selectedEntityKeys.length ||
        result.selectedEntityKeys.some(
          (key) =>
            !snapshot.candidates.some((candidate) => candidate.key === key),
        )
      )
        throw new ConnectError(
          "Model selected an invalid entity candidate",
          Code.FailedPrecondition,
        );
      const aboutness =
        mode === "explicit"
          ? r.explicitAboutness
          : mode === "none"
            ? []
            : [
                ...new Set(
                  snapshot.candidates
                    .filter((candidate) =>
                      result.selectedEntityKeys.includes(candidate.key),
                    )
                    .map((candidate) => candidate.entityRef),
                ),
              ];
      const request = create(FormMemoryRequestSchema, {
        operationId: r.operationId,
        subjectId: r.subjectId,
        input: {
          cognitiveRole: "declarative",
          formationMode: "grounded",
          groundingOccurrenceId: r.occurrenceId,
          semanticRole: result.semanticRole,
          text: result.text,
          title: result.title ?? undefined,
          epistemicClass: "derived",
          aboutness,
          tags: r.explicitTags,
          producer: modelProducer(
            result.producerMetadata,
            "memory_formation_text",
          ),
          basis: [
            {
              basis: {
                case: "evidence",
                value: {
                  occurrenceId: r.occurrenceId,
                  basisRole: "interpretation",
                  locator: snapshot.representationId
                    ? {
                        case: "derivedRepresentationId",
                        value: snapshot.representationId,
                      }
                    : { case: "wholeOccurrence", value: true },
                },
              },
            },
          ],
        },
      });
      proposed = {
        request: toJson(FormMemoryRequestSchema, request),
      };
      await kernel.modelWorkflow.saveWorkflow(
        { ...lease, proposalJson: JSON.stringify(proposed) },
        options,
      );
    }
    const request = fromJson(
      FormMemoryRequestSchema,
      proposed.request as JsonValue,
    );
    const memory = await kernel.memory.formMemory(request, options);
    await kernel.modelWorkflow.saveWorkflow(
      {
        ...lease,
        outcomeJson: JSON.stringify({
          revisionId: memory.revisionId,
        }),
      },
      options,
    );
    return {
      memory,
      degradation: [],
    };
  } catch (error) {
    if (failedExecutionTelemetry(error))
      await kernel.modelWorkflow.saveWorkflow(
        {
          ...lease,
          executionTelemetryJson: JSON.stringify(
            failedExecutionTelemetry(error),
          ),
        },
        opportunity.cleanup(),
      );
    throw error;
  } finally {
    await kernel.modelWorkflow
      .releaseWorkflow(lease, opportunity.cleanup())
      .catch(() => {});
  }
}
