import { expect, it } from "vitest";
import { canonicalDigest } from "../src/digest.js";
import { roleNames } from "../src/model/configuration.js";
import {
  providerContractForRole,
  structuredRoleContracts,
} from "../src/model/schemas/contracts.js";

it("owns every structured generation role and exports its production schema identity", () => {
  expect(Object.keys(structuredRoleContracts).sort()).toEqual([
    "episode_segmentation",
    "journal_synthesis",
    "material_direct_structuring",
    "material_structuring",
    "memory_consolidation",
    "memory_formation",
    "projection_steward",
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
  expect(identities.size).toBe(6);
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
