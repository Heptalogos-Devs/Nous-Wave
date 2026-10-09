// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, it, vi } from "vitest";
import { friendlyOutput } from "./friendly.js";
import type { CliEnvironment } from "./runtime.js";
import { renderText } from "./output.js";

it("keeps real retrieval participation visible without wire identities", async () => {
  const result = await friendlyOutput(
    {
      diagnostics: {
        laneStatus: { dense: "ready", lexical: "ready" },
        candidateCounts: { dense_candidates: 5n },
        trace: { queryId: "33333333-3333-4333-8333-333333333333" },
      },
    },
    { subjectId: "s" } as CliEnvironment,
  );
  expect(result).toEqual({
    diagnostics: {
      laneStatus: { dense: "ready", lexical: "ready" },
      candidateCounts: { dense_candidates: 5n },
    },
  });
});

it("preserves exact UUID/ref bodies and labels without requesting addresses", async () => {
  const id = "33333333-3333-4333-8333-333333333333";
  const bind = vi.fn();
  const bodies = {
    text: id,
    title: `memory:${id}`,
    synopsis: `entity:${id}`,
    message: `source_region:${id}`,
    nested: [{ text: `memory_revision:${id}`, contextText: id }],
  };
  expect(
    await friendlyOutput(bodies, {
      subjectId: "selected",
      client: { identity: { bind } },
    } as unknown as CliEnvironment),
  ).toEqual(bodies);
  expect(bind).not.toHaveBeenCalled();
});

it("keeps exact evidence locators actionable without altering source text or labels", async () => {
  const id = "33333333-3333-4333-8333-333333333333";
  const bind = vi.fn(async (request: { canonical: { kind: string } }) => ({
    lexicalRef:
      request.canonical.kind === "occurrence"
        ? "obs:fakoh-todud-rovor"
        : "src:vahol-gutij-nakur",
  }));
  const env = {
    subjectId: "selected",
    client: { identity: { bind } },
  } as unknown as CliEnvironment;
  const result = await friendlyOutput(
    {
      text: `The original source contains ${id}.`,
      basis: [
        {
          basis: {
            case: "evidence",
            value: {
              occurrenceId: id,
              locator: { case: "sourceRegionId", value: id },
            },
          },
        },
      ],
      sources: [{ reference: { kind: "source_region", value: id } }],
      diagnostics: { queryId: id },
    },
    env,
  );
  expect(result).toMatchObject({
    text: `The original source contains ${id}.`,
    basis: [
      {
        basis: {
          value: {
            occurrenceId: "obs:fakoh-todud-rovor",
            locator: { value: "src:vahol-gutij-nakur" },
          },
        },
      },
    ],
  });
  expect(result).not.toHaveProperty("diagnostics");
  expect(bind).toHaveBeenCalledTimes(2);
  expect(
    bind.mock.calls.every(
      ([request]) => "addressOnly" in request && request.addressOnly === true,
    ),
  ).toBe(true);
  expect(renderText(result)).toContain("src:vahol-gutij-nakur");
});
