import { qualificationConfig } from "./qualification-config.js";
import { create } from "@bufbuild/protobuf";
import { TimestampSchema } from "@bufbuild/protobuf/wkt";
import { createConnectTransport } from "@connectrpc/connect-node";
import {
  createNousClient,
  type NousClient,
} from "../../packages/client/src/index.js";
import { createCore } from "./src/server.js";
import { startKernel, type KernelProcess } from "./src/process.js";
import {
  CognitiveRefSchema,
  CognitiveSeedSchema,
  CreateEpisodeRequestSchema,
  CreateSubjectRequestSchema,
  CreateWorkContextRequestSchema,
  EpisodeMutationRequestSchema,
  EpisodeMemberSchema,
  EvidenceRefSchema,
  FormMemoryRequestSchema,
  InlineTextSchema,
  MemoryContentSchema,
  ObservationInputSchema,
  QueryExprSchema,
  QueryRequestSchema,
  RevisionSupportSchema,
  SetActiveWorkContextRequestSchema,
  SubjectCapabilitiesSchema,
  SubjectRequestSchema,
  TimeIntervalSchema,
  TemporalExtentSchema,
  UpdateWorkContextRequestSchema,
  WorkContextMutationRequestSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { ModelRuntime } from "./src/model/runtime.js";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import { tmpdir } from "node:os";

const subjectId = "00000000-0000-0000-0000-000000009901";
const instant = create(TimestampSchema, {
  seconds: BigInt(Math.floor(Date.now() / 1000)),
  nanos: 0,
});

type Boot = {
  kernel: KernelProcess;
  app: Awaited<ReturnType<typeof createCore>>;
  client: NousClient;
};

function executable() {
  return (
    process.env.NOUS_WAVE_KERNEL_EXECUTABLE ??
    join(
      process.cwd(),
      "target",
      "debug",
      process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
    )
  );
}

async function boot(configPath: string): Promise<Boot> {
  const kernel = await startKernel(executable(), configPath);
  const token = randomBytes(32).toString("hex");
  const app = await createCore({
    kernel: kernel.client,
    token,
    consumers: [
      {
        consumerId: "consumer:qualification:cognitive-runtime-episode",
        revision: "1",
        memory: "REQUIRED",
        runtime: "REQUIRED",
        resource: "FORBIDDEN",
        maxItems: 32,
        maxTextBytes: 4096,
        materialize: true,
      },
    ],
    models: new ModelRuntime(),
  });
  const endpoint = await app.listen({ host: "127.0.0.1", port: 0 });
  const client = createNousClient(
    createConnectTransport({
      baseUrl: endpoint,
      httpVersion: "1.1",
      defaultTimeoutMs: 30_000,
      interceptors: [
        (next) => async (request) => {
          request.header.set("authorization", "Bearer " + token);
          return next(request);
        },
      ],
    }),
  );
  return { kernel, app, client };
}

async function stop(value: Boot | undefined) {
  if (!value) return;
  await value.app.close();
  await value.kernel.stop();
}

function emptyQuery(sessionId: string) {
  return create(QueryRequestSchema, {
    subjectId,
    sessionId,
    expression: create(QueryExprSchema, { operation: "atom" }),
  });
}

function evidence(occurrenceId: string) {
  return create(RevisionSupportSchema, {
    support: {
      case: "evidence",
      value: create(EvidenceRefSchema, {
        occurrenceId,
        locator: { case: "wholeOccurrence", value: true },
        supportRole: "direct",
      }),
    },
  });
}

function occurrenceMember(occurrenceId: string) {
  return create(EpisodeMemberSchema, {
    reference: create(CognitiveRefSchema, {
      kind: "occurrence",
      value: occurrenceId,
    }),
    role: "member",
  });
}

function at(offsetSeconds: number) {
  return create(TimestampSchema, {
    seconds: instant.seconds + BigInt(offsetSeconds),
    nanos: 0,
  });
}

function span(startSeconds: number, endSeconds: number) {
  return create(TemporalExtentSchema, {
    value: {
      case: "interval",
      value: create(TimeIntervalSchema, {
        start: at(startSeconds),
        end: at(endSeconds),
      }),
    },
  });
}

async function main() {
  const root = await mkdtemp(
    join(tmpdir(), "nous-wave-cognitive-runtime-episode-"),
  );
  const configPath = join(root, "kernel.toml");
  await writeFile(
    configPath,
    qualificationConfig(root, "cognitive_runtime_episode"),
  );
  let current: Boot | undefined;
  try {
    current = await boot(configPath);
    const client = current.client;
    await client.subjects.create(
      create(CreateSubjectRequestSchema, {
        subjectId,
        operationId: "00000000-0000-0000-0000-000000009902",
        cognitiveSeed: create(CognitiveSeedSchema, {
          text: "schema_version = 1",
          format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
        }),
        capabilities: create(SubjectCapabilitiesSchema, { memory: true }),
      }),
    );
    const sessionA = await client.cognition.openSession(
      create(SubjectRequestSchema, { subjectId }),
    );
    const observed = await client.cognition.observe(
      create(ObservationInputSchema, {
        subjectId,
        sessionId: sessionA.sessionId,
        requestId: "00000000-0000-0000-0000-000000009903",
        sourceClass: "message",
        observedAt: instant,
        admit: false,
        material: {
          case: "inlineText",
          value: create(InlineTextSchema, {
            text: "runtime continuity",
            mediaType: "text/plain",
          }),
        },
      }),
    );
    const memory = await client.memory.form(
      create(FormMemoryRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009904",
        subjectId,
        input: create(MemoryContentSchema, {
          cognitiveRole: "declarative",
          formationMode: "grounded",
          groundingOccurrenceId: observed.occurrenceId,
          semanticRole: "fact",
          text: "runtime continuity",
          supports: [evidence(observed.occurrenceId)],
          validTime: create(TemporalExtentSchema, {}),
          formedAt: instant,
          epistemicClass: "observed",
        }),
      }),
    );
    let context = await client.cognition.createWorkContext(
      create(CreateWorkContextRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009905",
        subjectId,
        purpose: "continue the runtime task",
        references: [
          create(CognitiveRefSchema, {
            kind: "memory_revision",
            value: memory.revisionId,
          }),
        ],
      }),
    );
    const contextId = context.workContext?.workContextId;
    if (!contextId) throw new Error("WorkContext identity missing");
    await client.cognition.setActiveWorkContext(
      create(SetActiveWorkContextRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009906",
        subjectId,
        sessionId: sessionA.sessionId,
        expectedRuntimeRevision: sessionA.runtimeRevision,
        workContextId: contextId,
      }),
    );
    const activeQuery = await client.cognition.query(
      emptyQuery(sessionA.sessionId),
    );
    if (
      !activeQuery.hits.some((hit) => hit.revision?.value === memory.revisionId)
    )
      throw new Error(
        "active WorkContext did not surface exact MemoryRevision",
      );
    context = await client.cognition.pauseWorkContext(
      create(WorkContextMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009907",
        subjectId,
        workContextId: contextId,
        expectedRevision: context.workContext?.revision,
      }),
    );
    const pausedSession = await client.cognition.getSession({
      subjectId,
      id: sessionA.sessionId,
    });
    if (pausedSession.activeWorkContextId)
      throw new Error("pause did not clear foreground binding");
    await stop(current);
    current = await boot(configPath);
    const sessionB = await current.client.cognition.openSession(
      create(SubjectRequestSchema, { subjectId }),
    );
    context = await current.client.cognition.resumeWorkContext(
      create(WorkContextMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009908",
        subjectId,
        workContextId: contextId,
        expectedRevision: context.workContext?.revision,
      }),
    );
    await current.client.cognition.setActiveWorkContext(
      create(SetActiveWorkContextRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009909",
        subjectId,
        sessionId: sessionB.sessionId,
        expectedRuntimeRevision: sessionB.runtimeRevision,
        workContextId: contextId,
      }),
    );
    const continued = await current.client.cognition.query(
      emptyQuery(sessionB.sessionId),
    );
    if (
      !continued.hits.some((hit) => hit.revision?.value === memory.revisionId)
    )
      throw new Error(
        "cross-session WorkContext did not surface exact MemoryRevision",
      );

    const episode = await current.client.memory.createEpisode(
      create(CreateEpisodeRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009910",
        subjectId,
        trackKey: "interaction",
        boundaryExplanation: "one bounded interaction",
        experienceTime: create(TemporalExtentSchema, {}),
        formedAt: instant,
        members: [
          create(EpisodeMemberSchema, {
            reference: create(CognitiveRefSchema, {
              kind: "occurrence",
              value: observed.occurrenceId,
            }),
            role: "member",
          }),
        ],
        supports: [evidence(observed.occurrenceId)],
      }),
    );
    const episodeValue = episode.episode;
    const episodeId = episodeValue?.episodeId;
    const firstRevision = episodeValue?.currentRevisionId;
    if (!episodeValue || !episodeId || !firstRevision)
      throw new Error("Episode identity missing");
    const revised = await current.client.memory.reviseEpisode({
      operationId: "00000000-0000-0000-0000-000000009911",
      subjectId,
      episodeId,
      expectedObjectEpoch: episodeValue.objectEpoch,
      intent: "resegment",
      boundaryExplanation: "resegmented interaction",
      experienceTime: create(TemporalExtentSchema, {}),
      formedAt: instant,
      members: [
        create(EpisodeMemberSchema, {
          reference: create(CognitiveRefSchema, {
            kind: "occurrence",
            value: observed.occurrenceId,
          }),
          role: "member",
        }),
      ],
      supports: [evidence(observed.occurrenceId)],
    });
    const history = await current.client.memory.listEpisodeRevisions({
      subjectId,
      episodeId,
    });
    if (
      history.items.length !== 2 ||
      revised.episode?.currentRevisionId === firstRevision
    )
      throw new Error(
        "Episode revision history did not preserve the first revision",
      );
    const exact = await current.client.memory.getEpisodeRevision({
      subjectId,
      id: firstRevision,
    });
    if (exact.episodeId !== episodeId)
      throw new Error("historical Episode revision read mismatch");
    const currentRevision = revised.episode?.currentRevisionId;
    const revisedEpisode = revised.episode;
    if (!revisedEpisode || !currentRevision)
      throw new Error("current Episode revision missing");

    context = await current.client.cognition.updateWorkContext(
      create(UpdateWorkContextRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009912",
        subjectId,
        workContextId: contextId,
        expectedRevision: context.workContext?.revision,
        purpose: "continue the runtime task with an Episode",
        references: [
          create(CognitiveRefSchema, {
            kind: "memory_revision",
            value: memory.revisionId,
          }),
          create(CognitiveRefSchema, {
            kind: "episode_revision",
            value: currentRevision,
          }),
        ],
      }),
    );
    const episodeRuntimeQuery = await current.client.cognition.query(
      emptyQuery(sessionB.sessionId),
    );
    if (
      !episodeRuntimeQuery.hits.some(
        (hit) => hit.reference?.value === currentRevision,
      )
    )
      throw new Error("WorkContext did not surface exact EpisodeRevision");

    const boundedEpisode = await current.client.memory.createEpisode(
      create(CreateEpisodeRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009913",
        subjectId,
        trackKey: "segmentation",
        boundaryExplanation: "bounded parent segment",
        experienceTime: span(100, 110),
        formedAt: instant,
        members: [occurrenceMember(observed.occurrenceId)],
        supports: [evidence(observed.occurrenceId)],
      }),
    );
    if (!boundedEpisode.episode) throw new Error("bounded Episode missing");
    let overlapRejected = false;
    try {
      await current.client.memory.createEpisode(
        create(CreateEpisodeRequestSchema, {
          operationId: "00000000-0000-0000-0000-000000009914",
          subjectId,
          trackKey: "segmentation",
          boundaryExplanation: "overlapping sibling",
          experienceTime: span(105, 115),
          formedAt: instant,
          members: [occurrenceMember(observed.occurrenceId)],
          supports: [evidence(observed.occurrenceId)],
        }),
      );
    } catch {
      overlapRejected = true;
    }
    if (!overlapRejected)
      throw new Error("same-track overlapping Episode was accepted");
    const parallelEpisode = await current.client.memory.createEpisode(
      create(CreateEpisodeRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009915",
        subjectId,
        trackKey: "parallel",
        boundaryExplanation: "overlap on a separate track",
        experienceTime: span(105, 115),
        formedAt: instant,
        members: [occurrenceMember(observed.occurrenceId)],
        supports: [evidence(observed.occurrenceId)],
      }),
    );
    if (!parallelEpisode.episode)
      throw new Error("different-track overlapping Episode was rejected");

    const child = await current.client.memory.createEpisode(
      create(CreateEpisodeRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009916",
        subjectId,
        trackKey: "child",
        parentEpisodeRevisionId: currentRevision,
        boundaryExplanation: "child Episode",
        experienceTime: create(TemporalExtentSchema, {}),
        formedAt: instant,
        members: [occurrenceMember(observed.occurrenceId)],
        supports: [evidence(observed.occurrenceId)],
      }),
    );
    const childValue = child.episode;
    if (!childValue) throw new Error("child Episode missing");
    let hierarchyRejected = false;
    try {
      await current.client.memory.reviseEpisode({
        operationId: "00000000-0000-0000-0000-000000009917",
        subjectId,
        episodeId: childValue.episodeId,
        expectedObjectEpoch: childValue.objectEpoch,
        intent: "resegment",
        parentEpisodeRevisionId: childValue.currentRevisionId,
        boundaryExplanation: "cyclic child",
        experienceTime: create(TemporalExtentSchema, {}),
        formedAt: instant,
        members: [occurrenceMember(observed.occurrenceId)],
        supports: [evidence(observed.occurrenceId)],
      });
    } catch {
      hierarchyRejected = true;
    }
    if (!hierarchyRejected)
      throw new Error("Episode hierarchy cycle was accepted");

    await stop(current);
    current = await boot(configPath);
    const restartedEpisode = await current.client.memory.getEpisode({
      subjectId,
      id: episodeId,
    });
    if (restartedEpisode.episode?.currentRevisionId !== currentRevision)
      throw new Error("restart changed the current Episode revision");
    const restartedQuery = await current.client.cognition.query(
      emptyQuery(sessionB.sessionId),
    );
    if (
      !restartedQuery.hits.some(
        (hit) => hit.reference?.value === currentRevision,
      )
    )
      throw new Error("restart lost WorkContext EpisodeRevision recall");

    const ended = await current.client.cognition.endWorkContext(
      create(WorkContextMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009918",
        subjectId,
        workContextId: contextId,
        expectedRevision: context.workContext?.revision,
      }),
    );
    const endedSession = await current.client.cognition.getSession({
      subjectId,
      id: sessionB.sessionId,
    });
    if (endedSession.activeWorkContextId)
      throw new Error("end did not clear foreground binding");
    let endedResumeRejected = false;
    try {
      await current.client.cognition.resumeWorkContext(
        create(WorkContextMutationRequestSchema, {
          operationId: "00000000-0000-0000-0000-000000009919",
          subjectId,
          workContextId: contextId,
          expectedRevision: ended.workContext?.revision,
        }),
      );
    } catch {
      endedResumeRejected = true;
    }
    if (!endedResumeRejected) throw new Error("ended WorkContext resumed");

    const suppressedEpisode = await current.client.memory.suppressEpisode(
      create(EpisodeMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009920",
        subjectId,
        episodeId,
        expectedObjectEpoch: restartedEpisode.episode?.objectEpoch,
      }),
    );
    const restoredEpisode = await current.client.memory.restoreEpisode(
      create(EpisodeMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009921",
        subjectId,
        episodeId,
        expectedObjectEpoch: suppressedEpisode.episode?.objectEpoch,
      }),
    );
    if (restoredEpisode.episode?.currentRevisionId !== currentRevision)
      throw new Error("Episode restore created a content revision");
    await current.client.memory.purgeEpisode(
      create(EpisodeMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009922",
        subjectId,
        episodeId,
        expectedObjectEpoch: restoredEpisode.episode?.objectEpoch,
      }),
    );
    let purgeReadRejected = false;
    try {
      await current.client.memory.getEpisode({ subjectId, id: episodeId });
    } catch {
      purgeReadRejected = true;
    }
    if (!purgeReadRejected)
      throw new Error("purged Episode remained ordinarily readable");

    process.stdout.write(
      "COGNITIVE_RUNTIME_EPISODE_PASS subject=" +
        subjectId +
        " memory=" +
        memory.revisionId +
        " work_context=" +
        contextId +
        " episode=" +
        episodeId +
        " historical_revision=" +
        firstRevision +
        " restart=true cross_session=true episode_revision=true overlap=true hierarchy=true lifecycle=true purge=true\n",
    );
  } finally {
    await stop(current);
    await rm(root, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(
    error instanceof Error ? (error.stack ?? error.message) : String(error),
  );
  process.exitCode = 1;
});
