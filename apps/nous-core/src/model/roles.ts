export const roleNames = [
  "projection_steward",
  "memory_formation",
  "episode_segmentation",
  "journal_synthesis",
  "memory_consolidation",
  "topology_maintenance",
  "material_description",
  "material_structuring",
  "material_direct_structuring",
  "query_embedding",
  "query_rerank",
  "speech_transcription",
] as const;
export type ModelRole = (typeof roleNames)[number];
