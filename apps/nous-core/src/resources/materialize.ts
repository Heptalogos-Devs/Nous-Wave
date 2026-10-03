import { create, fromJson, toJson, type JsonValue } from "@bufbuild/protobuf";
import { Code, ConnectError, type CallOptions } from "@connectrpc/connect";
import { z } from "zod";
import {
  type MaterializeResourceRequest,
  ResourceMaterializationSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/management_pb.js";
import {
  ObservationInputSchema,
  StableExternalRefSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import type { KernelClient } from "../kernel-client.js";
import { canonicalDigest } from "../digest.js";
import type { ResourceRegistry } from "./registry.js";
import { ResourceProviderError } from "./adapter.js";

export async function materializeResource(
  kernel: KernelClient,
  registry: ResourceRegistry,
  request: MaterializeResourceRequest,
  options: CallOptions,
) {
  if (
    !request.reference ||
    !request.observedAt ||
    !z.string().uuid().safeParse(request.operationId).success
  )
    throw new ConnectError(
      "Selected resource materialization requires reference, stable operation ID and observed time",
      Code.InvalidArgument,
    );
  const reference = {
    ...request.reference,
    entryVersion: request.reference.entryVersion ?? null,
  };
  const identity = {
    subjectId: request.subjectId,
    owner: "material",
    operationKey: `resource:${request.operationId}`,
    semanticDigest: canonicalDigest({
      subject: request.subjectId,
      reference: toJson(StableExternalRefSchema, request.reference),
      observedAt: {
        seconds: request.observedAt.seconds.toString(),
        nanos: request.observedAt.nanos,
      },
    }),
  };
  const reservation = await kernel.modelMaterial.reserveWorkflow(
    { ...identity, snapshotJson: JSON.stringify({ reference }) },
    options,
  );
  if (reservation.outcomeJson)
    return fromJson(
      ResourceMaterializationSchema,
      JSON.parse(reservation.outcomeJson) as JsonValue,
    );
  if (reservation.busy || !reservation.leaseToken)
    throw new ConnectError(
      "Resource materialization operation is busy",
      Code.ResourceExhausted,
    );
  const lease = {
    subjectId: request.subjectId,
    owner: "material",
    operationKey: identity.operationKey,
    leaseToken: reservation.leaseToken,
  };
  try {
    const descriptor = await kernel.authority.getResource(
      { subjectId: request.subjectId, id: reference.resourceRef },
      options,
    );
    if (
      descriptor.adapterKind !== reference.providerKind ||
      descriptor.providerProfile !== reference.providerProfile ||
      descriptor.providerLocator !== reference.accessScope
    )
      throw new ConnectError(
        "Resource binding has changed",
        Code.FailedPrecondition,
      );
    const adapter = registry.resolve(descriptor);
    if (!adapter)
      throw new ConnectError(
        "Resource adapter is not configured",
        Code.FailedPrecondition,
      );
    let proposal = reservation.proposalJson
      ? z
          .strictObject({
            content: z.string().max(1048576),
            mediaType: z.string().max(128),
          })
          .parse(JSON.parse(reservation.proposalJson))
      : undefined;
    if (!proposal) {
      const material = await adapter.materialize(
        reference,
        options.signal ?? undefined,
      );
      proposal = { content: material.content, mediaType: material.mediaType };
      await kernel.modelMaterial.saveWorkflow(
        { ...lease, proposalJson: JSON.stringify(proposal) },
        options,
      );
    }
    const observation = await kernel.authority.recordObservation(
      create(ObservationInputSchema, {
        subjectId: request.subjectId,
        requestId: request.operationId,
        sourceClass: "resource",
        observedAt: request.observedAt,
        externalObjectRef: `object:resource-entry:${canonicalDigest(toJson(StableExternalRefSchema, request.reference))}`,
        context: {
          external_resource: toJson(StableExternalRefSchema, request.reference),
        },
        material: {
          case: "inlineText",
          value: { text: proposal.content, mediaType: proposal.mediaType },
        },
      }),
      options,
    );
    const result = create(ResourceMaterializationSchema, {
      reference: request.reference,
      observation,
      content: proposal.content,
      mediaType: proposal.mediaType,
    });
    await kernel.modelMaterial.saveWorkflow(
      {
        ...lease,
        outcomeJson: JSON.stringify(
          toJson(ResourceMaterializationSchema, result),
        ),
      },
      options,
    );
    return result;
  } catch (error) {
    if (options.signal?.aborted) throw error;
    if (error instanceof ResourceProviderError)
      throw new ConnectError(
        `Resource material unavailable (${error.status})`,
        Code.FailedPrecondition,
      );
    throw error;
  } finally {
    await kernel.modelMaterial
      .releaseWorkflow(lease, {
        timeoutMs: kernel.execution.workflow_ack_timeout_ms,
      })
      .catch(() => {});
  }
}
