// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from "vitest";
import type { connectNousInstance } from "@nous-wave/client/node";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { runCli } from "./commands.js";
import { cliErrorPayload } from "./agent.js";

const exec = promisify(execFile);
describe("Agent CLI protocol", () => {
  it("prepares with explicit context and passes the frozen preparation inspection through", async () => {
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
        '"decision" $return(memory)',
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
      ...prepared,
      boundQuery: { representation: { sha256: "digest" } },
    });
    expect(prepareQuery).toHaveBeenCalledWith({
      subjectId: "s",
      sessionId: "sess",
      workContextId: "work",
      nousql: '"decision" $return(memory)',
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
    expect(parsed).toHaveProperty("commands");
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
