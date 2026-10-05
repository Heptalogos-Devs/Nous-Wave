import { expect, it } from "vitest";
import {
  sourceIndex,
  supportedRoute,
  type Hit,
  type SourceInspection,
} from "./functional-grading.js";
it("grounds indirect Journal/Memory dependencies without counting cycles as source roots", () => {
  const catalog: SourceInspection = {
    episodes: [
      {
        episode_id: "ep",
        episode_revision_id: "ep-r",
        members: [{ ref_kind: "occurrence", ref_value: "observed" }],
      },
    ],
    journals: [
      {
        journal_id: "j",
        journal_revision_id: "j-r",
        sources: [{ ref_kind: "episode_revision", ref_value: "ep-r" }],
      },
    ],
    memories: [
      {
        memory: "m",
        revision: "m-r",
        supports: [
          { target_ref_kind: "journal_revision", target_ref: "j-r" },
          { target_ref_kind: "memory_revision", target_ref: "m-r" },
        ],
      },
    ],
    schemas: [],
    associations: [],
  };
  const roots = sourceIndex(catalog, {
    subjects: { example: { events: { event: { occurrenceId: "observed" } } } },
  });
  expect([...roots("memory:m")]).toEqual(["example:event"]);
  expect([...roots("memory:unknown")]).toEqual([]);
});
it("requires contiguous activated hops and positive source support, not a numerical path score", () => {
  const edge = (from: string, to: string) => ({
    from: { kind: "memory_revision", id: from },
    to: { kind: "memory_revision", id: to },
    flow: 0.2,
    support: [
      { provenance_root: "external:message:event", polarity: "positive" },
    ],
  });
  const readout = {
    activated_route: [
      "memory_revision:a",
      "memory_revision:b",
      "memory_revision:c",
    ],
    route_evidence: [edge("a", "b"), edge("b", "c")],
  };
  const hit = (): Hit => ({
    reference: { kind: "memory_revision", id: "c" },
    match_evidence: {
      final_rank: 1,
      explanation: JSON.stringify({ topologywave: readout }),
    },
  });
  expect(supportedRoute(hit(), 2)).toBe(true);
  expect(supportedRoute(hit(), 3)).toBe(false);
  readout.route_evidence[1]!.to.id = "other";
  expect(supportedRoute(hit(), 2)).toBe(false);
  readout.route_evidence[1] = edge("b", "c");
  readout.route_evidence[1]!.support[0]!.provenance_root = "";
  expect(supportedRoute(hit(), 2)).toBe(false);
});
