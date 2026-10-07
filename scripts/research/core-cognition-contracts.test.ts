// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "vitest";
import {
  validateManifest,
  stableId,
  assertResume,
  sourceMetrics,
  normalizeHit,
  verifyDigest,
} from "./core-cognition-contracts.js";
const manifest = {
  version: 1,
  pack_id: "fixture",
  role: "calibration",
  sources: [
    {
      id: "early",
      url: "https://example.org/early",
      version_identity: "commit:abc",
      publication_time: "2024-01-01T00:00:00Z",
      raw_sha256: "a".repeat(64),
      source_text_sha256: "b".repeat(64),
      extraction: "identity",
      rights: "CC0",
      raw_path: "data/research/raw",
      text_path: "data/research/text",
    },
  ],
  checkpoints: [{ id: "early", sources: ["early"], maintenance_grants: 1 }],
  queries: [
    {
      id: "q",
      text: "Early fact",
      category: "direct",
      expected_sources: ["early"],
      forbidden_sources: [],
      oracle_notes: "Fixture",
      variants: ["plain"],
    },
  ],
};
describe("semantic research contracts", () => {
  it("rejects unknown/duplicate source membership and invented query supports", () => {
    expect(validateManifest(manifest).pack_id).toBe("fixture");
    expect(() =>
      validateManifest({
        ...manifest,
        checkpoints: [{ ...manifest.checkpoints[0], sources: ["missing"] }],
      }),
    ).toThrow(/source/);
    expect(() =>
      validateManifest({
        ...manifest,
        checkpoints: [...manifest.checkpoints, ...manifest.checkpoints],
      }),
    ).toThrow(/duplicate/);
    expect(() =>
      validateManifest({
        ...manifest,
        queries: [{ ...manifest.queries[0], expected_sources: ["invented"] }],
      }),
    ).toThrow(/source/);
  });
  it("validates raw and text bytes, not only metadata", () => {
    expect(() =>
      verifyDigest(Buffer.from("corrupt"), "a".repeat(64), "raw"),
    ).toThrow(/digest/);
  });
  it("preserves stable UUID identities across duplicate resume", () => {
    expect(stableId("run", "formation/early")).toBe(
      stableId("run", "formation/early"),
    );
    expect(stableId("run", "formation/early")).not.toBe(
      stableId("run", "formation/later"),
    );
  });
  it("rejects resume input drift and every sealed identity mutation", () => {
    const a = {
      manifest: "m",
      oracle: "o",
      runner: "r",
      code: "c",
      model: "p",
      schema: "s",
      config: "g",
      source: "h",
    };
    expect(() => assertResume(a, a, true)).not.toThrow();
    for (const key of Object.keys(a))
      expect(() => assertResume(a, { ...a, [key]: "changed" }, true)).toThrow(
        /identity/,
      );
    expect(() => assertResume(a, { ...a, manifest: "changed" }, false)).toThrow(
      /manifest/,
    );
  });
  it("counts distinct expected sources and explicit interference", () => {
    const m = sourceMetrics(
      [["a"], ["a"], ["noise"], ["b"]],
      ["a", "b"],
      ["noise"],
    );
    expect(m.recall_at_1).toBe(0.5);
    expect(m.recall_at_5).toBe(1);
    expect(m.all_support_at_5).toBe(true);
    expect(m.mrr).toBe(1);
    expect(m.forbidden_sources).toEqual(["noise"]);
  });
  it("normalizes refs and facts without copying third-party text", () => {
    const raw = {
      reference: { kind: "memory", value: "m" },
      revision: { kind: "memory_revision", value: "r" },
      text: "THIRD PARTY RAW SOURCE",
      evidence: [],
      entityRefs: ["entity:source"],
    };
    expect(JSON.stringify(normalizeHit(raw))).not.toContain(raw.text);
    expect(normalizeHit(raw).revision?.value).toBe("r");
  });
});
