# Rust Crate Owners

`crates/` contains Rust domain and mechanism owners used by Nous Kernel.

This file is the catalog for the crate collection. A crate README explains only that crate's durable boundary; a crate `AGENTS.md` adds only local AI constraints beyond this file and the repository root rules.

| Crate                              | Responsibility                                                                                  |
| ---------------------------------- | ----------------------------------------------------------------------------------------------- |
| `core`                             | Shared identities, typed references, query contracts, and error foundations.                    |
| `subject-core`                     | Subject identity, initialization, and Character Seed lineage.                                   |
| `cognitive-runtime`                | Session continuity, ResidentSet, query orchestration, Resources, checkpoints, and use feedback. |
| `material` / `material-service`    | Artifact, ObservationOccurrence, derived representation, and materialization operations.        |
| `memory-domain` / `memory-service` | Memory, revisions, evidence relations, lifecycle, CognitiveSchema, Tags, and Associations.      |
| `cognitive-retrieval` / `serving`  | Retrieval channels, ranking, and rebuildable Serving generations.                               |
| `authority-store`                  | PostgreSQL canonical schemas, transactions, and persistence access.                             |
| `object-store`                     | Raw and derived object byte storage.                                                            |
| `protocol`                         | Rust bindings for the Protobuf contracts in [`proto/`](../proto/).                              |
| [`self-domain`](self-domain/README.md) / [`self-service`](self-service/README.md) | Self Facet, Narrative Identity, Cognitive Seed adoption, lifecycle, purge, and SelfDirect owner. |

The [current implementation architecture](../docs/architecture/current-implementation.md) explains how these owners compose.
