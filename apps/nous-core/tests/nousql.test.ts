// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, it } from "vitest";
import { canonical, parse } from "../src/nousql/parser.js";
import { compileNousQL } from "../src/nousql/compiler.js";

const referenceTime = new Date("2026-09-17T00:00:00Z");
const resolve = async (kind: string, locator: { value: string }) => ({
  canonical: { kind, value: `entity:${locator.value}` },
  lexicalRef:
    locator.value === "Alice"
      ? "ent:amber-lotus-cello-river"
      : "ent:quiet-piano-mint-cloud",
});
it("preserves Boolean scope and simplifies soft preference grouping", () => {
  const syntax = parse(
    '(@e("Alice") $source(file)) || (@e("Bob") $source(chat))',
  );
  expect(syntax.directives).toHaveLength(0);
  expect(syntax.children.map((c) => c.directives[0]?.positional[0])).toEqual([
    "file",
    "chat",
  ]);
  expect(canonical(parse('@e("Alice") +("school") -"noise"'))).toBe(
    '@e("Alice") +"school" -"noise"',
  );
  expect(() => parse('~"school"')).toThrow();
  expect(() => parse('"a" "b"')).toThrow();
  expect(() => parse('("a"')).toThrow();
});
it("binds names exactly, sorts joint participants, and rejects duplicate identity", async () => {
  const result = await compileNousQL(
    '@e("Bob","Alice") $return(memory) +"school"',
    resolve,
    referenceTime,
  );
  expect(result.boundCanonical).toContain(
    "@e(ent:amber-lotus-cello-river,ent:quiet-piano-mint-cloud)",
  );
  expect(result.expression.cues).toHaveLength(2);
  expect(
    result.expression.cues.every((cue) => cue.cue.case === "entityRef"),
  ).toBe(true);
  expect(result.expression.modifiers?.preferences[0]?.negative).toBe(false);
  await expect(
    compileNousQL('@e("Alice","Alice")', resolve, referenceTime),
  ).rejects.toThrow("DUPLICATE_ENTITY_PARTICIPANT");
});
it("rejects nested execution controls and requires explicit preference time axes", async () => {
  await expect(
    compileNousQL('("a" $limit(2)) || "b"', resolve, referenceTime),
  ).rejects.toThrow("query root");
  expect(() => parse('"a" +recent')).toThrow("recent(");
  const result = await compileNousQL(
    '"a" +recent(formed) -recent(observed)',
    resolve,
    referenceTime,
  );
  expect(result.expression.modifiers?.preferences.map((p) => p.key)).toEqual([
    "recent:formed",
    "recent:observed",
  ]);
  expect(result.boundCanonical).toContain("+recent(formed)");
});
it("keeps independent time axes and rejects ambiguous or duplicate modifiers", async () => {
  const now = new Date("2026-09-17T00:00:00Z");
  const result = await compileNousQL(
    '"study" $time(observed,within=30d) $effort(deep)',
    resolve,
    now,
  );
  expect(result.expression.modifiers?.constraints?.observed?.end?.seconds).toBe(
    BigInt(now.getTime() / 1000),
  );
  expect(result.expression.modifiers?.constraints?.occurred).toBeUndefined();
  await expect(
    compileNousQL('"study" $limit(3) $limit(4)', resolve, referenceTime),
  ).rejects.toThrow("Duplicate");
  await expect(
    compileNousQL(
      '"study" $time(valid,at="2026-09-17")',
      resolve,
      referenceTime,
    ),
  ).rejects.toThrow("offset");
  await expect(
    compileNousQL('"study" $persona', resolve, referenceTime),
  ).rejects.toThrow("unavailable");
});

it("projects cognition domains at query root", async () => {
  const result = await compileNousQL(
    '("experience" || "context") $return(episode,journal,memory,schema)',
    resolve,
    referenceTime,
  );
  expect(result.expression.modifiers?.projection?.domains).toEqual([
    "memory",
    "schema",
    "episode",
    "journal",
  ]);
});
it("leaves omitted result limits for the Kernel configuration snapshot", async () => {
  const omitted = await compileNousQL(
    '"sensor" $return(memory)',
    resolve,
    referenceTime,
  );
  expect(omitted.expression.modifiers?.limit).toBeUndefined();
  const explicit = await compileNousQL(
    '"sensor" $return(memory) $limit(3)',
    resolve,
    referenceTime,
  );
  expect(explicit.expression.modifiers?.limit).toBe(3);
});

it("binds all names and preferences with the root historical cut", async () => {
  const cuts: (string | undefined)[] = [];
  const resolver = async (
    kind: string,
    locator: { value: string },
    asOf?: Date,
  ) => {
    cuts.push(asOf?.toISOString());
    return {
      canonical: { kind, value: "old" },
      lexicalRef: "tag:amber-lotus-cello-river",
    };
  };
  await compileNousQL(
    '(@tag("Past") || "question") +@tag("Past") $asof(ago=1d)',
    resolver,
    referenceTime,
  );
  expect(cuts).toEqual([
    "2026-09-16T00:00:00.000Z",
    "2026-09-16T00:00:00.000Z",
  ]);
  cuts.length = 0;
  await compileNousQL('@tag("Past") $history', resolver, referenceTime);
  expect(cuts).toEqual([referenceTime.toISOString()]);
});
