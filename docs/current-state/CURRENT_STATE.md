# Nous Wave 当前实现状态

## 当前组合

当前 checkout 由 TypeScript Core、Rust Kernel、Protobuf contracts、官方 TypeScript Client、PostgreSQL Authority Store、Material、Memory、Cognitive Runtime、Retrieval 和 Serving owners 组成。Memory 在 Runtime composition 中可选；Core 不直接访问 Kernel 数据库。

## Memory Reference Profile 当前实现

- Shared primitives 使用 UUIDv7 一等 ID、`TemporalExtent`、exact revision refs、canonical BLAKE3 operation digest 和 typed error categories。
- Observation 保留 Artifact、Occurrence、SourceRegion、DerivedRepresentation 的独立身份；Occurrence 的 `occurred_time` 支持 Unknown、Instant 和半开 Interval。
- Memory 使用 object-level `CognitiveRole` 与 revision-level `FormationMode`；revision immutable，使用 `object_epoch` 与 Subject `authority_seq` 进行 fencing/order。
- EvidenceRef 必须绑定 ObservationOccurrence；CognitionDependency 指向 exact Memory/Schema revision；Memory lifecycle 分离 acceptance、integrity、suppression、purge 和 accessibility。
- CognitiveSchema、Schema evidence links、AssociationEvidence 和 Serving topology 分离；旧 Anchor/MemoryClass/support-value active path 已删除。
- UseEvent 使用调用方 `event_id`，幂等域为 `(subject, consumer_ref, event_id)`；`presented` 与 meaningful use 分离，结果支持/反驳使用保留。
- QueryPlan 固定 effort budgets、RRF lane plan 和 topology activation；Wave 使用包含 hop 与 remaining budget 的 propagation state；Serving generation 使用 authority watermark 和可重建 artifact。
- Purge 使用 durable receipt 两阶段 fence，删除 cognition-scope 内容与详细 UseEvent，并保留最小 retry tombstone；共享 Observation/Artifact 保留。
- Authority Store 使用五个按语义连续编号的 fresh-schema migrations：`0001_foundation`、`0002_memory_and_schema`、`0003_runtime_identity_material`、`0004_retrieval_indexes` 和 `0005_self_authority`；不保留日期或阶段兼容迁移。

## Self Authority 当前实现

- `nous-self-domain` / `nous-self-service` 已实现 Self Facet 与 Narrative Identity 的 immutable revision、support exactness、object epoch fencing、lifecycle、purge receipt 和 dependency revalidation。
- Self mutation 已在同一 Authority transaction 内递增 `authority_seq` 并写入全部 Serving projection watermarks；Self/Narrative current revision 可进入 lexical/exact Serving projection，并可由 Context resolver 按 exact revision materialize。
- Subject Core 已将 Character Seed active model 替换为 immutable Cognitive Seed version + Subject adoption；旧 `character_seeds` 在当前 migration 中被直接移除。
- Cognitive Query 已使用固定 Memory/Self owner composition，`SelfDirect` 支持 exact、cue 和 deterministic current listing；shared `CognitiveRef`、support/provenance 和 Serving/query hit revision 已扩展到 Self。
- Memory candidate materialization 已改为 owner-batch Authority reads；Schema final revalidation 同时检查 current、accepted、valid、normal 和 not-purging。

## 当前未完成

执行包的 Seed TOML 示例在 Narrative `text =` 行截断，未规定完整 narrative entry contract。自动 Seed parser、Seed→Self import 的 created/unchanged/conflicts 结果和完整初始化映射保持 `SPEC_GAP`，不把 source snapshot storage 误报为 parser 完成。

Social Cognition、Motivation、Desired Condition、Episode/Journal、Heptalogos live integration、自动人格学习和新图算法仍不属于本轮。

## 当前验证证据

- `corepack pnpm check`：PASS（Buf lint、TypeScript check、Vitest）。
- `just verify`：PASS（fmt、source-shape、快速 Self tests、workspace check、两组 Clippy、串行 Rust tests、deny、cargo-shear）。
- `just nextest`：PASS（48/48，单测试线程；测试结束后 embedded PostgreSQL 进程和精确临时根均为 0）。
- `cargo test -p nous-kernel --test self_authority -- --test-threads=1`：PASS；Self facet/narrative create/revision fencing、historical exact binding、SelfDirect/lexical Serving、Self context、Self/Narrative UseEvent resident、lifecycle、dependency revalidation 和 purge projection refresh。
- `cargo test -p nous-kernel --test query_correctness`：PASS；包含 stale lexical、tokenizer hit 与 derived producer association regression scenarios。
- `cargo test -p nous-kernel --test reference_profile`：PASS；使用 embedded PostgreSQL 覆盖 migration、form idempotency/digest conflict、revision、UseEvent、lifecycle、purge 和共享 source retention。
- `cargo deny` 的 advisory gate：PASS；lockfile 中 rustls 已升级到 0.23.45。剩余 duplicate dependency/license allowance 为 warning。
