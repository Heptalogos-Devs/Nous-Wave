# Rust Crate Index

| Crate | Responsibility | Entry | Important dependencies | Read when |
| --- | --- | --- | --- | --- |
| `core` | IDs, exact refs, query DTOs, shared semantic errors and provenance primitives | `crates/core/src/lib.rs` | serde, uuid, chrono | Changing shared contracts or identity |
| `protocol` | Generated Rust Protobuf/tonic bindings | `crates/protocol/src/generated/` | prost, tonic | Inspecting wire bindings; edit `proto/` instead |
| `persistence` | PostgreSQL pool, transactions, fresh schema, mutation envelopes, receipts and projection watermarks | `crates/persistence/src/lib.rs` | sqlx, core | Changing durable persistence mechanics |
| `object-store` | Immutable blob/object storage mechanism | `crates/object-store/src/lib.rs` | OpenDAL | Changing artifact storage |
| `configuration` | Typed registry, snapshots, overrides and configuration receipts | `crates/configuration/src/lib.rs` | persistence, core | Changing configuration resolution |
| `subject` | Subject identity, capability and Cognitive Seed adoption | `crates/subject/src/lib.rs` | persistence, configuration, runtime, object-store | Changing Subject lifecycle or seed source |
| `material` | Artifact/Observation/DerivedRepresentation semantics and operations | `crates/material/src/lib.rs` | persistence, runtime, object-store | Changing evidence admission or materialization |
| `memory` | Memory, CognitiveSchema, Episode, Journal, Tag, AssociationEvidence and Authority/query operations | `crates/memory/src/lib.rs` | persistence, runtime | Changing cognitive Authority, Episode or Memory query |
| `runtime` | Session, ResidentSet, WorkContext, UseEvent, QueryPlan, lane contract and fixed fusion | `crates/runtime/src/lib.rs` | persistence, configuration, core | Changing Runtime or query semantics |
| `retrieval` | Lexical/dense/topology projections, generations and providers | `crates/retrieval/src/lib.rs` | runtime, persistence, object-store | Changing Serving mechanics or experimental lanes |

`crates/protocol/src/generated/` is derived output. Change canonical `.proto` sources and regenerate.

- [Proquint 地址编码](persistence/src/proquint.rs)
- [Rust 实现](README.md)
