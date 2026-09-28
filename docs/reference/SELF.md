# Self Authority 当前接口

本文记录当前 checkout 已实现的 Self Authority 接口；长期 Self 语义仍由 Architecture-Vault 的 Target Design 与 Decisions 维护。

## Owner

- `crates/self-domain`：Self Facet、Narrative Identity 的 value objects、validation 和 revision/lifecycle 规则。
- `crates/self-service`：PostgreSQL Authority mutation、幂等 receipt、support revalidation、purge 和 `SelfDirect` Query contributor。
- `apps/nous-kernel`：private Kernel transport。
- `apps/nous-core` / `packages/client`：TypeScript host 与官方 client 的 Self RPC。

Self 不依赖 `memory-domain`。Memory、Self 与 Cognitive Runtime 通过 `nous-core` shared cognition primitives 和固定 owner composition 连接。

## 当前持久对象

- `CognitiveSeedVersion`：immutable source snapshot；`SubjectSeedAdoption` 记录 `initial` / `import` 及 operation digest。
- `SelfFacet`：固定 `(subject, kind, key)` identity，current revision、object epoch 和 lifecycle。
- `NarrativeIdentity`：固定 `(subject, key)` identity，exact revision references 与独立 supports。

Self Facet kind 为 `identity`、`role`、`capability`、`limitation`、`tendency`、`value`、`preference`。

## 查询

`QueryTarget::SelfCognition` 和 `SelfFacetCue` 进入 `SelfDirect` lane；lane weight 由 `retrieval.rrf.weights.self_direct` registry key 提供。Runtime 在所有 owner lane 生成后统一执行一次 RRF，再按 owner batch validation/materialization。无 cue 时按固定 kind 顺序返回 current facets，随后返回 current Narrative revision。返回前只接受 current、accepted、valid、normal、not-purging 状态。

Self exact object binding 在 QueryPlan binding 时固定 current revision 与 object epoch；执行期间不静默换 head。检索/呈现本身不产生 UseEvent。

## Seed import

`crates/cognitive-seed` 实现 Cognitive Seed v1 TOML deny-unknown-fields parser、Self facet/narrative validation 和 semantic path。Kernel 创建启用 Self 的 Subject 后，以 Subject/Seed/path 派生 deterministic UUIDv5 operation id，独立导入 Self entries；source version 仍先保存，单项 conflict 不回滚其他 entry。

Seed support 持久化为 `SeedSupportRef(seed_version_id, semantic_path)`，semantic path 不从外部 mutation 请求自由指定。
