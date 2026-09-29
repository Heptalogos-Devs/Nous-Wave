# Nous Wave 当前实现状态

## 当前施工授权

当前唯一新增代码施工线是 [Structural Rebase — Memory Reference Profile R1](../plans/active/2026-09-29-structural-rebase-memory-reference-profile.md)。Self Authority 与 Configuration/Social 是待移除的 pre-production executable slices；长期语义仍由 Architecture-Vault 维护。

## 当前组合

当前 checkout 由 TypeScript Core、Rust Kernel、Protobuf contracts、官方 TypeScript Client、Configuration Service、Subject/Material、Memory、Self、Social、Cognitive Runtime、Retrieval、Serving 和 PostgreSQL Authority Store 组成。Memory、Self、Social 都按 Process/Subject capabilities optional composition；Core 不直接访问 Kernel 数据库。

## Configuration 与能力组合

- `crates/configuration-service` 提供 owner registry、typed descriptor、deployment/system/subject override、权限 tier、immutable `ConfigSnapshot`、BLAKE3 subset digest 和 idempotent set/clear persistence。
- 配置解析顺序为 reference default → deployment file → persisted system → persisted subject；`RestartProcess` 变更保持 active snapshot 不变并返回 pending restart。
- `0006_configuration.sql` 持久化 configuration state/overrides/mutation receipts/typed subject capabilities。Subject metadata 与运行 settings 分离。
- 默认 process/新 Subject 是 Memory=true、Self=false、Social=false；Memory-only query/Serving/restart 基础路径已有 focused coverage。
- Accessibility、RRF/query budget、Wave quality/seed weights、Serving policy 和 Social formation/query policy 已由 owner registry 提供 reference defaults。

## Memory Reference Profile 当前实现

- Shared primitives 使用 UUIDv7 一等 ID、`TemporalExtent`、exact revision refs、canonical BLAKE3 operation digest 和 typed error categories。
- Observation 保留 Artifact、Occurrence、SourceRegion、DerivedRepresentation 的独立身份；Occurrence 的 `occurred_time` 支持 Unknown、Instant 和半开 Interval。
- Memory 使用 object-level `CognitiveRole` 与 revision-level `FormationMode`；revision immutable，使用 `object_epoch` 与 Subject `authority_seq` 进行 fencing/order。
- EvidenceRef 必须绑定 ObservationOccurrence；CognitionDependency 指向 exact Memory/Schema revision；Memory lifecycle 分离 acceptance、integrity、suppression、purge 和 accessibility。
- CognitiveSchema、Schema evidence links、AssociationEvidence 和 Serving topology 分离；旧 Anchor/MemoryClass/support-value active path 已删除。
- UseEvent 使用调用方 `event_id`，幂等域为 `(subject, consumer_ref, event_id)`；`presented` 与 meaningful use 分离，结果支持/反驳使用保留。
- QueryPlan 固定 effort budgets、RRF lane plan 和 topology activation；Wave 使用包含 hop 与 remaining budget 的 propagation state；Serving generation 使用 authority watermark 和可重建 artifact。
- Purge 使用 durable receipt 两阶段 fence，删除 cognition-scope 内容与详细 UseEvent，并保留最小 retry tombstone；共享 Observation/Artifact 保留。
- Observation admission 使用 subject-safe atomic ResidentSet batch；batch 内重复 ref 合并，no-op 不推进 runtime revision，eviction 与 admission 在同一 transaction；持久坏 resident ref 显式返回错误。
- Authority Store 使用九个按语义连续编号的 fresh-schema migrations：`0001_foundation`–`0005_self_authority`、`0006_configuration`、`0007_social_cognition`、`0008_memory_reference_closure` 和 `0009_configuration_receipt_digests`；不保留日期或阶段兼容迁移。
- Configuration mutation receipt 冻结对应 system/subject resolved `active_digest` 与 `desired_digest`；旧 pre-production receipt 在 `0009` 中清空，不伪造历史结果。
- TemporalLane 对 instant 与 interval 使用不同 half-open overlap 条件；instant 位于查询区间起点时不再被错误排除。

## Self Authority 当前实现

- `nous-self-domain` / `nous-self-service` 已实现 Self Facet 与 Narrative Identity 的 immutable revision、support exactness、object epoch fencing、lifecycle、purge receipt 和 dependency revalidation。
- Self mutation 已在同一 Authority transaction 内递增 `authority_seq` 并写入全部 Serving projection watermarks；Self/Narrative current revision 可进入 lexical/exact Serving projection，并可由 Context resolver 按 exact revision materialize。
- Subject Core 已将 Character Seed active model 替换为 immutable Cognitive Seed version + Subject adoption；旧 `character_seeds` 在当前 migration 中被直接移除。
- Cognitive Seed v1 parser 已拒绝 unknown fields，完整 Narrative multiline、Self key/scope 和 semantic path 已纳入 Seed contract；启用 Self 的 Subject 会使用 deterministic UUIDv5 operation id 导入 facet/narrative entries。
- Cognitive Query 由 Runtime 统一聚合 Shared lexical/dense、Memory/Self/Social direct lanes 并执行一次 RRF；`LaneOutput` generic contract 由 `cognitive-retrieval` 持有，owner 只提供 direct lane/batch materialization。
- Serving generation 的 text/topology source composition 同时受 process 与 Subject capabilities约束；Self/Social owner不直接读取 Tantivy/usearch。
- Memory candidate materialization 已改为 owner-batch Authority reads；Schema final revalidation 同时检查 current、accepted、valid、normal 和 not-purging。

## Social Cognition 当前实现

- `crates/social-domain` 拥有 SocialParty、Relation Type view/degree/temporal semantics、Relationship/LanguageConvention value validation 和 scope canonicalization。
- `crates/social-service` 持久化 Relation Type、directed Relationship revisions、LanguageConvention revisions/formation evidence、policy digest 和 lifecycle/purge mutations；formation policy 从同一 ConfigSnapshot 解析，并在 Authority-backed acceptance 中验证 support ownership、external actor 和 provenance independence。
- `0007_social_cognition.sql` 建立 Social Authority tables/indexes 与 purge tombstones；Social semantic refs 已加入 shared `CognitiveRef`、Authority exact binding、Serving source 和 Query direct lanes。
- Proto、generated Rust/TypeScript 和 official Client 已包含 Social create/revise/lifecycle contracts；Social seed import 使用 `social.relation-types/*`、`social.relationships/*`、`social.conventions/*` semantic paths。

## 当前未完成或未运行

- Social revise/lifecycle/suppression、LanguageConvention actor/root acceptance 和 Seed→Social created/unchanged/conflict 已有 focused evidence；purge retry/source retention、disabled deferred、restart/rebuild 和完整多来源矩阵仍为 `NOT_RUN`，不等同于 PASS。
- Memory Reference Profile Closure R2 已有 deterministic multi-session closure scenario、versioned fixture/oracle、full-scale structural fixture、observed performance baseline、真实 Kernel subprocess restart 和 Core+Kernel+official TypeScript Client 场景；provider-unavailable、suppressed-exact、validation-budget 三项 targeted diagnostic oracle 已 PASS，完整 lane/category oracle 与 Wave benchmark gate 仍为 `NOT_RUN`。
- R2 scale fixture 当前记录 10,000 current Memory revisions、3,000 historical revisions、30,000 AssociationEvidence、2,000 entity refs、1,000 Tags、200 CognitiveSchemas；query count 与 RSS 尚未 instrument。
- Motivation、Desired Condition、Episode/Journal、Heptalogos live integration、自动人格学习和新图算法仍不属于本轮。

## 当前验证证据

- `corepack pnpm check`：PASS（Buf lint、TypeScript check、Vitest）。
- `just verify`：PASS（本轮 configuration/social 扩展后的 fmt、source-shape、Self focused tests、workspace check、两组 Clippy、串行 Rust tests、deny、cargo-shear）。
- `just nextest`：PASS（exit code 0，单测试线程；测试结束后 embedded PostgreSQL 进程和精确临时根均为 0）。
- `cargo test -p nous-kernel --test self_authority -- --test-threads=1`：PASS；Self facet/narrative create/revision fencing、historical exact binding、SelfDirect/lexical Serving、Self context、Self/Narrative UseEvent resident、lifecycle、dependency revalidation 和 purge projection refresh。
- `cargo test -p nous-kernel --test query_correctness`：PASS；包含 stale lexical、tokenizer hit 与 derived producer association regression scenarios。
- `cargo test -p nous-kernel --test reference_profile`：PASS；使用 embedded PostgreSQL 覆盖 migration、form idempotency/digest conflict、revision、UseEvent、lifecycle、purge 和共享 source retention。
- `cargo deny` 的 advisory gate：PASS；lockfile 中 rustls 已升级到 0.23.45。剩余 duplicate dependency/license allowance 为 warning。
- R2 focused evidence：`runtime_residency` 4/4、`configuration` 2/2、`social_cognition` 4/4、`memory_reference_closure` 1/1、`scale_fixture` 1/1、`nous-cognitive-retrieval` unit 8/8；这些是本轮实际运行边界，不替代最终 `just verify`/`nextest`。
- 最终 R2 门禁：`corepack pnpm generate` PASS、`corepack pnpm check` PASS、`just verify` PASS、`just nextest` PASS（当前提交范围均已重跑）；`just dupes` FAIL（32 exact groups，max 16），`just osv` FAIL（`osv-scanner` 不可用）。
