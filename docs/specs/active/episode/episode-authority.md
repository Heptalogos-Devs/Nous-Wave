# Episode Authority

状态：IMPLEMENTATION-AUTHORIZING

Episode remains inside the Memory owner. No Episode crate, process, or second persistence owner is introduced.

## Identity and revision

- `EpisodeId` is a stable Subject-owned object identity; `EpisodeRevisionId` is immutable exact organization.
- `track_key` is part of object identity. Changing track creates a new Episode object and an explicit relation.
- revision intent is `resegment` or `reinterpret`; creation has no intent.
- object epoch and current head fence mutation.
- members are ordered exact refs to `Occurrence`, `MemoryRevision`, `CognitiveSchemaRevision`, and later exact `EpisodeRevision`; duplicate or foreign refs fail.
- each revision has at least one support, preserves provenance, and never rewrites its source Observation or Memory.

## Hierarchy and tracks

Parent is an exact EpisodeRevision. It must be same-Subject, non-self, acyclic, and time-contained when both spans are known.

Known comparable sibling spans under the same parent and track are non-overlapping. Unknown/open spans are allowed but cannot prove a complete partition. Validation is transaction-scoped under a Subject/parent/track lock. Different tracks may overlap.

Accepted relation kinds are `split_from`, `merged_from`, `temporal_successor`, and `derived_from`, and always connect exact EpisodeRevision IDs.

Automatic boundary detection, automatic split/merge, ranked Episode retrieval, Journal generation, and Dream remain deferred.

## Lifecycle

Exact get/history/list, suppress/restore, withdraw/reaccept, and purge follow the existing Memory lifecycle dimensions and durable receipt semantics. Suppression, restore, and purge never create a content revision.
