import assert from "node:assert/strict";
import { spawn } from "node:child_process";
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

function proposalModels() {
  const models = new ModelRuntime();
  Object.defineProperty(models.invocations, "capabilities", {
    get: () =>
      ["episode_segmentation", "journal_synthesis", "memory_consolidation"].map(
        (role) => ({
          name: `model.${role}`,
          state: "READY",
          detail: "Executable smoke proposal adapter",
        }),
      ),
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
  return { models, calls };
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
  const { models, calls } = proposalModels();
  let app = await createCore({
    kernel: KernelClient.connect(endpoint, token),
    token,
    consumers: [],
    models,
  });
  let client = connectNous(
    await app.listen({ host: "127.0.0.1", port: 0 }),
    token,
  );
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
    client = connectNous(
      await app.listen({ host: "127.0.0.1", port: 0 }),
      token,
    );
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
