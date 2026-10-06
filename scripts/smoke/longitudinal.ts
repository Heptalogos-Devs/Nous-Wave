// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { workspaceTemp } from "../workspace.js";
import { randomUUID } from "node:crypto";
import { createInterface } from "node:readline";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { fromJson, type JsonValue } from "@bufbuild/protobuf";
import { connectNous } from "@nous-wave/client/node";
import { MaintenancePlanSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import { KernelClient } from "../../apps/nous-core/src/kernel-client.js";
import { createCore } from "../../apps/nous-core/src/server.js";
import { ModelRuntime } from "../../apps/nous-core/src/model/runtime.js";
import { modelConfigurationSchema } from "../../apps/nous-core/src/model/configuration.js";
import {
  episodePartitionSchema,
  journalSynthesisSchema,
} from "../../apps/nous-core/src/model/schemas/longitudinal.js";
import { consolidationSchema } from "../../apps/nous-core/src/model/schemas/consolidation.js";

import {
  runCognitiveFunctional,
  runSelectedTextCompatibility,
} from "../research/cognitive-functional.js";
import { conceptMaintenanceSchema } from "../../apps/nous-core/src/model/schemas/concept-maintenance.js";

function proposalModels() {
  const models = new ModelRuntime();
  let functional = false;
  Object.defineProperty(models.invocations, "capabilities", {
    get: () =>
      [
        "episode_segmentation",
        "journal_synthesis",
        "memory_consolidation",
        ...(functional ? ["memory_formation", "concept_maintenance"] : []),
      ].map((role) => ({
        name: `model.${role}`,
        state: "READY",
        detail: "Executable smoke proposal adapter",
      })),
  });
  const calls = { episode: 0, journal: 0, consolidation: 0 };
  models.invocations.snapshot = (role) => ({
    role,
    configuration: modelConfigurationSchema.parse({}),
    profileDigest: "a".repeat(64),
    configDigest: "b".repeat(64),
  });
  const producerMetadata = {
    implementation: "longitudinal-proposal-stub",
    protocol: "openai-chat",
    model: "deterministic",
    profileDigest: "a".repeat(64),
    configDigest: "b".repeat(64),
    outputSchemaDigest: "c".repeat(64),
    promptId: "smoke/longitudinal",
    promptDigest: "d".repeat(64),
  };
  const plan = (input: string) =>
    fromJson(MaintenancePlanSchema, JSON.parse(input) as JsonValue);
  models.segmentEpisode = async (input) => {
    calls.episode++;
    const source = plan(input);
    return {
      value: episodePartitionSchema.parse({
        action: "partition",
        segments: [
          {
            memberKeys: source.members.map((member) => member.key),
            title: "Calibration sequence",
            boundaryExplanation: "Sequential calibration observations.",
          },
        ],
      }),
      producerMetadata,
    };
  };
  models.synthesizeJournal = async (input) => {
    calls.journal++;
    const source = plan(input);
    const supportKeys = source.supports
      .filter(
        (entry) =>
          entry.support?.support.case === "cognitionDependency" &&
          entry.support.support.value.targetRevision?.kind ===
            "episode_revision",
      )
      .map((entry) => entry.key);
    return {
      value: journalSynthesisSchema.parse({
        action: "commit",
        title: "Calibration journal",
        narrative: "Calibration plan confirmed.",
        points: [
          {
            role: "decision",
            text: "Calibration plan confirmed.",
            supportKeys,
          },
        ],
      }),
      producerMetadata,
    };
  };
  models.consolidate = async (input) => {
    calls.consolidation++;
    const source = plan(input);
    const member = source.members[0];
    assert(member);
    const evidence = source.supports.find(
      (entry) =>
        entry.support?.support.case === "evidence" &&
        entry.support.support.value.occurrenceId === member.occurrenceId,
    );
    assert(evidence);
    const actions = source.candidates.some(
      (candidate) => candidate.target?.reference?.kind === "memory_revision",
    )
      ? [
          {
            action: "skip",
            reason: "The supported calibration Memory is available.",
          },
        ]
      : [
          {
            action: "create_memory",
            content: {
              cognitiveRole: "experiential",
              formationMode: "grounded",
              groundingMemberKey: member.key,
              semanticRole: "calibration",
              text: "Calibration plan confirmed.",
              title: "Calibration",
              supportKeys: [evidence.key],
              entityKeys: [],
              validTime: { kind: "unknown" },
              epistemicClass: "observed",
            },
          },
        ];
    return { value: consolidationSchema.parse({ actions }), producerMetadata };
  };
  const enableFunctional = () => {
    functional = true;
    Object.defineProperty(models, "embeddingModel", {
      get: () => "deterministic-test",
    });
    models.invocations.embeddingBatch = async (texts) => ({
      value: texts.map(() => [1, 0]),
      producerMetadata,
    });
    models.form = async (input) => {
      const source = JSON.parse(input) as { evidenceText: string };
      return {
        text: source.evidenceText,
        semanticRole: "reported_fact",
        title: null,
        selectedEntityKeys: [],
        producerMetadata,
      };
    };
    models.consolidate = async () => ({
      value: consolidationSchema.parse({
        actions: [
          {
            action: "skip",
            reason: "Explicit memories already retain the bounded facts",
          },
        ],
      }),
      producerMetadata,
    });
    models.segmentEpisode = async (input) => {
      const source = plan(input);
      return {
        value: episodePartitionSchema.parse({
          action: "partition",
          segments: [
            {
              memberKeys: source.members.map((member) => member.key),
              title: "Observed work session",
              boundaryExplanation:
                "The closed session delimits these observations",
            },
          ],
        }),
        producerMetadata,
      };
    };
    models.synthesizeJournal = async () => ({
      value: journalSynthesisSchema.parse({ action: "no_change" }),
      producerMetadata,
    });
    models.maintainConcepts = async (input) => {
      const source = JSON.parse(input) as {
        cognition: { key: string; text: string; exactSupportKey: string }[];
        tags: { key: string; content: { label: string } }[];
      };
      const focus = source.cognition[0]!;
      const text = focus.text.toLowerCase();
      let label: string | undefined;
      if (text.includes("warm-start readiness")) label = "warm-start readiness";
      else if (text.includes("paired-light check"))
        label = "paired-light check";
      else if (
        text.includes("double-lamp gate") ||
        text.includes("audit recorded both signatures")
      )
        label = "double-lamp gate";
      else if (
        /breakfast|morning/.test(text) &&
        /journal|morning routine/.test(text) &&
        !text.includes("did not adopt")
      )
        label = "breakfast journal routine";
      else if (
        /consumer lease|reader lease|reader-lease|retire the old snapshot|leases for|retire first|retirement.*reclaim|lease-before-reclaim|preserved active consumers/i.test(
          text,
        )
      )
        label = "consumer lease reclamation";
      const relationRules: [RegExp, RegExp][] = [
        [
          /missed setting to the single-person checklist/,
          /private term double-lamp gate/,
        ],
        [/private term double-lamp gate/, /next tide release/],
        [/second block|old backing/, /retired snapshots remain/],
        [/retired snapshots remain/, /retire the old snapshot/],
        [/retire the old snapshot/, /used the analogy to separate/],
        [/used the analogy to separate/, /implemented consumer leases/],
      ];
      const extra = relationRules.flatMap(([from, to]) =>
        from.test(text)
          ? source.cognition
              .filter(
                (candidate) =>
                  candidate.key !== focus.key &&
                  to.test(candidate.text.toLowerCase()),
              )
              .slice(0, 1)
              .map((candidate) => ({
                action: "create_association",
                fromKey: focus.key,
                toKey: candidate.key,
                relation: "assoc.related",
                supportKeys: [focus.exactSupportKey, candidate.exactSupportKey],
                reason:
                  "The supplied cognition describes the mechanism or remedy for this focus",
              }))
          : [],
      );
      if (!label)
        return {
          value: conceptMaintenanceSchema.parse({
            actions: extra.length ? extra : [{ action: "no_change" }],
          }),
          producerMetadata,
        };
      const tag = source.tags.find(
        (candidate) => candidate.content.label === label,
      );
      const supportKeys = [focus.exactSupportKey];
      const attach = {
        action: "attach_tag",
        cognitionKey: focus.key,
        tagKey: tag?.key ?? "new_concept",
        supportKeys,
        reason: "The accepted focus expresses this durable concept",
      };
      const description =
        label === "double-lamp gate" || label === "paired-light check"
          ? "Tide deployment approval requires two independent reviewer signatures recorded on a checklist"
          : label === "consumer lease reclamation"
            ? "Snapshot readers and build cache consumers retain leases until retired assets can be safely reclaimed after release"
            : label === "breakfast journal routine"
              ? "Aya writes the field journal after breakfast in the morning instead of late at night"
              : "Service warm-start readiness requires a successful health probe";
      const actions = tag
        ? [attach]
        : [
            {
              action: "create_tag",
              key: "new_concept",
              cognitionKeys: [focus.key],
              content: { label, description, kind_hint: "practice" },
              supportKeys,
              reason: "The focus expresses a useful continuing practice",
            },
            attach,
          ];
      const gate = source.tags.find(
        (candidate) => candidate.content.label === "double-lamp gate",
      );
      const alias = source.tags.find(
        (candidate) => candidate.content.label === "paired-light check",
      );
      const orderedActions =
        text.includes("alias") && gate
          ? [
              ...(!alias ? [actions[0]!] : []),
              {
                action: "merge_tags",
                survivorKey: gate.key,
                retiredKeys: [alias?.key ?? "new_concept"],
                supportKeys,
                reason: "The focus explicitly identifies equivalent names",
              },
              { ...attach, tagKey: gate.key },
            ]
          : actions;
      return {
        value: conceptMaintenanceSchema.parse({
          actions: [...orderedActions, ...extra].slice(0, 4),
        }),
        producerMetadata,
      };
    };
  };
  return { models, calls, enableFunctional };
}

async function scenario(endpoint: string, token: string) {
  const channel = createInterface({ input: process.stdin });
  const replies = channel[Symbol.asyncIterator]();
  const control = async (request: Record<string, unknown>) => {
    process.stdout.write(JSON.stringify(request) + "\n");
    const response = await replies.next();
    assert(!response.done, "harness control closed");
    return JSON.parse(response.value) as { endpoint?: string };
  };
  const { models, calls, enableFunctional } = proposalModels();
  let app = await createCore({
    kernel: KernelClient.connect(endpoint, token),
    token,
    consumers: [],
    models,
  });
  let publicEndpoint = await app.listen({ host: "127.0.0.1", port: 0 });
  let client = connectNous(publicEndpoint, token);
  const subjectId = randomUUID();
  const advance = async (seconds: number) =>
    control({ command: "advance", subject: subjectId, seconds });
  const maintain = async (maxModelCalls = 4, maxOperations = 4) => {
    const result = await client.cognition.grantMaintenance({
      subjectId,
      maxOperations,
      maxModelCalls,
      maxElapsedMs: 10000,
    });
    for (const operation of result.results)
      assert(
        operation.status !== "rejected_invalid" &&
          (operation.status !== "blocked_dependency" ||
            operation.problemCode === "source_not_settled"),
        JSON.stringify(operation),
      );
    return result;
  };
  try {
    await client.subjects.create({
      subjectId,
      operationId: randomUUID(),
      cognitiveSeed: {
        text: "schema_version = 1",
        format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
        provenance: {},
      },
    });
    const session = await client.cognition.openSession({ subjectId });
    for (const text of [
      "Calibration plan confirmed.",
      "Calibration measurements collected.",
      "Calibration review completed.",
    ]) {
      await client.cognition.observe({
        subjectId,
        sessionId: session.sessionId,
        sourceClass: "message",
        material: {
          case: "inlineText",
          value: { text, mediaType: "text/plain" },
        },
        context: {},
      });
      await advance(2);
    }
    assert.deepEqual(calls, { episode: 0, journal: 0, consolidation: 0 });
    const nearline = await client.cognition.grantMaintenance({
      subjectId,
      maxOperations: 1,
      maxModelCalls: 0,
      maxElapsedMs: 10000,
    });
    assert.equal(nearline.modelCalls, 0);
    assert.equal(
      (await client.memory.listEpisodes({ subjectId, status: "accepted" }))
        .items.length,
      0,
    );
    await advance(2000);
    await maintain();
    assert.equal(
      (await client.memory.listEpisodes({ subjectId, status: "accepted" }))
        .items.length,
      1,
    );
    for (let pass = 0; pass < 4; pass++) {
      await advance(400);
      await maintain();
    }
    const episodes = (
      await client.memory.listEpisodes({ subjectId, status: "accepted" })
    ).items;
    const journals = (
      await client.memory.listJournals({ subjectId, status: "accepted" })
    ).items;
    const memories = (
      await client.memory.list({ subjectId, status: "accepted" })
    ).items;
    assert.equal(episodes.length, 1);
    assert.equal(journals.length, 1);
    assert.equal(memories.length, 1);
    const episode = episodes[0]!;
    const journal = journals[0]!;
    const memory = memories[0]!;
    assert.equal(episode.currentRevision?.members.length, 3);
    assert.equal(episode.currentRevision?.title, "Calibration sequence");
    assert(
      journal.currentRevision?.points.every(
        (point) => point.supports.length > 0,
      ),
    );
    assert(calls.episode > 0 && calls.journal > 0 && calls.consolidation > 0);
    const refs = [
      {
        kind: "episode_revision",
        value: episode.currentRevision!.episodeRevisionId,
      },
      {
        kind: "journal_revision",
        value: journal.currentRevision!.journalRevisionId,
      },
      {
        kind: "memory_revision",
        value: memory.revisionId,
      },
    ];
    await app.close();
    const restarted = await control({ command: "restart" });
    assert(restarted.endpoint);
    app = await createCore({
      kernel: KernelClient.connect(restarted.endpoint, token),
      token,
      consumers: [],
      models,
    });
    publicEndpoint = await app.listen({ host: "127.0.0.1", port: 0 });
    client = connectNous(publicEndpoint, token);
    assert.equal(
      (
        await client.memory.getEpisodeRevision({
          subjectId,
          id: refs[0]!.value,
        })
      ).episodeRevisionId,
      refs[0]!.value,
    );
    assert.equal(
      (
        await client.memory.getJournalRevision({
          subjectId,
          id: refs[1]!.value,
        })
      ).journalRevisionId,
      refs[1]!.value,
    );
    assert.equal(
      (await client.memory.revision({ subjectId, id: refs[2]!.value }))
        .revisionId,
      refs[2]!.value,
    );
    const continuation = await client.cognition.createWorkContext({
      operationId: randomUUID(),
      subjectId,
      purpose: "Continue calibration",
      references: refs,
    });
    assert.deepEqual(continuation.workContext?.references, refs);
    const exact = await client.cognition.query({
      subjectId,
      expression: {
        operation: "atom",
        cues: refs.map((reference) => ({
          cue: { case: "reference", value: reference },
        })),
      },
    });
    assert.equal(exact.hits.length, 3);
    const query = await client.cognition.query({
      subjectId,
      expression: {
        operation: "atom",
        cues: [{ cue: { case: "text", value: "Calibration" } }],
        children: [],
        modifiers: { domains: ["episode", "journal", "memory"], limit: 16 },
      },
    });
    for (const reference of refs)
      assert(
        query.hits.some(
          (hit) =>
            hit.reference?.kind === reference.kind &&
            hit.reference.value === reference.value,
        ),
      );
    const use = {
      subjectId,
      sessionId: session.sessionId,
      consumerRef: "consumer:smoke:longitudinal",
      events: refs.map((reference) => ({
        eventId: randomUUID(),
        reference,
        kind: "referenced",
        occurredAt: timestampFromDate(new Date("2026-10-03T01:00:00Z")),
        context: {},
      })),
    };
    assert.equal((await client.cognition.reportUse(use)).acceptedCount, 3);
    assert.equal((await client.cognition.reportUse(use)).duplicateCount, 3);
    const before = { ...calls };
    await maintain();
    assert.deepEqual(calls, before);
    console.error("Longitudinal smoke passed");
    if (process.env.NOUS_FUNCTIONAL_SMOKE === "1") {
      enableFunctional();
      const results = await runCognitiveFunctional(
        client,
        [
          "baseline-rrf",
          "nous-node-potential-v1",
          "vcp-dtsc-v9.2.1-adapter-v1",
          "vcp-rivermemo-v3.1-adapter-v1",
        ],
        "data/research/cognitive-functional/deterministic-smoke.json",
      );
      const failed = results.filter(
        (result) =>
          result.stage === "formation" &&
          Array.isArray(result.failures) &&
          result.failures.length > 0,
      );
      console.error(
        JSON.stringify({
          functionalFormationFailures: failed.map((result) => ({
            scenario: result.scenario,
            failures: result.failures,
          })),
        }),
      );
      assert.equal(failed.length, 0, "Functional formation contracts");
      const runRoot = await workspaceTemp("smoke", "cli-functional-");
      try {
        await mkdir(runRoot, { recursive: true });
        await writeFile(
          join(runRoot, "core.json"),
          JSON.stringify({ endpoint: publicEndpoint, token }),
          { mode: 0o600 },
        );
        const cli = async (...args: string[]) => {
          try {
            const result = await promisify(execFile)(
              process.execPath,
              [
                "node_modules/tsx/dist/cli.mjs",
                "apps/nous-cli/src/main.ts",
                "--run-root",
                runRoot,
                "--json",
                ...args,
              ],
              { cwd: process.cwd(), maxBuffer: 2 * 1024 * 1024 },
            );
            return {
              code: 0,
              value: JSON.parse(result.stdout) as Record<string, unknown>,
            };
          } catch (error) {
            const failure = error as { code: number; stderr: string };
            return {
              code: failure.code,
              value: JSON.parse(failure.stderr) as Record<string, unknown>,
            };
          }
        };
        const help = await cli("help");
        assert.equal(help.code, 0);
        const incident = results.find(
          (result) =>
            result.stage === "formation" && result.scenario === "incident",
        )!;
        const subject = String(incident.subjectId);
        const ambiguous = await cli(
          "identity",
          "resolve",
          "--subject",
          subject,
          "--kind",
          "entity",
          "--name",
          "Sam Lee",
        );
        assert.notEqual(ambiguous.code, 0);
        assert.equal(ambiguous.value.code, "AMBIGUOUS_REFERENCE");
        const candidates = ambiguous.value.candidates as {
          lexicalRef: string;
          aliases: string[];
        }[];
        assert.equal(candidates.length, 2);
        const chosen = candidates.find((candidate) =>
          candidate.aliases.includes("Tools Sam"),
        )!;
        const resolved = await cli(
          "identity",
          "resolve",
          "--subject",
          subject,
          "--kind",
          "entity",
          "--lexical-ref",
          chosen.lexicalRef,
        );
        assert.equal(resolved.code, 0);
        assert.equal(resolved.value.status, "BOUND");
        const expression = `("Prior consumer lease reclamation relevant to the build cache" && @e(${chosen.lexicalRef})) $memory $limit(8)`;
        const prepared = await cli(
          "query",
          "prepare",
          expression,
          "--subject",
          subject,
        );
        assert.equal(prepared.code, 0, JSON.stringify(prepared.value));
        assert(typeof prepared.value.embeddingText === "string");
        const queried = await cli("query", expression, "--subject", subject);
        assert.equal(queried.code, 0, JSON.stringify(queried.value));
        assert(Array.isArray(queried.value.hits));
        assert(queried.value.hits.length > 0);
        console.error(
          "CLI Agent flow passed: help JSON, ambiguity, candidate LexicalRef, prepare, query",
        );
      } finally {
        await rm(runRoot, { recursive: true, force: true });
      }
      const compatibility = await runSelectedTextCompatibility(
        client,
        "data/research/recovery/text-compatibility-input.json",
        "data/research/cognitive-functional/selected-text-smoke.json",
      );
      assert.equal(compatibility.length, 24);
      for (const item of compatibility) {
        assert(
          item.ranks.every((source) => source.rank > 0),
          `Selected required source omitted: ${item.key}`,
        );
        assert(
          item.ranks.some((source) => source.rank === 1),
          `Selected first relevant source regressed: ${item.key}`,
        );
      }

      console.error(
        JSON.stringify({
          selectedCompatibility: compatibility.map((item) => ({
            key: item.key,
            profile: item.profile,
            ranks: item.ranks,
          })),
        }),
      );
    }
  } finally {
    await app.close();
    channel.close();
    process.stdout.write(JSON.stringify({ command: "done" }) + "\n");
    process.stdin.destroy();
  }
}

const endpoint = process.env.NOUS_LONGITUDINAL_ENDPOINT;
if (endpoint) {
  const token = process.env.NOUS_LONGITUDINAL_TOKEN;
  assert(token);
  await scenario(endpoint, token).catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
} else {
  await new Promise<void>((resolve, reject) => {
    const child = spawn(
      "cargo",
      [
        "test",
        "-p",
        "nous-kernel",
        "--test",
        "longitudinal_smoke",
        "--",
        "--nocapture",
      ],
      {
        stdio: "inherit",
        env: { ...process.env, NOUS_LONGITUDINAL_NODE: process.execPath },
        windowsHide: true,
      },
    );
    child.once("error", reject);
    child.once("exit", (code) =>
      code === 0
        ? resolve()
        : reject(new Error(`longitudinal smoke exited ${code}`)),
    );
  });
}
