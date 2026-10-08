// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { expect, it, vi } from "vitest";
import {
  operation,
  sourceUnits,
  type RunState,
  SemanticRun,
} from "./core-cognition-semantic.js";
import type { NousClient } from "@nous-wave/client";
import type { Manifest } from "./core-cognition-contracts.js";
const state = (): RunState => ({
  version: 1,
  run_id: "r",
  identities: {},
  sealed: false,
  started_at: "now",
  subjects: { formed: "s", raw_control: "c" },
  operations: {},
  checkpoints: {},
  phases: {},
});
it("resumes paid stable mutations without repeating completed calls and rejects changed intent", async () => {
  const s = state(),
    call = vi.fn(async () => ({ revision: "exact" })),
    save = vi.fn(async () => {});
  expect(
    await operation(s, "form", { operationId: "fixed" }, true, save, call),
  ).toEqual({ revision: "exact" });
  await operation(s, "form", { operationId: "fixed" }, true, save, call);
  expect(call).toHaveBeenCalledTimes(1);
  await expect(
    operation(s, "form", { operationId: "changed" }, true, save, call),
  ).rejects.toThrow(/changed/);
});
it("does not repeat an uncertain public grant or query after a lost response", async () => {
  const s = state(),
    save = async () => {},
    call = vi.fn(async () => {
      throw new Error("response lost");
    });
  await expect(operation(s, "grant", {}, false, save, call)).rejects.toThrow(
    "response lost",
  );
  await expect(operation(s, "grant", {}, false, save, call)).rejects.toThrow(
    /uncertain/,
  );
  expect(call).toHaveBeenCalledTimes(1);
});
it("splits only source bytes, preserving Unicode, full coverage and deterministic regions", () => {
  const text = "source\n" + "真实原文🙂".repeat(3000) + "\nend\n";
  const units = sourceUnits(text, 12000);
  expect(units.map((u) => u.text).join("")).toBe(text);
  expect(units.every((u) => Buffer.byteLength(u.text) <= 12000)).toBe(true);
  expect(units.at(-1)?.end).toBe(Buffer.byteLength(text));
  expect(sourceUnits(text, 12000)).toEqual(units);
});
it("prepares unquoted checkpoints and keeps the original TextCue in Tag and exploration arms", async () => {
  const prepareQuery = vi.fn().mockResolvedValue({
    boundQuery: JSON.stringify({
      temporal_frame: { clock_now: "2026-10-08T00:00:00Z" },
      authority_watermark: 7,
    }),
  });
  const client = { cognition: { prepareQuery } } as unknown as NousClient;
  const s = state();
  s.freeze = {
    formed: "7",
    raw_control: "0",
    snapshot_digest: "d",
    review_digest: "r",
    tag_bindings: { q: "real-tag" },
  };
  const run = new SemanticRun(
    client,
    {} as Manifest,
    s,
    "data/research/test",
    "unused",
    10,
    1000,
    "unused",
  );
  expect(await run.captureCut("s")).toEqual({
    cut: "2026-10-08T00:00:00Z",
    authority_seq: "7",
  });
  expect(prepareQuery.mock.calls[0]?.[0].nousql).toBe(
    "source qualification checkpoint",
  );
  const q = {
    id: "q",
    text: "Why did he change it?",
    category: "weak-cue",
    expected_sources: [],
    forbidden_sources: [],
    oracle_notes: "original intent",
    variants: [],
  };
  for (const variant of ["tag_direct", "explore"]) {
    const request = await run.queryRequest(q, variant, "formed", [
      { tagId: "real-tag" },
    ] as Parameters<SemanticRun["queryRequest"]>[3]);
    expect(request.expression?.cues).toEqual([
      { cue: { case: "text", value: q.text } },
      { cue: { case: "tagId", value: "real-tag" } },
    ]);
    if (variant === "explore")
      expect(request.expression?.modifiers?.exploration).toBe(
        "bounded_associative",
      );
  }
});
