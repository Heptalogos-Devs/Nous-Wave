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
  CreateSubjectRequestSchema,
  CueSchema,
  EvidenceRefSchema,
  FormMemoryRequestSchema,
  InlineTextSchema,
  MemoryContentSchema,
  MemoryMutationRequestSchema,
  ObservationInputSchema,
  QueryExprSchema,
  QueryRequestSchema,
  ReportUseRequestSchema,
  RevisionSupportSchema,
  SubjectCapabilitiesSchema,
  SubjectRequestSchema,
  TemporalExtentSchema,
  UseEventSchema,
} from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { ModelRuntime } from "./src/model/runtime.js";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import { tmpdir } from "node:os";

const subjectId = "00000000-0000-0000-0000-000000009801";
const occurrenceRequestId = "00000000-0000-0000-0000-000000009802";
const formOperationId = "00000000-0000-0000-0000-000000009803";
const useEventId = "00000000-0000-0000-0000-000000009804";
const consumerRef = "consumer:qualification:memory-reference";

type Boot = {
  kernel: KernelProcess;
  app: Awaited<ReturnType<typeof createCore>>;
  client: NousClient;
};

const instant = create(TimestampSchema, {
  seconds: BigInt(Math.floor(Date.now() / 1000)),
  nanos: 0,
});

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
        consumerId: consumerRef,
        revision: "1",
        memory: "REQUIRED",
        runtime: "REQUIRED",
        resource: "FORBIDDEN",
        maxItems: 16,
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
          request.header.set("authorization", `Bearer ${token}`);
          return next(request);
        },
      ],
    }),
  );
  return { kernel, app, client };
}

async function stop(booted: Boot | undefined) {
  if (!booted) return;
  await booted.app.close();
  await booted.kernel.stop();
}

function queryExpression(text?: string) {
  return create(QueryExprSchema, {
    operation: "atom",
    cues: text
      ? [
          create(CueSchema, {
            cue: { case: "text", value: text },
          }),
        ]
      : [],
  });
}

async function query(
  client: NousClient,
  sessionId: string | undefined,
  text?: string,
) {
  return client.cognition.query(
    create(QueryRequestSchema, {
      subjectId,
      sessionId,
      expression: queryExpression(text),
    }),
  );
}

function hitMatches(
  hit: { reference?: { value: string }; revision?: { value: string } },
  memoryId: string,
  revisionId: string,
) {
  return (
    hit.reference?.value === memoryId ||
    hit.reference?.value === revisionId ||
    hit.revision?.value === revisionId
  );
}

async function main() {
  const root = await mkdtemp(join(tmpdir(), "nous-wave-memory-reference-"));
  const configPath = join(root, "kernel.toml");
  await writeFile(configPath, qualificationConfig(root, "memory_reference"));

  let booted: Boot | undefined;
  try {
    booted = await boot(configPath);
    const client = booted.client;
    const created = await client.subjects.create(
      create(CreateSubjectRequestSchema, {
        subjectId,
        operationId: "00000000-0000-0000-0000-000000009801",
        cognitiveSeed: create(CognitiveSeedSchema, {
          text: "schema_version = 1",
          format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
        }),
        capabilities: create(SubjectCapabilitiesSchema, { memory: true }),
      }),
    );
    if (created.subjectId !== subjectId)
      throw new Error("Subject identity mismatch");

    const sessionA = await client.cognition.openSession(
      create(SubjectRequestSchema, { subjectId }),
    );
    const sessionB = await client.cognition.openSession(
      create(SubjectRequestSchema, { subjectId }),
    );
    const observed = await client.cognition.observe(
      create(ObservationInputSchema, {
        subjectId,
        sessionId: sessionA.sessionId,
        requestId: occurrenceRequestId,
        sourceClass: "message",
        observedAt: instant,
        admit: true,
        material: {
          case: "inlineText",
          value: create(InlineTextSchema, {
            text: "Alice is present",
            mediaType: "text/plain",
          }),
        },
      }),
    );
    const formed = await client.memory.form(
      create(FormMemoryRequestSchema, {
        operationId: formOperationId,
        subjectId,
        input: create(MemoryContentSchema, {
          cognitiveRole: "declarative",
          formationMode: "grounded",
          groundingOccurrenceId: observed.occurrenceId,
          semanticRole: "fact",
          text: "Alice is present",
          supports: [
            create(RevisionSupportSchema, {
              support: {
                case: "evidence",
                value: create(EvidenceRefSchema, {
                  occurrenceId: observed.occurrenceId,
                  locator: { case: "wholeOccurrence", value: true },
                  supportRole: "direct",
                }),
              },
            }),
          ],
          validTime: create(TemporalExtentSchema, {}),
          formedAt: instant,
          epistemicClass: "observed",
        }),
      }),
    );
    if (!formed.memoryId || !formed.revisionId)
      throw new Error("Memory identity missing");
    if (
      !formed.supports.some(
        (support) =>
          support.support.case === "evidence" &&
          support.support.value.occurrenceId === observed.occurrenceId,
      )
    )
      throw new Error("Memory provenance does not trace to Observation");

    const recalled = await query(client, undefined, "Alice is present");
    if (
      !recalled.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("public Query did not recall the formed Memory");

    const useRequest = create(ReportUseRequestSchema, {
      subjectId,
      sessionId: sessionA.sessionId,
      consumerRef,
      events: [
        create(UseEventSchema, {
          eventId: useEventId,
          reference: create(CognitiveRefSchema, {
            kind: "memory_revision",
            value: formed.revisionId,
          }),
          kind: "referenced",
          occurredAt: instant,
        }),
      ],
    });
    const accepted = await client.cognition.reportUse(useRequest);
    const duplicate = await client.cognition.reportUse(useRequest);
    if (accepted.acceptedCount !== 1 || duplicate.duplicateCount !== 1)
      throw new Error("UseEvent retry was not idempotent");

    const residentA = await query(client, sessionA.sessionId);
    const residentB = await query(client, sessionB.sessionId);
    if (
      !residentA.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("Session A did not expose its resident Memory");
    if (
      residentB.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("Session B observed Session A residency");

    await stop(booted);
    booted = await boot(configPath);
    const restarted = await booted.client.memory.get({
      subjectId,
      id: formed.memoryId,
    });
    if (restarted.revisionId !== formed.revisionId)
      throw new Error("restart changed the Authority revision");
    const recalledAfterRestart = await query(
      booted.client,
      undefined,
      "Alice is present",
    );
    if (
      !recalledAfterRestart.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("restart lost public Query recall");

    const suppressed = await booted.client.memory.suppress(
      create(MemoryMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009805",
        subjectId,
        memoryId: formed.memoryId,
        expectedObjectEpoch: restarted.objectEpoch,
      }),
    );
    const hidden = await query(booted.client, undefined, "Alice is present");
    if (
      hidden.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("suppressed Memory remained query-visible");
    const restored = await booted.client.memory.restore(
      create(MemoryMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009806",
        subjectId,
        memoryId: formed.memoryId,
        expectedObjectEpoch: suppressed.objectEpoch,
      }),
    );
    if (restored.revisionId !== formed.revisionId)
      throw new Error("restore created a content revision");
    const visibleAgain = await query(
      booted.client,
      undefined,
      "Alice is present",
    );
    if (
      !visibleAgain.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("restore did not restore public recall");

    await booted.client.memory.purge(
      create(MemoryMutationRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000009807",
        subjectId,
        memoryId: formed.memoryId,
        expectedObjectEpoch: restored.objectEpoch,
      }),
    );
    const purgedRetry = await booted.client.cognition.reportUse(useRequest);
    if (purgedRetry.duplicateCount !== 1)
      throw new Error("purged UseEvent retry lost its idempotency receipt");
    const afterPurge = await query(
      booted.client,
      undefined,
      "Alice is present",
    );
    if (
      afterPurge.hits.some((hit) =>
        hitMatches(hit, formed.memoryId, formed.revisionId),
      )
    )
      throw new Error("purged Memory remained query-visible");

    process.stdout.write(
      `MEMORY_REFERENCE_PASS subject=${subjectId} memory=${formed.memoryId} revision=${formed.revisionId} sessions=2 restart=true suppress_restore=true purge=true traceback=true\n`,
    );
  } finally {
    await stop(booted);
    await rm(root, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(
    error instanceof Error ? (error.stack ?? error.message) : String(error),
  );
  process.exitCode = 1;
});
