import type { z } from "zod";
import type { ModelRole } from "../roles.js";
import { formationSchema } from "./formation.js";
import { projectionStewardSchema } from "./projection.js";
import { materialInterpretationSchema } from "./material-interpretation.js";
import {
  episodePartitionSchema,
  journalSynthesisSchema,
} from "./longitudinal.js";
import { topologyMaintenanceSchema } from "./topology.js";
import { consolidationSchema } from "./consolidation.js";
import { structuredOutputContract } from "./provider.js";

function contract<T extends z.ZodType>(
  id: string,
  providerName: string,
  owner: T,
) {
  return { id, providerName, owner };
}
const material = contract(
  "material.interpretation",
  "material_interpretation",
  materialInterpretationSchema,
);
const structuredRoleContracts = {
  projection_steward: contract(
    "projection.steward",
    "projection_steward",
    projectionStewardSchema,
  ),
  memory_formation: contract(
    "memory.formation",
    "memory_formation",
    formationSchema,
  ),
  material_structuring: material,
  material_direct_structuring: material,
  episode_segmentation: contract(
    "episode.partition",
    "episode_partition",
    episodePartitionSchema,
  ),
  journal_synthesis: contract(
    "journal.synthesis",
    "journal_synthesis",
    journalSynthesisSchema,
  ),
  topology_maintenance: contract(
    "topology.maintenance",
    "topology_maintenance",
    topologyMaintenanceSchema,
  ),
  memory_consolidation: contract(
    "memory.consolidation",
    "memory_consolidation",
    consolidationSchema,
  ),
} as const;
type StructuredRole = keyof typeof structuredRoleContracts;
export type ModelGenerationOutput<R extends ModelRole> =
  R extends StructuredRole
    ? z.output<(typeof structuredRoleContracts)[R]["owner"]>
    : string;

export function structuredContractForRole(role: ModelRole) {
  return (
    structuredRoleContracts as Partial<
      Record<ModelRole, { id: string; providerName: string; owner: z.ZodType }>
    >
  )[role];
}
export function providerContractForRole(role: ModelRole) {
  const descriptor = structuredContractForRole(role);
  return descriptor
    ? { ...descriptor, ...structuredOutputContract(descriptor.owner) }
    : undefined;
}
