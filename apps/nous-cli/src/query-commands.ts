// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { CliError, boundedInteger } from "./agent.js";
import type { CliEnvironment } from "./runtime.js";
const cognitionKinds = new Set([
  "memory_revision",
  "cognitive_schema_revision",
  "episode_revision",
  "journal_revision",
]);
export async function queryCommands(
  env: CliEnvironment,
  action?: string,
  argument?: string,
  positionals: string[] = [],
) {
  const { client, state, subjectId, values, required, save } = env;
  const preparing = action === "prepare" || action === "inspect";
  const nousql = values["query-file"]
    ? await env.readText(values["query-file"], 32768)
    : required(
        positionals.length
          ? positionals.slice(preparing ? 2 : 1).join(" ")
          : preparing
            ? argument
            : action,
        "NousQL",
      );
  if (Buffer.byteLength(nousql) > 32768)
    throw new CliError("INVALID_ARGUMENT", "NousQL exceeds 32 KiB");
  const request = {
    subjectId,
    sessionId: state.sessionId,
    workContextId: state.workContextId,
    nousql,
  };
  if (preparing) {
    const response = await client.cognition.prepareQuery(request);
    if (values.raw)
      return {
        ...response,
        boundQuery: JSON.parse(response.boundQuery) as unknown,
      };
    const bound = JSON.parse(response.boundQuery) as {
      representation?: { sha256?: string; truncation_flags?: string[] };
      context_snapshot?: {
        digest?: string;
        work_context?: {
          work_context_id: string;
          revision: number;
          purpose: string;
          cognition_anchors: unknown[];
          entity_anchors: string[];
          tag_anchors: string[];
        };
        session?: {
          session_id: string;
          runtime_revision: number;
          resident: unknown[];
        };
        history_exclusions?: unknown[];
      };
      result_projection?: unknown;
      temporal_frame?: unknown;
      query_activation?: {
        degradation?: unknown;
        explicit_tags?: unknown[];
        inferred_tags?: unknown[];
        model_calls?: number;
      };
    };
    return {
      intent: nousql,
      representation: {
        sha256: bound.representation?.sha256,
        chars: Array.from(response.embeddingText).length,
        truncationFlags: bound.representation?.truncation_flags ?? [],
      },
      context: {
        digest: bound.context_snapshot?.digest,
        purpose: bound.context_snapshot?.work_context?.purpose,
        workContextId: bound.context_snapshot?.work_context?.work_context_id,
        workContextRevision: bound.context_snapshot?.work_context?.revision,
        cognitionAnchors:
          bound.context_snapshot?.work_context?.cognition_anchors,
        entities: bound.context_snapshot?.work_context?.entity_anchors,
        tags: bound.context_snapshot?.work_context?.tag_anchors,
        sessionId: bound.context_snapshot?.session?.session_id,
        residentCount: bound.context_snapshot?.session?.resident.length,
        historyExclusions: bound.context_snapshot?.history_exclusions,
      },
      activation: {
        queryAndContextTags: bound.query_activation?.explicit_tags,
        inferredTags: bound.query_activation?.inferred_tags,
        modelCalls: bound.query_activation?.model_calls,
      },
      projection: bound.result_projection,
      temporalFrame: bound.temporal_frame,
      capabilities: {
        textEmbedding: response.textEmbeddingRequirement,
        rerank: response.rerankRequirement,
        conceptEnrichment: response.conceptEnrichmentRequirement,
        conceptEnrichmentMode: response.conceptEnrichmentMode,
      },
      embeddingRequired: response.embeddingRequired,
      degradation: bound.query_activation?.degradation,
    };
  }
  // A failed new query must not leave result:N pointing at an unrelated old answer.
  await save({ ...state, lastQuery: undefined });
  const response = await client.cognition.query(request);
  const refs = response.hits.map((hit) => hit.revision ?? hit.reference);
  if (refs.some((ref) => !ref))
    throw new CliError(
      "REFERENCE_TYPE_MISMATCH",
      "Query result has no reference",
    );
  await save({
    ...state,
    lastQuery: {
      subjectId,
      queryId: response.queryId,
      results: refs.map((ref) => ({ kind: ref!.kind, value: ref!.value })),
    },
  });
  if (values.raw) return response;
  return {
    queryId: response.queryId,
    queryRef: "query:last",
    status: response.status,
    results: response.hits.map((hit, index) => ({
      lexicalRef: hit.lexicalRef,
      result: `result:${index + 1}`,
      ref: `${refs[index]!.kind}:${refs[index]!.value}`,
      text: hit.text,
      authority: hit.authority,
      cognitiveRole: hit.cognitiveRole,
      formationMode: hit.formationMode,
      next: cognitionKinds.has(refs[index]!.kind)
        ? `show result:${index + 1} | trace result:${index + 1} | use result:${index + 1} | context pin --cognition result:${index + 1}`
        : refs[index]!.kind === "occurrence"
          ? `show result:${index + 1} | read result:${index + 1} | context pin --cognition result:${index + 1}`
          : refs[index]!.kind === "resource"
            ? `show result:${index + 1}`
            : [
                  "artifact",
                  "source_region",
                  "derived_representation",
                  "derived_region",
                ].includes(refs[index]!.kind)
              ? `show result:${index + 1} | read result:${index + 1}`
              : "Returned source reference; cognition use and pin are unavailable",
    })),
    resourceRecords: response.resourceRecords,
    resourceActions: response.resourceActions,
    diagnostics: response.diagnostics && {
      ...response.diagnostics,
      ...(response.diagnostics.trace
        ? { trace: JSON.parse(response.diagnostics.trace) as unknown }
        : {}),
    },
    degradation: response.degradation,
  };
}
export async function showCommands(env: CliEnvironment, reference?: string) {
  const { client, subjectId, required, resolveReference } = env;
  const ref = await resolveReference(
    required(reference, "Exact reference or result:N"),
  );
  const input = { subjectId, id: ref.value };
  switch (ref.kind) {
    case "memory_revision":
      return client.memory.revision(input);
    case "cognitive_schema_revision":
      return client.concepts.getSchemaRevision(input);
    case "episode_revision":
      return client.memory.getEpisodeRevision(input);
    case "journal_revision":
      return client.memory.getJournalRevision(input);
    case "occurrence":
      return client.material.occurrence(input);
    case "artifact":
      return client.material.getArtifact(input);
    case "source_region":
      return client.material.sourceRegion(input);
    case "derived_representation":
      return client.material.representation(input);
    case "derived_region":
      return client.material.derivedRegion(input);
    case "resource":
      return client.resources.get(input);
    default:
      throw new CliError(
        "REFERENCE_TYPE_MISMATCH",
        `show is unavailable for ${ref.kind}; use the returned source details`,
      );
  }
}

export async function readCommands(env: CliEnvironment, reference?: string) {
  const ref = await env.resolveReference(
    env.required(reference, "Material reference or result:N"),
  );
  if (
    ![
      "occurrence",
      "artifact",
      "source_region",
      "derived_representation",
      "derived_region",
    ].includes(ref.kind)
  )
    throw new CliError(
      "REFERENCE_TYPE_MISMATCH",
      "read requires a Material reference; show reads cognition revisions",
    );
  const material = await env.client.material.materialize({
    subjectId: env.subjectId,
    reference: ref,
    maxBytes: BigInt(
      boundedInteger(
        env.values["max-bytes"] ?? "65536",
        1,
        1048576,
        "--max-bytes",
      ),
    ),
  });
  const textLike = /^text\/|^application\/(?:json|xml|.*\+json)(?:;|$)/.test(
    material.mediaType,
  );
  return {
    reference: material.reference,
    mediaType: material.mediaType,
    totalBytes: material.totalBytes,
    rangeStart: material.rangeStart,
    rangeEnd: material.rangeEnd,
    partial: material.partial,
    ...(textLike
      ? {
          text: new TextDecoder("utf-8", { fatal: true }).decode(
            material.content,
            { stream: material.partial },
          ),
        }
      : {
          next: "Binary source; derive <SourceRegion reference> for a textual interpretation. Use the src: reference returned by observe or trace.",
        }),
    evidence: material.evidence,
    degradation: material.degradation,
  };
}
