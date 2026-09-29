import { describe, expect, it } from "vitest";
import { compileNousQL } from "../src/nousql/compiler.js";

describe("R1 host contract", () => {
  it("compiles an exact revision query with current typed fields", async () => {
    const result = await compileNousQL(
      '@e("Alice") $cognitiveRole("declarative")',
      async (_kind, locator) => ({
        canonical: {
          kind: "entity",
          value: `entity:test:${locator.value.toLowerCase()}`,
        },
        lexicalRef: locator.value,
      }),
    );
    expect(result.expression.cues[0]?.cue.case).toBe("reference");
    expect(result.expression.modifiers?.constraints?.cognitiveRoles).toEqual([
      "declarative",
    ]);
  });
});
