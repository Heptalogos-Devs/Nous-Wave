// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, it } from "vitest";
import { canonicalDigest } from "../src/digest.js";
import { roleNames } from "../src/model/roles.js";
import {
  providerContractForRole,
  structuredContractForRole,
} from "../src/model/schemas/contracts.js";

it("owns every structured generation role and exports its production schema identity", () => {
  expect(
    roleNames.filter((role) => structuredContractForRole(role)).sort(),
  ).toEqual([
    "concept_maintenance",
    "episode_segmentation",
    "journal_synthesis",
    "material_direct_structuring",
    "material_structuring",
    "memory_consolidation",
    "memory_formation",
    "projection_steward",
    "query_concept_enrichment",
  ]);
  const identities = new Set<string>();
  for (const role of roleNames) {
    const contract = providerContractForRole(role);
    if (!contract) continue;
    identities.add(contract.id);
    expect(contract.digest).toBe(canonicalDigest(contract.providerSchema));
    expect(contract.providerSchema.title).toBeTruthy();
    expect(contract.providerSchema.description).toBeTruthy();
    expect(contract.providerName).toMatch(/^[A-Za-z0-9_-]+$/);
  }
  expect(identities.size).toBe(8);
  expect(providerContractForRole("material_structuring")?.digest).toBe(
    providerContractForRole("material_direct_structuring")?.digest,
  );
  const formation = providerContractForRole("memory_formation")!.providerSchema;
  expect(formation.required).toEqual([
    "text",
    "semanticRole",
    "title",
    "selectedEntityKeys",
  ]);
  expect(formation.additionalProperties).toBe(false);
});

it("query concept output accepts only local keys and bounded ephemeral hypotheses", () => {
  const schema = structuredContractForRole("query_concept_enrichment")!.owner;
  expect(
    schema.safeParse({
      existing_tags: [{ key: "c0", strength: 0.8 }],
      novel_concepts: [{ text: "reader reclamation" }],
    }).success,
  ).toBe(true);
  for (const existing_tags of [
    [{ key: "tag:invented", strength: 1 }],
    [{ key: "c0", strength: 2 }],
    [{ key: "c0", strength: 0.8, tag_id: "invented" }],
  ]) {
    expect(
      schema.safeParse({ existing_tags, novel_concepts: [] }).success,
    ).toBe(false);
  }
});
