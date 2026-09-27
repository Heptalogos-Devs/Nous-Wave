# 06 — Implementation Map

状态：IMPLEMENTATION GUIDE
目标路径：`docs/specs/active/cognitive-retrieval/06-implementation-map.md`

本文件不新增语义，只把 Spec 映射到当前代码 owner。

## 1. Query / BoundQuery

当前：

- `crates/cognitive-runtime/src/query/plan.rs`
- `crates/cognitive-runtime/src/query/orchestrate.rs`
- `crates/memory-service/src/query.rs`

目标：

- Cognitive Runtime拥有 query binding / QueryPlan；
- Memory Service编排 Memory/Schema lane providers；
- lane provider返回统一 LaneOutput；
-删除 arbitrary first-N candidate query。

建议新增模块：

```text
crates/cognitive-runtime/src/query/bind.rs
crates/cognitive-runtime/src/query/types.rs

crates/memory-service/src/query/
  mod.rs
  exact.rs
  runtime.rs
  entity.rs
  lexical.rs
  dense.rs
  temporal.rs
  schema.rs
  topology.rs
  validate.rs
```

具体私有拆分可调整。

## 2. Fusion

当前：

- `crates/cognitive-retrieval/src/ranking.rs`
- `crates/memory-service/src/query.rs`

目标：

唯一 production RRF owner：

`crates/cognitive-retrieval/src/ranking.rs`

Memory Service只组装 LaneOutput并调用。

最终 `rg` 不应在 `memory-service` 找到 duplicated：

- FAMILY_WEIGHTS
- `60.0 + rank`
- RRF denominator formula

## 3. Authority lane queries

`crates/authority-store`

新增/整理：

- current revision batch lookup
- entity lane query
- temporal lane query
- final validation batch query
- provenance facets batch query
- session resident query

禁止在 Memory query循环内一 candidate一 SQL。

## 4. Runtime Use

`crates/cognitive-runtime/src/use_feedback.rs`

修改：

- same-batch coalescing
- stored duplicate分类
- side effect only new items
- duplicate-only session no update

`resident_refs` schema无需重新设计，除非实现发现缺少必要索引。

建议索引：

```text
resident_refs(session_id,state,last_meaningful_use_at)
cognitive_use_events(subject_id,ref_kind,ref_value,occurred_at)
```

## 5. Memory identity guard

`crates/memory-service/src/lib.rs` / revise path。

在 transaction commit前：

-读取 parent revision aboutness；
-比较 new aboutness；
-parent非空且交集空 -> FailedPrecondition。

不要用 embedding/text similarity决定 revision identity。

## 6. Schema

当前：

- `crates/memory-domain/src/lib.rs`
- `crates/memory-service/src/schema.rs`

新增：

- SchemaFormationKind
- independent-root validator

`create_schema/split/merge` 的 synthesized children都必须经过相同 gate。

explicit_import必须是显式 API/input mode，不能让自动 consolidation伪装 explicit。

## 7. Association

当前：

- `crates/memory-service/src/topology.rs`
- `association_evidence*` tables

修改：

- exact cognition endpoint validation
- producer signature input
- AssociationSupport union
- UseEventRef support

DB migration允许PRE_PRODUCTION clean replacement/add column，不要求 legacy compatibility。

## 8. Topology projection

当前：

- `crates/authority-store/src/topology_input.rs`
- `crates/serving/src/build.rs`
- `crates/cognitive-retrieval/src/graph.rs`
- `crates/cognitive-retrieval/src/wave.rs`

修改：

- projection input展开 provenance roots
- build传真实 provenance_root
- mutable Memory/Schema nodes移除
- contradiction/counterexample/negative过滤
- typed seed weights

Wave propagation merge key当前正确，必须保留。

## 9. Serving

当前：

- `crates/serving/src/lifecycle.rs`
- `crates/serving/src/build.rs`
- `crates/authority-store/src/serving.rs`
- `projection_input.rs`

继续保留：

- immutable generations
- watermark publication fence
- checksum/config/space compatibility

本实施重点不是重写 Serving lifecycle，而是：

- stale candidate正确 Final Authority；
- current-only index source；
- diagnostics。

## 10. Proto / TS

如果新增公开字段：

- Query diagnostics
- SchemaFormationKind
- Association support/producer

必须修改：

- `proto/nous/wave/v1alpha1/*`
- regenerate
- `packages/protocol-ts`
- `packages/client`

禁止手改 generated files。

## 11. Tests

现有：

- `apps/nous-kernel/tests/reference_profile.rs`
- `qualification_edges.rs`

新增建议：

```text
apps/nous-kernel/tests/query_correctness.rs
apps/nous-kernel/tests/runtime_idempotency.rs
apps/nous-kernel/tests/topology_provenance.rs
apps/nous-kernel/tests/scale_fixture.rs
```

不要把所有新断言继续塞进一个超长 integration test。

## 12. Documentation landing

Agent首先将随包文件落库：

```text
docs/current-state/audits/2026-09-27-reference-profile-review.md
docs/plans/active/2026-09-27-cognitive-retrieval-evaluation.md
docs/specs/README.md
docs/specs/active/cognitive-retrieval/*
```

然后更新：

- `docs/INDEX.md`
- `docs/plans/README.md`

落库后再开始代码施工。

## 13. Deletion / grep checks

最终 active product path 不应再存在：

- arbitrary `ORDER BY memory_id LIMIT candidate_limit` 作为多 lane candidate source
- lexical `contains(cue)` admission
- runtime `reference_in_subject()` 作为 residency判定
- positive topology `contradicts`
- Association MemoryId/SchemaId cognition endpoint
- duplicated RRF constants in Memory Service

## 14. Unresolved decisions

None.
