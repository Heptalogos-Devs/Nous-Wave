# 03 — CognitiveSchema, AssociationEvidence and AssociativeTopology Closure

状态：IMPLEMENTATION-AUTHORIZING
目标路径：`docs/specs/active/r2-cognitive-retrieval/03-schema-association-topology.md`

## 1. Scope

冻结：

- CognitiveSchema formation gate；
- Schema evidence role；
- Schema evidence增补；
- Association endpoint / support / producer contract；
- Association provenance root展开；
-普通 Wave adjacency语义；
- typed seed weights；
- Wave R1继续作为 reference topology implementation。

## 2. CognitiveSchema formation

Schema object/revision结构继续沿用R1。

新增创建来源：

```text
SchemaFormationKind
- explicit_import
- synthesized
```

建议存储在 SchemaRevision，因为它描述该 revision 的形成来源。

### 2.1 explicit_import

用于：

-人类明确输入；
- Host明确结构化导入；
-受信管理工具显式创建。

要求：

-至少 1 条有效 evidence link；
-保存 source/producer identity；
-不要求 2 个 independent roots。

### 2.2 synthesized

用于系统 consolidation / schema induction。

要求全部满足：

1. `evidence_links.len >= 2`
2. normalized support inputs >= 2
3. known independent provenance roots >= 2
4. UnknownDependency 不增加 independent root count
5. dependency graph无 cycle

同一 Observation的 Support + BoundaryCase仍只有一个 root，不满足 synthesized gate。

## 3. SchemaEvidenceRole

保持：

```text
support
counterexample
boundary_case
```

### support

支持 structural claim。

### counterexample

反驳/限制 structural claim。

### boundary_case

描述适用边界，不计 positive support。

## 4. add_schema_evidence

向当前 SchemaRevision增加 evidence link：

-不创建 content revision；
- `object_epoch += 1`;
- current revision id不变；
- topology/exact projection invalidated；
-若新增 link改变 independent-root gate，不自动改 structural text。

删除/撤销 evidence link同理。

## 5. Schema revision

只有以下变化需要新 content revision：

- structural_claim
- applicability scope
- boundary definition
- title/narrative representation

evidence数量变化本身不需要 revision。

## 6. Schema synthesized gate复用 Provenance substrate

使用 Memory/Schema统一 provenance closure：

```text
normalize_inputs(evidence_links)
derive_root_sets(...)
count_known_independent_roots(...)
```

禁止按：

- evidence link数量；
-模型数量；
-derived summary数量；

代替 source independence。

## 7. Association endpoint

`AssociationEvidence.from/to`：

### cognition endpoint

只允许 exact：

- MemoryRevision
- CognitiveSchemaRevision

以下直接 `INVALID_ARGUMENT`：

- Memory
- CognitiveSchema

服务端不得静默解析 current head。

### stable structural endpoint

允许：

- Entity
- Tag
- Resource

## 8. Association support union

R2将 Association support从单一 `RevisionSupport`扩展为：

```text
AssociationSupport
- SourceOrCognition(RevisionSupport)
- UseEvent(UseEventRef)
```

`UseEventRef`：

```text
subject_id
consumer_ref
event_id
```

必须能在 `cognitive_use_events` 或 `purged_use_receipts` 中验证。

purged UseEvent只证明事件身份存在，不允许重新恢复已purge cognition identity。

## 9. AssociationSupportClass contract

### host_explicit

-允许 producer signature可选；
-至少一个 support；
-表示 Host/管理者明确声明的 association。

### source_evidence

-至少一个 SourceEvidence；
-不能只靠 CognitionDependency。

### cognitive_derivation

- `producer_signature_id` REQUIRED；
-至少一个 CognitionDependency；
-保存模型/算法 producer identity。

### meaningful_use

-至少一个 UseEventRef；
-允许的 UseKind：
  - referenced
  - acted_on
  - result_supported
  - corrected
  - pinned

`result_refuted` 不产生 positive meaningful-use association；如需要表达反向证据，应创建 `polarity=negative` association。

### derived_structure

- `producer_signature_id` REQUIRED；
-用于 deterministic builder / structure extraction；
- support必须能追溯输入。

## 10. Association producer identity

CreateAssociationRequest增加：

```text
producer_signature_id?
supports: Vec<AssociationSupport>
```

事务提交前验证：

- producer signature属于允许 producer registry；
-其 algorithm/model/config identity可追踪。

## 11. TopologyProjectionInput

不再把一个 AssociationEvidence压成单条 `provenance_root=None` edge。

对每条 active positive AssociationEvidence：

1.展开全部 supports；
2.解析 source root sets；
3.对每个 known root产生一个 `TopologyEdgeContribution`；
4.同一 AssociationEvidence 的 unknown dependency最多产生一个 conservative anonymous root：
   `unknown-association:<association_evidence_id>`。

结构：

```text
TopologyEdgeContribution
- from exact/stable ref
- to exact/stable ref
- relation_kind
- support_class
- polarity
- provenance_root
- support_mass = 1.0
```

Graph builder再按 `(from,to,root)`取最高 quality。

## 12. Structural edges

### aboutness

current MemoryRevision ↔ Entity。

root：

```text
structure:aboutness:<memory_revision_id>:<entity_ref>
```

support class=`derived_structure`。

### tag attachment

current MemoryRevision ↔ Tag。

root：

```text
structure:tag:<memory_revision_id>:<tag_id>
```

### schema support

只对 `SchemaEvidenceRole::support` 建立：

SchemaRevision ↔ exact supported cognition revision。

root来自该 evidence link 的 provenance root closure。

BoundaryCase/Counterexample不进入 positive adjacency。

## 13. Revision relations

可进入普通 positive adjacency：

- derived_from
- temporal_successor
- elaborates

不进入普通 positive adjacency：

- contradicts
- replaces_basis（R2默认不作为 associative relevance）
-所有 negative association
- schema counterexample
- schema boundary_case

这些仍保留 Authority与diagnostics。

## 14. Topology node identity

普通 topology nodes只允许：

- current active MemoryRevision
- current active CognitiveSchemaRevision
- Entity
- Tag
- Resource

禁止：

- Memory mutable object node
- CognitiveSchema mutable object node
- withdrawn/suppressed/revalidation/purging cognition revision作为current topology node

历史 exact revision不进入普通 serving topology。

## 15. Edge quality

Support class quality：

| class | q |
|---|---:|
| host_explicit | 1.00 |
| source_evidence | 0.90 |
| cognitive_derivation | 0.75 |
| derived_structure | 0.65 |
| meaningful_use | 0.50 |

同 `(from,to,root)`：

```text
root_mass = max(q * support_mass)
```

edge raw：

```text
raw = Σ root_mass
```

R2 ordinary graph只有positive contributions；negative不做 `positive-negative` 抵消，因为普通 Wave是non-negative graph。

负向证据保留给未来 signed/competitive experiment。

## 16. Hub penalty / outbound normalization

保留 Wave R1 reference：

```text
median_inflow = median(positive node inflow), fallback 1
relative = inflow / median_inflow
penalty = clamp(relative ^ -0.35, 0.35, 1.25)
adjusted = raw * penalty
```

每 node outbound conductance总质量：

```text
<= 0.90
```

max neighbors：

```text
32
```

## 17. Propagation config

继续使用：

```text
minimum_state_energy = 1e-4
immediate_return_multiplier = 0.20
initial_budget_steps = 4
normal_edge_cost = 1
fir_gamma = 0.55
```

QueryPlan提供：

- max hops
- max states

merge key必须包含：

```text
(previous_node, current_node, hop, remaining_budget_steps)
```

不能重新退化为只按 current node合并。

## 18. Seed weights

实现：

| family | weight |
|---|---:|
| exact target | 1.00 |
| explicit schema | 1.00 |
| relation cue | 1.00 |
| entity cue | 0.90 |
| runtime situation/resident | 0.85 |
| tag cue | 0.75 |
| lexical promoted | 0.60 |
| dense promoted | 0.60 |
| residual discovery | 0.40 |

R2 residual默认 OFF。

同 node权重先求和，再 normalize整个 seed vector到 1。

## 19. Wave output

Wave只产生：

`topology_wave` lane rank。

rank：

1. node_potential DESC
2. first_hop ASC
3. canonical ref ASC

禁止任何 plan外 additive topology bonus。

## 20. Topology diagnostics

至少：

- generation id
- authority watermark
- seed families / weights
- root contribution count
- nodes explored
- max states
- discarded state mass
- complete
- first hop
- strongest parent

Full diagnostics可以显示 relation/root摘要，但不得泄露 purge内容。

## 21. Mutation invalidation

以下变更 invalidates topology：

- Memory current revision
- Memory aboutness/tag
- Schema revision
- Schema evidence link add/revoke
- Association add/revoke
- Tag revision
- lifecycle使 cognition进入/离开 active set

Lexical/dense只有文本/表示变化时才 invalidated。

## 22. Required tests

1. synthesized Schema同一 root两条 links -> reject。
2. synthesized Schema两个 known independent roots -> accept。
3. explicit_import一个 support -> accept。
4. add_schema_evidence不创建 content revision。
5. MemoryId作为 Association endpoint -> invalid。
6. SchemaId作为 Association endpoint -> invalid。
7. cognitive_derivation无 producer -> invalid。
8. meaningful_use无 UseEvent -> invalid。
9. result_refuted不能创建 positive meaningful-use association。
10.同 root三条 association supports只产生一个 root contribution。
11.两个 independent roots产生两个 contribution。
12. unknown root单 association最多一个 conservative root。
13. contradicts不进入 adjacency。
14. counterexample/boundary不进入 positive adjacency。
15. topology nodes无 mutable Memory/Schema object。
16. seed weights按表执行。
17. merge key保留 remaining budget。
18. Wave只通过 topology RRF lane影响 final score。

## 23. Rejected alternatives

-按 evidence link数量判断 Schema synthesis：拒绝。
- mutable cognition endpoint：拒绝。
- negative edge塞入普通 non-negative Wave：拒绝。
- benchmark前实现 signed graph：拒绝。
- producer-less derived association：拒绝。

## 24. Unresolved decisions

None.
