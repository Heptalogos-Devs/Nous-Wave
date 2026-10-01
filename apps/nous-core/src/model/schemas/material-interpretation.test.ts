import { describe, expect, it } from "vitest";
import { canonicalDigest } from "../../digest.js";
import {
  materialInterpretationSchema,
  materialInterpretationSchemaDigest,
  structuredMaterialResult,
} from "./material-interpretation.js";
import { structuredOutputContract } from "./provider.js";
const materialInterpretationJsonSchema = structuredOutputContract(
  materialInterpretationSchema,
).providerSchema;

const output = () => ({
  summary: "A rocket launch with uncertain commentary.",
  coverage: {
    visual: "observed",
    audio: "not_available",
    embedded_text: "not_available",
  },
  observations: [
    {
      kind: "event",
      content: "Rocket launch",
      basis: "direct",
      start_ms: 0,
      end_ms: 1000,
      support_keys: ["S000"],
    },
  ],
  mentions: [
    {
      surface: "rocket",
      category: "object",
      role: null,
      support_keys: ["S000"],
    },
  ],
  embedded_text: [],
  speech: [],
  interpretations: [
    {
      content: "May be a historical launch",
      status: "tentative",
      support_keys: ["S000"],
    },
  ],
  uncertainties: [
    { issue: "No audio available", alternatives: [], support_keys: ["S000"] },
  ],
});
const source = {
  visual: true,
  audio: false,
  catalog: { S000: { kind: "source_region", value: "source-id" } },
  durationMs: 1000,
};
describe("Material interpretation contract", () => {
  it("exports one strict provider schema with required nullable scalars and a shape-sensitive digest", () => {
    expect(materialInterpretationJsonSchema.additionalProperties).toBe(false);
    expect(materialInterpretationJsonSchema.required).toContain(
      "uncertainties",
    );
    expect(
      materialInterpretationSchema.parse(JSON.parse(JSON.stringify(output())))
        .mentions[0]?.role,
    ).toBeNull();
    const changed = materialInterpretationSchema.extend({
      additional_fact: materialInterpretationSchema.shape.summary,
    });
    expect(canonicalDigest(materialInterpretationJsonSchema)).toBe(
      materialInterpretationSchemaDigest,
    );
    expect(
      canonicalDigest(changed.toJSONSchema({ target: "draft-7" })),
    ).not.toBe(materialInterpretationSchemaDigest);
    expect(() =>
      materialInterpretationSchema.parse({ ...output(), unexpected: "fact" }),
    ).toThrow();
  });
  it("rejects unsupported direct facts, forged keys and source-time overflow", () => {
    const forged = output();
    forged.observations[0]!.support_keys = ["D999"];
    expect(() => structuredMaterialResult(forged, source)).toThrow("Unknown");
    forged.observations[0]!.support_keys = [];
    expect(() => structuredMaterialResult(forged, source)).toThrow(
      "requires source support",
    );
    const late = output();
    late.observations[0]!.end_ms = 1001;
    expect(() => structuredMaterialResult(late, source)).toThrow("timestamp");
  });
  it("does not turn absent modalities into model observations", () => {
    expect(() =>
      structuredMaterialResult(output(), {
        ...source,
        visual: false,
        audio: true,
      }),
    ).toThrow("visual");
    const invented = output();
    invented.coverage.audio = "observed";
    expect(() => structuredMaterialResult(invented, source)).toThrow("audio");
  });
  it("projects repeatably without promoting uncertainty or tentative interpretations", () => {
    const { text, structuredPayload } = structuredMaterialResult(
      output(),
      source,
    );
    expect(structuredMaterialResult(output(), source).text).toBe(text);
    expect(structuredPayload.observations).toMatchObject([
      { supports: [{ kind: "source_region", value: "source-id" }] },
    ]);
    expect(JSON.stringify(structuredPayload)).not.toContain("support_keys");
    expect(text).toContain("[tentative] May be a historical launch");
    expect(text).toContain("Uncertainties:\n- No audio available");
  });
});
