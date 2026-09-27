# Nous Wave 当前实现状态

## 当前组合

当前 checkout 由 TypeScript Core、Rust Kernel、Protobuf contracts、官方 TypeScript Client、PostgreSQL Authority Store、Material、Memory、Cognitive Runtime、Retrieval 和 Serving owners 组成。Memory 在 Runtime composition 中可选；Core 不直接访问 Kernel 数据库。

## R1 Reference Profile 当前实现

- Shared primitives 使用 UUIDv7 一等 ID、`TemporalExtent`、exact revision refs、canonical BLAKE3 operation digest 和 typed error categories。
- Observation 保留 Artifact、Occurrence、SourceRegion、DerivedRepresentation 的独立身份；Occurrence 的 `occurred_time` 支持 Unknown、Instant 和半开 Interval。
- Memory 使用 object-level `CognitiveRole` 与 revision-level `FormationMode`；revision immutable，使用 `object_epoch` 与 Subject `authority_seq` 进行 fencing/order。
- EvidenceRef 必须绑定 ObservationOccurrence；CognitionDependency 指向 exact Memory/Schema revision；Memory lifecycle 分离 acceptance、integrity、suppression、purge 和 accessibility。
- CognitiveSchema、Schema evidence links、AssociationEvidence 和 Serving topology 分离；旧 Anchor/MemoryClass/support-value active path 已删除。
- UseEvent 使用调用方 `event_id`，幂等域为 `(subject, consumer_ref, event_id)`；`presented` 与 meaningful use 分离，结果支持/反驳使用保留。
- QueryPlan 固定 effort budgets、RRF lane plan 和 topology activation；Wave 使用包含 hop 与 remaining budget 的 propagation state；Serving generation 使用 authority watermark 和可重建 artifact。
- Purge 使用 durable receipt 两阶段 fence，删除 cognition-scope 内容与详细 UseEvent，并保留最小 retry tombstone；共享 Observation/Artifact 保留。

## 仍不属于本轮实现范围

Self、Social Cognition、Motivation、Desired Condition、Episode/Journal、Heptalogos live integration、自动 identity matcher、自动 Schema split/merge decision、完整 source/privacy purge 和生产级 learned retrieval benchmark 仍由后续阶段负责。当前实现不把这些目标领域标记为已完成。

## 当前验证证据

- `corepack pnpm check`：PASS（Buf lint、TypeScript check、Vitest）。
- `just verify`：PASS（fmt、workspace check、Clippy、Rust tests、deny、cargo-shear、source-shape）。
- `cargo test -p nous-kernel --test reference_profile`：PASS；使用 embedded PostgreSQL 覆盖 migration、form idempotency/digest conflict、revision、UseEvent、lifecycle、purge 和共享 source retention。
- `cargo deny` 的 advisory gate：PASS；lockfile 中 rustls 已升级到 0.23.45。剩余 duplicate dependency/license allowance 为 warning。
