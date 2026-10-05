import { describe, expect, it } from "vitest";
import { create } from "@bufbuild/protobuf";
import { MaintenancePlanSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import { ProducerSignatureSchema } from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { consolidationSchema } from "../model/schemas/consolidation.js";
import { consolidationRequest } from "./consolidation.js";
const occurrence = "10000000-0000-4000-8000-000000000001";
const revision = "20000000-0000-4000-8000-000000000001";
const plan = create(MaintenancePlanSchema, {
  subjectId: "30000000-0000-4000-8000-000000000001",
  authoritySeq: 12n,
  maxConsolidationActions: 8,
  consolidationSource: {
    reference: { kind: "episode_revision", value: revision },
    expectedEpoch: 2n,
  },
  members: [{ key: "member", occurrenceId: occurrence }],
  supports: [
    {
      key: "source",
      support: {
        support: {
          case: "evidence",
          value: {
            occurrenceId: occurrence,
            locator: { case: "wholeOccurrence", value: true },
            supportRole: "direct",
          },
        },
      },
    },
  ],
  candidates: [
    {
      key: "target",
      target: {
        reference: { kind: "memory_revision", value: revision },
        expectedEpoch: 7n,
      },
    },
  ],
});
const content = {
  cognitiveRole: "declarative",
  formationMode: "grounded",
  groundingMemberKey: "member",
  semanticRole: "statement",
  text: "A useful supported fact.",
  title: null,
  supportKeys: ["source"],
  entityKeys: [],
  validTime: { kind: "unknown" },
  epistemicClass: "derived",
};
describe("consolidation exact catalog resolution", () => {
  it("resolves creation, revision and earlier-action relation references without model IDs", () => {
    const proposal = consolidationSchema.parse({
      actions: [
        { action: "create_memory", content },
        {
          action: "revise_memory",
          targetKey: "target",
          intent: "correct",
          content,
        },
        {
          action: "link_relation",
          from: { kind: "action", index: 0 },
          to: { kind: "candidate", key: "target" },
          relation: "elaborates",
        },
      ],
    });
    const request = consolidationRequest(
      plan,
      proposal,
      "40000000-0000-4000-8000-000000000001",
      create(ProducerSignatureSchema),
    );
    const created = request.actions[0]!.action;
    expect(created.case).toBe("createMemory");
    if (created.case !== "createMemory") throw new Error("Expected creation");
    expect(created.value.content?.groundingOccurrenceId).toBe(occurrence);
    const revised = request.actions[1]!.action;
    if (revised.case !== "reviseMemory") throw new Error("Expected revision");
    expect(revised.value.target?.expectedEpoch).toBe(7n);
    expect(request.source?.expectedEpoch).toBe(2n);
    expect(request.actions[2]!.action.case).toBe("linkRelation");
  });
  it("rejects Schema/skip action endpoints before committing a consistency group", () => {
    for (const first of [
      { action: "skip", reason: "No update" },
      {
        action: "create_schema",
        content: {
          title: null,
          structuralClaim: "A bounded recurring pattern.",
          applicability: "Two independently observed events.",
          boundaryDefinition: "Only these events.",
          formationKind: "synthesized",
          entityKeys: [],
          validTime: { kind: "unknown" },
          evidence: [{ role: "support", supportKey: "source" }],
        },
      },
    ]) {
      const proposal = consolidationSchema.parse({
        actions: [
          first,
          {
            action: "link_relation",
            from: { kind: "action", index: 0 },
            to: { kind: "candidate", key: "target" },
            relation: "derived_from",
          },
        ],
      });
      expect(() =>
        consolidationRequest(
          plan,
          proposal,
          "operation",
          create(ProducerSignatureSchema),
        ),
      ).toThrow("Strong Memory relations require Memory action endpoints");
    }
  });
  it("rejects invented support, entity, target and forward relation keys", () => {
    for (const changed of [
      { ...content, supportKeys: ["invented"] },
      { ...content, entityKeys: ["invented"] },
      { ...content, groundingMemberKey: "invented" },
    ]) {
      const proposal = consolidationSchema.parse({
        actions: [{ action: "create_memory", content: changed }],
      });
      expect(() =>
        consolidationRequest(
          plan,
          proposal,
          "operation",
          create(ProducerSignatureSchema),
        ),
      ).toThrow();
    }
    for (const action of [
      {
        action: "revise_memory",
        targetKey: "invented",
        intent: "correct",
        content,
      },
      {
        action: "link_relation",
        from: { kind: "action", index: 1 },
        to: { kind: "candidate", key: "target" },
        relation: "elaborates",
      },
    ])
      expect(() =>
        consolidationRequest(
          plan,
          consolidationSchema.parse({ actions: [action] }),
          "operation",
          create(ProducerSignatureSchema),
        ),
      ).toThrow();
  });
});
