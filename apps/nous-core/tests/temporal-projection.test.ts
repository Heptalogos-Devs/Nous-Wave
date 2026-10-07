// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { expect, it } from "vitest";
import { compileNousQL } from "../src/nousql/compiler.js";
import { canonical, parse } from "../src/nousql/parser.js";
const now = new Date("2026-10-07T00:00:00Z");
const resolve = async () => ({
  canonical: { kind: "tag", value: "tag-id" },
  lexicalRef: "tag:amber-lotus-cello-river",
});
it("separates root projection from query intent and rejects obsolete directives", async () => {
  const result = await compileNousQL(
    "migration $return(schema,memory)",
    resolve,
    now,
  );
  expect(result.expression.modifiers?.projection?.domains).toEqual([
    "memory",
    "schema",
  ]);
  expect(result.boundCanonical).toContain("$return(memory,schema)");
  for (const domain of [
    "memory",
    "schema",
    "episode",
    "journal",
    "evidence",
    "resource",
  ]) {
    await expect(
      compileNousQL(`migration $${domain}`, resolve, now),
    ).rejects.toThrow("Unknown directive");
  }
  for (const text of [
    "migration $return(memory,memory)",
    "migration $return(cognition,schema)",
    '("a" $return(memory)) || "b"',
    "a $return(memory) $return(schema)",
  ]) {
    await expect(compileNousQL(text, resolve, now)).rejects.toThrow();
  }
});
it("captures relative as-of and multiple independent time axes", async () => {
  const result = await compileNousQL(
    'policy $asof(ago=30d) $history $time(observed,within=7d) $time(occurred,from="2026-01-01T00:00:00Z")',
    resolve,
    now,
  );
  const m = result.expression.modifiers!;
  expect(m.asOf?.seconds).toBe(BigInt(now.getTime() / 1000 - 30 * 86400));
  expect(m.history).toBe(true);
  expect(m.clockNow?.seconds).toBe(BigInt(now.getTime() / 1000));
  expect(m.constraints?.observed?.end?.seconds).toBe(m.clockNow?.seconds);
  expect(m.constraints?.occurred?.start?.seconds).toBe(1767225600n);
  expect(result.boundCanonical).toContain('$asof("2026-09-07T00:00:00.000Z")');
  expect(result.boundCanonical).not.toContain("within=");
  await expect(
    compileNousQL(
      "a $time(observed,within=7d) $time(observed,within=30d)",
      resolve,
      now,
    ),
  ).rejects.toThrow("Duplicate");
  for (const text of [
    'a $asof("2026-01-01")',
    "a $asof(ago=0d)",
    'a $asof("2026-01-01T00:00:00Z",ago=30d)',
    '("a" $history) || "b"',
    '("a" $asof(ago=30d)) || "b"',
  ]) {
    await expect(compileNousQL(text, resolve, now)).rejects.toThrow();
  }
});
it("keeps semantic concept phrases distinct from durable Tag selectors", () => {
  const text = "Recall reclamation #active-reader-reclamation";
  expect(canonical(parse(text))).toBe(text);
  expect(parse(text).cues[0]?.kind).toBe("concept");
  expect(
    parse('Recall reclamation @tag("active-reader reclamation")').cues[0]?.kind,
  ).toBe("selector");
});
