// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { createHash, randomUUID } from "node:crypto";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { resolve, dirname } from "node:path";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import type { NousClient } from "@nous-wave/client";
import { connectNousInstance } from "@nous-wave/client/node";

interface Scenario {
  key: string;
  entities: { key: string; display_name: string; aliases: string[] }[];
  events: {
    key: string;
    text: string;
    occurred_at: string;
    observed_at: string;
    session_key: string;
    entity_keys: string[];
  }[];
  work_context: { key: string; purpose: string; open_questions: string[] };
  expected_maintenance: {
    concepts: {
      key: string;
      meaning: string;
      names: string[];
      reuse_events?: string[];
      distinct_from?: string;
    }[];
    no_center_for: string[];
    supported_relations?: [string, string, string][];
    ambiguity?: { label: string; candidates: string[] };
  };
}
interface FunctionalQuery {
  key: string;
  scenario_key: string;
  category: string;
  prepared_query: {
    canonical_text: string;
    entity_keys?: string[];
    concept_keys?: string[];
    exact_event_keys?: string[];
    work_context_key?: string;
    exploration?: string;
    time_axis?: string;
    time_start?: string;
    time_end?: string;
    minimum_hops?: number;
  };
  oracle: {
    required_event_keys: string[];
    forbidden_event_keys: string[];
    requires_supported_path: boolean;
  };
}
const at = (value: string) => timestampFromDate(new Date(value));
export async function runCognitiveFunctional(
  client: NousClient,
  profiles: string[],
  output: string,
  limits = { maxModelCalls: 128, maxElapsedMs: 60000 },
) {
  if (
    !Number.isSafeInteger(limits.maxModelCalls) ||
    limits.maxModelCalls < 1 ||
    !Number.isSafeInteger(limits.maxElapsedMs) ||
    limits.maxElapsedMs < 1
  )
    throw new Error("Functional limits require positive integers");
  const operationOptions = { signal: AbortSignal.timeout(limits.maxElapsedMs) };
  let modelCalls = 0;
  const remainingCalls = () => limits.maxModelCalls - modelCalls;
  const reserveFormation = () => {
    operationOptions.signal.throwIfAborted();
    if (!remainingCalls())
      throw new Error("Functional model call limit reached");
    modelCalls++;
  };
  const { scenarios } = JSON.parse(
    await readFile(
      resolve("docs/research/corpus/functional/scenarios.json"),
      "utf8",
    ),
  ) as { scenarios: Scenario[] };
  const { queries } = JSON.parse(
    await readFile(
      resolve("docs/research/corpus/functional/queries.json"),
      "utf8",
    ),
  ) as { queries: FunctionalQuery[] };
  const results: Record<string, unknown>[] = [];
  for (const scenario of scenarios) {
    const subjectId = randomUUID();
    await client.subjects.create(
      {
        subjectId,
        operationId: randomUUID(),
        cognitiveSeed: {
          text: "schema_version = 1",
          format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
          provenance: {},
        },
      },
      operationOptions,
    );
    const entities = new Map<string, string>();
    for (const entity of scenario.entities) {
      const value = `entity:functional:${subjectId}:${entity.key}`;
      await client.identity.bind(
        {
          subjectId,
          canonical: { kind: "entity", value },
          displayName: entity.display_name,
          aliases: entity.aliases,
        },
        operationOptions,
      );
      entities.set(entity.key, value);
    }
    const work = await client.cognition.createWorkContext(
      {
        subjectId,
        operationId: randomUUID(),
        purpose: scenario.work_context.purpose,
        unresolvedQuestions: scenario.work_context.open_questions,
      },
      operationOptions,
    );
    const sessions = new Map<string, string>();
    const memories = new Map<
      string,
      {
        revisionId: string;
        occurrenceId: string;
        sourceRegionId?: string;
        artifactId?: string;
      }
    >();
    for (const event of scenario.events) {
      let sessionId = sessions.get(event.session_key);
      if (!sessionId) {
        sessionId = (
          await client.cognition.openSession({ subjectId }, operationOptions)
        ).sessionId;
        sessions.set(event.session_key, sessionId);
      }
      const observation = await client.cognition.observe(
        {
          subjectId,
          sessionId,
          requestId: randomUUID(),
          sourceClass: "host_event",
          admit: true,
          occurredTime: {
            value: { case: "instant", value: at(event.occurred_at) },
          },
          observedAt: at(event.observed_at),
          material: {
            case: "inlineText",
            value: { text: event.text, mediaType: "text/plain" },
          },
          entities: event.entity_keys.map((key) => ({
            surface: scenario.entities.find((e) => e.key === key)!.display_name,
            entityRef: entities.get(key),
          })),
        },
        operationOptions,
      );
      reserveFormation();
      const formation = await client.model.formFromObservation(
        {
          subjectId,
          occurrenceId: observation.occurrenceId,
          operationId: randomUUID(),
          aboutnessMode: "explicit",
          explicitAboutness: event.entity_keys.map((key) => entities.get(key)!),
        },
        operationOptions,
      );
      if (!formation.memory)
        throw new Error(`Formation unavailable for ${event.key}`);
      memories.set(event.key, {
        revisionId: formation.memory.revisionId,
        occurrenceId: observation.occurrenceId,
        sourceRegionId: observation.sourceRegionId,
        artifactId: observation.artifactId,
      });
    }
    for (const sessionId of sessions.values())
      await client.cognition.closeSession(
        { subjectId, id: sessionId },
        operationOptions,
      );
    const maintenance = [];
    for (let grant = 0; grant < 8; grant++) {
      operationOptions.signal.throwIfAborted();
      if (!remainingCalls())
        throw new Error("Functional model call limit reached");
      const result = await client.cognition.grantMaintenance(
        {
          subjectId,
          maxOperations: 16,
          maxModelCalls: Math.min(16, remainingCalls()),
          maxElapsedMs: Math.min(15000, limits.maxElapsedMs),
        },
        operationOptions,
      );
      modelCalls += result.modelCalls;
      maintenance.push(result);
      if (!result.results.length) break;
    }
    const tags = (
      await client.topology.listTags(
        {
          subjectId,
          status: "active",
          page: { pageSize: 64 },
        },
        operationOptions,
      )
    ).items;
    const concepts = new Map<string, string>();
    const failures: string[] = [];
    for (const concept of scenario.expected_maintenance.concepts) {
      let id: string | undefined;
      for (const name of concept.names) {
        const resolved = await client.identity.resolve(
          {
            subjectId,
            kind: "tag",
            locator: { case: "name", value: name },
          },
          operationOptions,
        );
        if (resolved.status === "BOUND") {
          const value = resolved.candidates[0]?.canonical?.value;
          if (id && value !== id)
            failures.push(`unmerged_alias:${concept.key}:${name}`);
          id = value;
        }
      }
      if (!id) {
        failures.push(`concept_missing:${concept.key}`);
        continue;
      }
      concepts.set(concept.key, id);
      for (const key of concept.reuse_events ?? []) {
        const memory = memories.get(key)!;
        const neighborhood = await client.topology.neighborhood(
          {
            subjectId,
            root: { kind: "memory_revision", value: memory.revisionId },
            maxNodes: 64,
            maxDepth: 1,
          },
          operationOptions,
        );
        if (
          !neighborhood.associations.some(
            (a) =>
              a.relationKind === "tag_attachment" &&
              a.to?.value === id &&
              a.supports.length > 0,
          )
        )
          failures.push(`attachment_missing:${key}:${concept.key}`);
      }
    }
    for (const concept of scenario.expected_maintenance.concepts) {
      if (
        concept.distinct_from &&
        concepts.get(concept.key) === concepts.get(concept.distinct_from)
      )
        failures.push(`distinct_concepts_merged:${concept.key}`);
    }
    for (const [from, to, relation] of scenario.expected_maintenance
      .supported_relations ?? []) {
      const neighborhood = await client.topology.neighborhood(
        {
          subjectId,
          root: {
            kind: "memory_revision",
            value: memories.get(from)!.revisionId,
          },
          maxNodes: 64,
          maxDepth: 1,
        },
        operationOptions,
      );
      if (
        !neighborhood.associations.some(
          (a) =>
            a.to?.value === memories.get(to)!.revisionId &&
            a.relationKind === `assoc.${relation}` &&
            a.supports.length > 0,
        )
      )
        failures.push(`relation_missing:${from}:${to}`);
    }
    for (const name of scenario.expected_maintenance.no_center_for)
      if (tags.some((tag) => tag.label.toLowerCase() === name.toLowerCase()))
        failures.push(`noise_concept:${name}`);
    if (scenario.expected_maintenance.ambiguity) {
      const ambiguity = await client.identity.resolve(
        {
          subjectId,
          kind: "entity",
          locator: {
            case: "name",
            value: scenario.expected_maintenance.ambiguity.label,
          },
        },
        operationOptions,
      );
      if (ambiguity.status !== "AMBIGUOUS_REFERENCE")
        failures.push("identity_ambiguity_missing");
    }
    results.push({
      scenario: scenario.key,
      stage: "formation",
      subjectId,
      tags,
      maintenance,
      failures,
    });
    if (failures.length) continue;
    for (const item of queries.filter((q) => q.scenario_key === scenario.key)) {
      let semanticInput: string | undefined;
      for (const profile of profiles) {
        await client.configuration.setSubject(
          {
            subjectId,
            operationId: randomUUID(),
            path: "retrieval.cognitive.profile",
            value: profile,
          },
          operationOptions,
        );
        const p = item.prepared_query;
        const request: Parameters<NousClient["cognition"]["query"]>[0] = {
          subjectId,
          workContextId: p.work_context_key
            ? work.workContext?.workContextId
            : undefined,
          expression: {
            operation: "atom",
            cues: [
              { cue: { case: "text", value: p.canonical_text } },
              ...(p.entity_keys ?? []).map((key) => ({
                cue: { case: "entityRef" as const, value: entities.get(key)! },
              })),
              ...(p.concept_keys ?? []).map((key) => ({
                cue: { case: "tagId" as const, value: concepts.get(key)! },
              })),
              ...(p.exact_event_keys ?? []).map((key) => ({
                cue: {
                  case: "reference" as const,
                  value: {
                    kind: "memory_revision",
                    value: memories.get(key)!.revisionId,
                  },
                },
              })),
            ],
            modifiers: {
              limit: 16,
              effort: p.minimum_hops && p.minimum_hops > 2 ? "deep" : "normal",
              materialize: true,
              diagnostics: "full",
              exploration:
                p.exploration === "associative"
                  ? "bounded_associative"
                  : p.exploration,
              constraints: p.time_axis
                ? {
                    [p.time_axis.replace("_time", "")]: {
                      start: p.time_start ? at(p.time_start) : undefined,
                      end: p.time_end ? at(p.time_end) : undefined,
                    },
                  }
                : undefined,
            },
          },
          capabilities: { rerank: "forbidden", textEmbedding: "optional" },
        };
        const prepared = await client.cognition.prepareQuery(
          request,
          operationOptions,
        );
        if (
          semanticInput !== undefined &&
          semanticInput !== prepared.embeddingText
        )
          throw new Error(
            `Prepared representation changed across profiles: ${item.key}`,
          );
        semanticInput = prepared.embeddingText;
        const response = await client.cognition.query(
          request,
          operationOptions,
        );
        const foundEvents = new Set<string>();
        for (const hit of response.hits) {
          const ids = [
            hit.reference?.value,
            hit.revision?.value,
            ...hit.evidence.map((e) => e.reference?.value),
          ].filter((id) => id !== undefined);
          for (const [key, memory] of memories)
            if (
              ids.includes(memory.revisionId) ||
              ids.includes(memory.occurrenceId) ||
              (memory.sourceRegionId !== undefined &&
                ids.includes(memory.sourceRegionId)) ||
              (memory.artifactId !== undefined &&
                ids.includes(memory.artifactId))
            )
              foundEvents.add(key);
        }
        const requiredFound = item.oracle.required_event_keys.filter((key) =>
          foundEvents.has(key),
        );
        const forbiddenFound = item.oracle.forbidden_event_keys.filter((key) =>
          foundEvents.has(key),
        );
        const supportedTopologyHits = response.hits.filter(
          (hit) =>
            hit.evidenceFamilies.some((family) =>
              family.includes("topology"),
            ) && hit.evidence.length > 0,
        ).length;

        results.push({
          scenario: scenario.key,
          stage: "query",
          key: item.key,
          category: item.category,
          profile,
          preparedText: prepared.embeddingText,
          response,
          oracle: item.oracle,
          requiredFound,
          forbiddenFound,
          supportedTopologyHits,
        });
      }
    }
  }
  await mkdir(dirname(output), { recursive: true });
  await writeFile(
    output,
    JSON.stringify(
      {
        scope: "hand-authored functional corpus; no benchmark accuracy claim",
        profiles,
        modelCalls,
        limits,
        results,
      },
      (_key, value: unknown) =>
        typeof value === "bigint" ? value.toString() : value,
      2,
    ) + "\n",
  );
  return results;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  const { values } = parseArgs({
    options: {
      "run-root": { type: "string" },
      profiles: { type: "string", default: "baseline-rrf" },
      "compat-input": { type: "string" },
      "max-model-calls": { type: "string", default: "128" },
      "max-elapsed-ms": { type: "string", default: "60000" },
      output: {
        type: "string",
        default: "data/research/cognitive-functional/results.json",
      },
    },
  });
  if (!values["run-root"])
    throw new Error(
      "Use --run-root of an already running official Core instance",
    );
  const client = await connectNousInstance({ runRoot: values["run-root"] });
  const results = await runCognitiveFunctional(
    client,
    values.profiles.split(","),
    resolve(values.output),
    {
      maxModelCalls: Number(values["max-model-calls"]),
      maxElapsedMs: Number(values["max-elapsed-ms"]),
    },
  );
  if (values["compat-input"])
    await runSelectedTextCompatibility(
      client,
      resolve(values["compat-input"]),
      resolve(values.output) + ".selected.json",
    );
  const failed = results.filter(
    (r) =>
      r.stage === "formation" &&
      Array.isArray(r.failures) &&
      r.failures.length > 0,
  );
  process.stdout.write(
    JSON.stringify({
      output: resolve(values.output),
      formationFailures: failed.length,
    }) + "\n",
  );
  if (failed.length) process.exitCode = 1;
}

export async function runSelectedTextCompatibility(
  client: NousClient,
  inputPath: string,
  output: string,
  profiles = [
    "baseline-rrf",
    "nous-node-potential-v1",
    "vcp-dtsc-v9.2.1-adapter-v1",
    "vcp-rivermemo-v3.1-adapter-v1",
  ],
) {
  interface Selection {
    key: string;
    text: string;
    source_pool: { key: string; sha256: string }[];
    required_sources: string[];
    sources: { key: string; text: string; occurred_at: string }[];
  }
  const selection = JSON.parse(
    await readFile(
      resolve("docs/research/corpus/text-compatibility-selection.json"),
      "utf8",
    ),
  ) as { cases: Selection[] };
  const inputs = JSON.parse(await readFile(inputPath, "utf8")) as {
    cases: Selection[];
  };
  const results = [];
  for (const selected of selection.cases) {
    const sourceCase = inputs.cases.find((c) => c.key === selected.key);
    if (!sourceCase)
      throw new Error(`Missing selected raw source pool: ${selected.key}`);
    const subjectId = randomUUID();
    await client.subjects.create({
      subjectId,
      operationId: randomUUID(),
      cognitiveSeed: {
        text: "schema_version = 1",
        format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
        provenance: {},
      },
    });
    const refs = new Map<string, string>();
    for (const source of sourceCase.sources) {
      const metadata = selected.source_pool.find((s) => s.key === source.key);
      if (
        !metadata ||
        createHash("sha256").update(source.text).digest("hex") !==
          metadata.sha256
      )
        throw new Error(`Selected source checksum mismatch: ${source.key}`);
      const observation = await client.cognition.observe({
        subjectId,
        requestId: randomUUID(),
        sourceClass: "import",
        occurredTime: {
          value: { case: "instant", value: at(source.occurred_at) },
        },
        observedAt: at(source.occurred_at),
        material: {
          case: "inlineText",
          value: { text: source.text, mediaType: "text/plain" },
        },
      });
      const memory = await client.memory.form({
        subjectId,
        operationId: randomUUID(),
        input: {
          cognitiveRole: "declarative",
          formationMode: "grounded",
          groundingOccurrenceId: observation.occurrenceId,
          semanticRole: "raw_text_compatibility",
          text: source.text,
          supports: [
            {
              support: {
                case: "evidence",
                value: {
                  occurrenceId: observation.occurrenceId,
                  locator: { case: "wholeOccurrence", value: true },
                  supportRole: "direct",
                },
              },
            },
          ],
          epistemicClass: "observed",
        },
      });
      refs.set(source.key, memory.revisionId);
    }
    const request: Parameters<NousClient["cognition"]["query"]>[0] = {
      subjectId,
      textOnlyCompatibility: true,
      capabilities: { textEmbedding: "forbidden", rerank: "forbidden" },
      expression: {
        operation: "atom",
        cues: [{ cue: { case: "text", value: selected.text } }],
        modifiers: {
          projection: { domains: ["memory"] },
          limit: 8,
          materialize: true,
          diagnostics: "full",
        },
      },
    };
    for (const profile of profiles) {
      await client.configuration.setSubject({
        subjectId,
        operationId: randomUUID(),
        path: "retrieval.cognitive.profile",
        value: profile,
      });
      const prepared = await client.cognition.prepareQuery(request);
      const response = await client.cognition.query(request);
      const ranks = selected.required_sources.map((key) => ({
        key,
        rank:
          response.hits.findIndex(
            (hit) =>
              hit.revision?.value === refs.get(key) ||
              hit.reference?.value === refs.get(key),
          ) + 1,
      }));
      results.push({
        key: selected.key,
        profile,
        preparedText: prepared.embeddingText,
        ranks,
        response,
      });
    }
  }
  await mkdir(dirname(output), { recursive: true });
  await writeFile(
    output,
    JSON.stringify(
      {
        scope:
          "six selected raw-text compatibility cases; no benchmark accuracy claim; no contextual oracle input",
        externalProviderCalls: 0,
        rerankCalls: 0,
        results,
      },
      (_key, value: unknown) =>
        typeof value === "bigint" ? value.toString() : value,
      2,
    ) + "\n",
  );
  return results;
}
