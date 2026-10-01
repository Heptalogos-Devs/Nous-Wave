import { create } from "@bufbuild/protobuf";
import { ExternalResourceResultSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/model_pb.js";
import {
  ExternalResourceRecordSchema,
  ResourceProviderEvidenceSchema,
  StableExternalRefSchema,
  type ResourceAction,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { ResourceProviderError } from "./adapter.js";
import type { ResourceRegistry } from "./registry.js";

export async function executeResourceActions(
  registry: ResourceRegistry,
  actions: ResourceAction[],
  signal?: AbortSignal,
) {
  const results = [];
  for (const action of actions) {
    if (signal?.aborted) throw signal.reason;
    const binding = {
      resourceRef: action.resourceRef,
      adapterKind: action.adapterKind,
      providerProfile: action.providerProfile,
      providerLocator: action.providerLocator,
    };
    const adapter = registry.resolve(binding);
    const started = performance.now();
    let requestCount = 0;
    const context = {
      requestStarted: () => {
        requestCount++;
      },
    };
    const evidence = () =>
      create(ResourceProviderEvidenceSchema, {
        profileDigest: adapter?.profileDigest ?? "",
        requestCount,
        latencyMs: performance.now() - started,
      });
    if (!adapter) {
      results.push(
        create(ExternalResourceResultSchema, {
          actionId: action.actionId,
          resourceRef: action.resourceRef,
          status: "unavailable",
          providerEvidence: evidence(),
          diagnostics: ["adapter_unavailable"],
        }),
      );
      continue;
    }
    try {
      if (action.action === "inspect_synopsis") {
        const descriptor = await adapter.describe(binding, signal, context);
        results.push(
          create(ExternalResourceResultSchema, {
            actionId: action.actionId,
            resourceRef: action.resourceRef,
            status: descriptor.accessible ? "success" : "denied",
            providerEvidence: evidence(),
            diagnostics: descriptor.accessible ? [] : ["provider_denied"],
          }),
        );
        continue;
      }
      const records = await adapter.search(
        binding,
        action.queryText,
        action.limit,
        signal,
        context,
      );
      if (action.currentAuthority || action.materialize) {
        for (const record of records) {
          const material = await adapter.materialize(
            record.reference,
            signal,
            context,
          );
          record.versionStatus = material.version.status;
          record.accessStatus = material.access.status;
        }
      }
      results.push(
        create(ExternalResourceResultSchema, {
          actionId: action.actionId,
          resourceRef: action.resourceRef,
          status: "success",
          providerEvidence: evidence(),
          records: records.map((record) =>
            create(ExternalResourceRecordSchema, {
              ...record,
              reference: create(StableExternalRefSchema, {
                ...record.reference,
                entryVersion: record.reference.entryVersion ?? undefined,
              }),
            }),
          ),
        }),
      );
    } catch (error) {
      if (signal?.aborted) throw error;
      results.push(
        create(ExternalResourceResultSchema, {
          actionId: action.actionId,
          resourceRef: action.resourceRef,
          status:
            error instanceof ResourceProviderError ? error.status : "failed",
          providerEvidence: evidence(),
          diagnostics: [
            error instanceof ResourceProviderError
              ? `provider_${error.status}`
              : "provider_failed",
          ],
        }),
      );
    }
  }
  return results;
}
