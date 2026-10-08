// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from "vitest";
import type { connectNousInstance } from "@nous-wave/client/node";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { runCli } from "./commands.js";
import { cliErrorPayload } from "./agent.js";
import { workspaceTemp } from "../../../scripts/workspace.js";

const exec = promisify(execFile);
it("invalidates result indices when the next query fails", async () => {
  const root = await workspaceTemp("tests", "cli-failed-query-");
  const query = vi
    .fn()
    .mockResolvedValueOnce({
      queryId: "first",
      hits: [
        {
          reference: {
            kind: "memory_revision",
            value: "33333333-3333-4333-8333-333333333333",
          },
          text: "old result",
        },
      ],
    })
    .mockRejectedValueOnce(new Error("query rejected"));
  const revision = vi.fn();
  const client = {
    cognition: { query },
    memory: { revision },
  } as unknown as Awaited<ReturnType<typeof connectNousInstance>>;
  const connect = vi.fn().mockResolvedValue(client);
  const globals = [
    "--run-root",
    "/tmp/nous",
    "--instance-root",
    root,
    "--subject",
    "s",
  ];
  await runCli(["query", "first", ...globals], connect);
  await expect(
    runCli(["query", "invalid", ...globals], connect),
  ).rejects.toThrow("query rejected");
  await expect(
    runCli(["show", "result:1", ...globals], connect),
  ).rejects.toMatchObject({ code: "RESULT_SUBJECT_MISMATCH" });
  expect(revision).not.toHaveBeenCalled();
});
it("reads source text through its Material owner with explicit byte bounds", async () => {
  const id = "33333333-3333-4333-8333-333333333333";
  const materialize = vi.fn().mockResolvedValue({
    reference: { kind: "source_region", value: id },
    mediaType: "text/markdown",
    content: new TextEncoder().encode("真实来源原文"),
    totalBytes: 18n,
    rangeStart: 0n,
    rangeEnd: 18n,
    partial: false,
    evidence: [],
    degradation: [],
  });
  const client = { material: { materialize } } as unknown as Awaited<
    ReturnType<typeof connectNousInstance>
  >;
  const result = await runCli(
    [
      "read",
      `source_region:${id}`,
      "--run-root",
      "/tmp/nous",
      "--subject",
      "s",
    ],
    vi.fn().mockResolvedValue(client),
  );
  expect(result).toHaveProperty("data.text", "真实来源原文");
  expect(materialize).toHaveBeenCalledWith({
    subjectId: "s",
    reference: { kind: "source_region", value: id },
    maxBytes: 65536n,
  });
});
it("continues canonical Material references returned by observe and trace", async () => {
  const id = "33333333-3333-4333-8333-333333333333";
  const sourceRegion = vi.fn().mockResolvedValue({ sourceRegionId: id });
  const resolve = vi.fn();
  const client = {
    material: { sourceRegion },
    identity: { resolve },
  } as unknown as Awaited<ReturnType<typeof connectNousInstance>>;
  const result = await runCli(
    [
      "show",
      `source_region:${id}`,
      "--run-root",
      "/tmp/nous",
      "--subject",
      "s",
    ],
    vi.fn().mockResolvedValue(client),
  );
  expect(result).toHaveProperty("data.sourceRegionId", id);
  expect(sourceRegion).toHaveBeenCalledWith({ subjectId: "s", id });
  expect(resolve).not.toHaveBeenCalled();
});
describe("Agent CLI protocol", () => {
  it("prepares with explicit context and summarizes the frozen inspection", async () => {
    const prepared = {
      boundQuery: '{"representation":{"sha256":"digest"}}',
      embeddingText: "complete intent",
      embeddingRequired: true,
    };
    const prepareQuery = vi.fn().mockResolvedValue(prepared);
    const query = vi.fn();
    const client = { cognition: { prepareQuery, query } } as unknown as Awaited<
      ReturnType<typeof connectNousInstance>
    >;
    const connect = vi.fn().mockResolvedValue(client);
    const result = await runCli(
      [
        "query",
        "inspect",
        "decision $return(memory)",
        "--instance-root",
        "/tmp/nous-cli-agent-tests",
        "--run-root",
        "/tmp/nous",
        "--subject",
        "s",
        "--session",
        "sess",
        "--work-context",
        "work",
      ],
      connect,
    );
    expect(result).toMatchObject({
      schemaVersion: "nous.cli.v1",
      data: {
        intent: "decision $return(memory)",
        representation: { sha256: "digest", chars: 15 },
      },
    });
    expect(prepareQuery).toHaveBeenCalledWith({
      subjectId: "s",
      sessionId: "sess",
      workContextId: "work",
      nousql: "decision $return(memory)",
    });
    expect(query).not.toHaveBeenCalled();
  });

  it("preserves domain ambiguity candidates through identity and RPC errors", async () => {
    const result = {
      status: "AMBIGUOUS_REFERENCE",
      candidates: [
        { canonical: { kind: "entity", value: "Alice-1" } },
        { canonical: { kind: "entity", value: "Alice-2" } },
      ],
    };
    const client = {
      identity: { resolve: vi.fn().mockResolvedValue(result) },
    } as unknown as Awaited<ReturnType<typeof connectNousInstance>>;
    await expect(
      runCli(
        [
          "identity",
          "resolve",
          "--name",
          "Alice",
          "--kind",
          "entity",
          "--instance-root",
          "/tmp/nous-cli-agent-tests",
          "--run-root",
          "/tmp/nous",
          "--subject",
          "s",
        ],
        vi.fn().mockResolvedValue(client),
      ),
    ).rejects.toMatchObject({
      code: "AMBIGUOUS_REFERENCE",
      candidates: result.candidates,
    });
    expect(
      cliErrorPayload(
        new Error(
          '{"code":"UNRESOLVED_QUERY_REFERENCE","details":[{"start":0}]}',
        ),
      ),
    ).toMatchObject({
      code: "UNRESOLVED_QUERY_REFERENCE",
      details: [{ start: 0 }],
    });
  });

  it("supports discovery without an instance and emits parse errors only on stderr", async () => {
    const entry = "apps/nous-cli/src/main.ts";
    const help = await exec(process.execPath, [
      "node_modules/tsx/dist/cli.mjs",
      entry,
      "help",
      "--json",
    ]);
    const parsed: unknown = JSON.parse(help.stdout);
    expect(parsed).toHaveProperty("data.commands");
    expect(help.stdout).toContain('"command":"query prepare|inspect"');
    expect(help.stderr).toBe("");
    const failure: unknown = await exec(process.execPath, [
      "node_modules/tsx/dist/cli.mjs",
      entry,
      "--nonexistent",
      "--json",
    ]).catch((error: unknown) => error);
    expect(failure).toMatchObject({ code: 1, stdout: "" });
    if (
      !failure ||
      typeof failure !== "object" ||
      !("stderr" in failure) ||
      typeof failure.stderr !== "string"
    )
      throw new Error("missing process stderr");
    const payload: unknown = JSON.parse(failure.stderr);
    expect(payload).toMatchObject({
      code: "INVALID_ARGUMENT",
      details: [],
      candidates: [],
    });
  });
});

it("formation passes explicit Tag identities without an inference model", async () => {
  const formFromObservation = vi.fn().mockResolvedValue({ memoryId: "memory" });
  const resolve = vi.fn().mockResolvedValue({
    status: "BOUND",
    candidates: [
      {
        canonical: {
          kind: "tag",
          value: "33333333-3333-4333-8333-333333333333",
        },
      },
    ],
  });
  const client = {
    model: { formFromObservation },
    identity: { resolve },
  } as unknown as Awaited<ReturnType<typeof connectNousInstance>>;
  await runCli(
    [
      "form",
      "occ",
      "--instance-root",
      "/tmp/nous-cli-agent-tests",
      "--run-root",
      "/tmp/nous",
      "--subject",
      "s",
      "--tag",
      "tag:amber-lotus-cello-river,tag:44444444-4444-4444-8444-444444444444",
    ],
    vi.fn().mockResolvedValue(client),
  );
  expect(resolve).toHaveBeenCalledWith({
    subjectId: "s",
    kind: "tag",
    locator: { case: "lexicalRef", value: "tag:amber-lotus-cello-river" },
  });
  expect(formFromObservation).toHaveBeenCalledWith(
    expect.objectContaining({
      explicitTags: [
        "33333333-3333-4333-8333-333333333333",
        "44444444-4444-4444-8444-444444444444",
      ],
    }),
  );
});

it("routes explicit Tag creation and bounded maintenance grants to Concept/Host APIs", async () => {
  const createTag = vi.fn().mockResolvedValue({ tagId: "tag" });
  const grantMaintenance = vi.fn().mockResolvedValue({ operations: 1 });
  const client = {
    concepts: { createTag },
    cognition: { grantMaintenance },
  } as unknown as Awaited<ReturnType<typeof connectNousInstance>>;
  const connect = vi.fn().mockResolvedValue(client);
  await runCli(
    [
      "tag",
      "create",
      "--instance-root",
      "/tmp/nous-cli-agent-tests",
      "--run-root",
      "/tmp/nous",
      "--subject",
      "s",
      "--name",
      "active-reader reclamation",
      "--description",
      "Wait until all active readers leave",
      "--operation-id",
      "op",
    ],
    connect,
  );
  expect(createTag).toHaveBeenCalledWith(
    expect.objectContaining({
      subjectId: "s",
      operationId: "op",
      tag: {
        label: "active-reader reclamation",
        description: "Wait until all active readers leave",
        kindHint: undefined,
        origin: "host_explicit",
      },
    }),
  );
  await runCli(
    [
      "maintenance",
      "grant",
      "--instance-root",
      "/tmp/nous-cli-agent-tests",
      "--run-root",
      "/tmp/nous",
      "--subject",
      "s",
      "--max-operations",
      "2",
      "--max-model-calls",
      "0",
    ],
    connect,
  );
  expect(grantMaintenance).toHaveBeenCalledWith({
    subjectId: "s",
    maxOperations: 2,
    maxModelCalls: 0,
    maxElapsedMs: 30000,
  });
  await expect(
    runCli(
      [
        "maintenance",
        "grant",
        "--instance-root",
        "/tmp/nous-cli-agent-tests",
        "--run-root",
        "/tmp/nous",
        "--subject",
        "s",
        "--max-operations",
        "33",
      ],
      connect,
    ),
  ).rejects.toMatchObject({ code: "INVALID_ARGUMENT" });
});
it("uses exact typed revisions and caller-stable feedback identity", async () => {
  const reportUse = vi.fn().mockResolvedValue({ acceptedCount: 1 });
  const client = { cognition: { reportUse } } as unknown as Awaited<
    ReturnType<typeof connectNousInstance>
  >;
  const connect = vi.fn().mockResolvedValue(client);
  const ref = "journal_revision:33333333-3333-4333-8333-333333333333";
  await runCli(
    [
      "use",
      ref,
      "--instance-root",
      "/tmp/nous-cli-agent-tests",
      "--run-root",
      "/tmp/nous",
      "--subject",
      "s",
      "--kind",
      "result_supported",
      "--event-id",
      "event",
      "--query-id",
      "44444444-4444-4444-8444-444444444444",
      "--occurred-at",
      "2026-10-07T00:00:00Z",
    ],
    connect,
  );
  expect(reportUse).toHaveBeenCalledWith(
    expect.objectContaining({
      events: [
        {
          eventId: "event",
          queryId: "44444444-4444-4444-8444-444444444444",
          kind: "result_supported",
          reference: {
            kind: "journal_revision",
            value: "33333333-3333-4333-8333-333333333333",
          },
          occurredAt: { seconds: 1791331200n, nanos: 0 },
        },
      ],
    }),
  );
  await expect(
    runCli(
      [
        "use",
        "memory:33333333-3333-4333-8333-333333333333",
        "--instance-root",
        "/tmp/nous-cli-agent-tests",
        "--run-root",
        "/tmp/nous",
        "--subject",
        "s",
      ],
      connect,
    ),
  ).rejects.toMatchObject({ code: "INVALID_ARGUMENT" });
});

it("serves NousQL JSON guidance without requiring a daemon or instance", async () => {
  const connect = vi.fn();
  const result = await runCli(["help", "nousql", "--json"], connect);
  expect(connect).not.toHaveBeenCalled();
  expect(result).toHaveProperty("data.concepts");
  expect(result).toHaveProperty("data.selectors");
  expect(result).toHaveProperty("data.time");
  expect(result).toHaveProperty("data.exploration");
  expect(result).toHaveProperty("data.projection");
  expect(result).toHaveProperty("data.workflow");
  const text = JSON.stringify(result);
  expect(text).toContain("Strong recommendation");
  expect(text).toContain("original pronoun");
  for (const token of [
    "$time",
    "$asof",
    "$history",
    "$explore",
    "$return",
    "@tag",
    "#concept",
  ])
    expect(text).toContain(token);
});
