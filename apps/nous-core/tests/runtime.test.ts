// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it, vi } from "vitest";
import { ContextCompiler } from "../src/cognition/context.js";
import { ModelRuntime } from "../src/model/runtime.js";
import { ProjectionPlanner } from "../src/cognition/projection.js";
import type { ConsumerPolicy, Projection, Segment } from "../src/domain.js";

const segment = (id: string, text = id): Segment => ({
  segmentId: id,
  text,
  semanticRole: "evidence",
  sourceRefs: [{ kind: "memory", value: id }],
  evidence: [],
  authority: "subject_cognition",
  stability: "EPOCH_STABLE",
});
const projection = (...segments: Segment[]): Projection => ({
  projectionId: "p",
  consumerId: "test",
  sourceRuntimeRevision: 1n,
  segments,
  degradation: [],
});
const policy: ConsumerPolicy = {
  consumerId: "test",
  revision: "1",
  memory: "PREFERRED",
  resource: "FORBIDDEN",
  runtime: "OPTIONAL",
  maxItems: 10,
  maxTextBytes: 20,
  materialize: true,
};

describe("managed context synchronization", () => {
  it("appends only to a synchronized immutable prefix and resets on source revision", () => {
    const compiler = new ContextCompiler(256);
    const key = { subjectId: "s", sessionId: "session", consumerId: "test" };
    const initial = compiler.compile(key, "1", projection(segment("a")));
    expect(initial.kind).toBe("RESET");
    const append = compiler.compile(
      key,
      "1",
      projection(segment("a"), segment("b")),
      initial.cursor,
    );
    expect(append.kind).toBe("APPEND");
    expect(append.projection.segments.map((s) => s.segmentId)).toEqual(["b"]);
    expect(
      compiler.compile(
        key,
        "1",
        projection(segment("a", "revised"), segment("b")),
        append.cursor,
      ).kind,
    ).toBe("RESET");
    expect(
      compiler.compile(
        { ...key, consumerId: "another" },
        "1",
        projection(segment("a")),
        initial.cursor,
      ).kind,
    ).toBe("RESET");
    expect(
      new ContextCompiler(256).compile(
        key,
        "1",
        projection(segment("a")),
        initial.cursor,
      ).kind,
    ).toBe("RESET");
  });
});
describe("projection authority boundaries", () => {
  it("applies the cognition consumer policy to Schema contributions", async () => {
    const planner = new ProjectionPlanner(
      new Map([["test", { ...policy, memory: "FORBIDDEN" } as ConsumerPolicy]]),
    );
    const input = {
      ...segment("schema", "supported pattern"),
      sourceRefs: [
        { kind: "cognitive_schema_revision", value: "schema revision" },
      ],
    };
    const result = await planner.build(projection(input), {
      maxItems: 5,
      maxTextBytes: 20,
    });
    expect(result.segments).toEqual([]);
  });
  it("keeps projected segment identity stable across fresh contribution IDs and resets on evidence drift", async () => {
    const planner = new ProjectionPlanner(new Map([["test", policy]]));
    const compiler = new ContextCompiler(256);
    const key = { subjectId: "s", sessionId: "session", consumerId: "test" };
    const original = segment("source", "same content");
    const first = await planner.build(projection(original), {
      maxItems: 5,
      maxTextBytes: 20,
    });
    const second = await planner.build(
      projection({ ...original, segmentId: "fresh contribution ID" }),
      { maxItems: 5, maxTextBytes: 20 },
    );
    expect(second.segments[0]?.segmentId).toBe(first.segments[0]?.segmentId);
    expect(second.projectionId).toBe(first.projectionId);
    const initial = compiler.compile(key, "1", first);
    const unchanged = compiler.compile(key, "1", second, initial.cursor);
    expect(unchanged.kind).toBe("APPEND");
    expect(unchanged.projection.segments).toEqual([]);
    expect(unchanged.cursor).toEqual(initial.cursor);
    const changed = await planner.build(
      projection({
        ...original,
        evidence: [
          {
            basisRole: "direct",
            reference: { kind: "occurrence", value: "different source" },
          },
        ],
      }),
      { maxItems: 5, maxTextBytes: 20 },
    );
    expect(compiler.compile(key, "1", changed, unchanged.cursor).kind).toBe(
      "RESET",
    );
  });
  it("rejects invented model refs and preserves deterministic source selection", async () => {
    const model = new ModelRuntime();
    vi.spyOn(model.invocations, "profile").mockReturnValue({} as never);
    vi.spyOn(model.invocations, "generate").mockResolvedValue({
      value: { selectedIds: ["invented"] },
    } as never);
    const result = await new ProjectionPlanner(
      new Map([["test", policy]]),
      model,
    ).build(projection(segment("a")), { maxItems: 5, maxTextBytes: 20 });
    expect(result.segments.map((s) => s.sourceRefs)).toEqual([
      [{ kind: "memory", value: "a" }],
    ]);
    expect(result.degradation[0]?.code).toBe("steward_rejected");
  });
  it("enforces bytes without splitting UTF-8 and keeps required sources", async () => {
    const required = { ...policy, memory: "REQUIRED" as const };
    const planner = new ProjectionPlanner(
      new Map([["test", required]]),
      new ModelRuntime(),
    );
    const result = await planner.build(projection(segment("a", "中文中文")), {
      maxItems: 1,
      maxTextBytes: 5,
    });
    expect(result.segments[0]?.text).toBe("中");
    expect(result.segments).toHaveLength(1);
  });
});
