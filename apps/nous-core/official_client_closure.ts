import { create } from "@bufbuild/protobuf";
import { TimestampSchema } from "@bufbuild/protobuf/wkt";
import { createConnectTransport } from "@connectrpc/connect-node";
import { createNousClient } from "../../packages/client/src/index.js";
import { createCore } from "./src/server.js";
import { startKernel } from "./src/process.js";
import {
  CognitiveSeedSchema,
  CognitiveRefSchema,
  CreateSubjectRequestSchema,
  EvidenceRefSchema,
  FormMemoryRequestSchema,
  InlineTextSchema,
  MemoryContentSchema,
  ObservationInputSchema,
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

const now = create(TimestampSchema, {
  seconds: BigInt(Math.floor(Date.now() / 1000)),
  nanos: 0,
});

async function main() {
  const root = await mkdtemp(join(tmpdir(), "nous-wave-official-client-"));
  const kernelConfig = join(root, "kernel.toml");
  await writeFile(
    kernelConfig,
    `\n\n[bootstrap.database]\nmode = "managed"\nurl = ""\nmax_connections = 4\nname = "official_client_closure"\ninstall_dir = "postgres-install"\ndata_dir = "postgres-data"\n\n[bootstrap.object_store]\nbackend = "fs"\nroot = "objects"\nmax_upload_bytes = 1048576\n\n[bootstrap.serving]\nroot = "serving"\n\n[settings.capabilities.process]\nmemory = true\n\n[settings.capabilities.subject_defaults]\nmemory = true\n`,
  );

  const kernel = await startKernel(
    process.env.NOUS_WAVE_KERNEL_EXECUTABLE ??
      join(process.cwd(), "target", "debug", "nous-kernel.exe"),
    kernelConfig,
  );
  const coreToken = randomBytes(32).toString("hex");
  const app = await createCore({
    kernel: kernel.client,
    token: coreToken,
    consumers: [
      {
        consumerId: "consumer:official:reference",
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
  try {
    const endpoint = await app.listen({ host: "127.0.0.1", port: 0 });
    const client = createNousClient(
      createConnectTransport({
        baseUrl: endpoint,
        httpVersion: "1.1",
        defaultTimeoutMs: 30_000,
        interceptors: [
          (next) => async (request) => {
            request.header.set("authorization", `Bearer ${coreToken}`);
            return next(request);
          },
        ],
      }),
    );
    const subjectId = "00000000-0000-0000-0000-000000008801";
    const created = await client.subjects.create(
      create(CreateSubjectRequestSchema, {
        subjectId,
        operationId: "00000000-0000-0000-0000-000000008802",
        cognitiveSeed: create(CognitiveSeedSchema, {
          text: "schema_version = 1",
          format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
        }),
        capabilities: create(SubjectCapabilitiesSchema, {
          memory: true,
        }),
      }),
    );
    if (created.subjectId !== subjectId)
      throw new Error("official client Subject mismatch");

    const session = await client.cognition.openSession(
      create(SubjectRequestSchema, { subjectId }),
    );
    const observed = await client.cognition.observe(
      create(ObservationInputSchema, {
        subjectId,
        sessionId: session.sessionId,
        requestId: "00000000-0000-0000-0000-000000008803",
        sourceClass: "message",
        observedAt: now,
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
    if (!observed.occurrenceId)
      throw new Error("official client Observation missing occurrence");

    const formed = await client.memory.form(
      create(FormMemoryRequestSchema, {
        operationId: "00000000-0000-0000-0000-000000008804",
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
          aboutness: [],
          tags: [],
          validTime: create(TemporalExtentSchema, {}),
          formedAt: now,
          epistemicClass: "observed",
        }),
      }),
    );
    if (!formed.revisionId)
      throw new Error("official client Memory missing revision");

    const useRequest = create(ReportUseRequestSchema, {
      subjectId,
      sessionId: session.sessionId,
      consumerRef: "consumer:official:reference",
      events: [
        create(UseEventSchema, {
          eventId: "00000000-0000-0000-0000-000000008805",
          reference: create(CognitiveRefSchema, {
            kind: "memory_revision",
            value: formed.revisionId,
          }),
          kind: "referenced",
          occurredAt: now,
        }),
      ],
    });
    const accepted = await client.cognition.reportUse(useRequest);
    const duplicate = await client.cognition.reportUse(useRequest);
    if (accepted.acceptedCount !== 1 || duplicate.duplicateCount !== 1)
      throw new Error("official client UseEvent idempotency mismatch");
    const readback = await client.memory.get({
      subjectId,
      id: formed.memoryId,
    });
    if (readback.revisionId !== formed.revisionId)
      throw new Error("official client Memory readback mismatch");
    const sessionAfter = await client.cognition.getSession({
      subjectId,
      id: session.sessionId,
    });
    if (sessionAfter.runtimeRevision < 1n)
      throw new Error("official client runtime use was not persisted");
    process.stdout.write(
      `OFFICIAL_CLIENT_CLOSURE_PASS subject=${subjectId} revision=${formed.revisionId} session=${session.sessionId}\n`,
    );
  } finally {
    await app.close();
    await kernel.stop();
    await rm(root, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(
    error instanceof Error ? (error.stack ?? error.message) : String(error),
  );
  process.exitCode = 1;
});
