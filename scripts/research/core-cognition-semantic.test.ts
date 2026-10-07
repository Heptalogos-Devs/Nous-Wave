// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { expect, it, vi } from "vitest";
import {
  operation,
  sourceUnits,
  type RunState,
} from "./core-cognition-semantic.js";
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
