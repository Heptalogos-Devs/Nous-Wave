# 01 — BoundQuery and Typed Retrieval Lanes

状态：IMPLEMENTATION-AUTHORIZING
目标路径：`docs/specs/active/r2-cognitive-retrieval/01-bound-query-and-lanes.md`

## 1. Scope

本 Spec 冻结：

- Query binding；
- current / historical revision语义；
- QueryPlan；
- hard constraints；
- typed LaneProvider；
- lane budgets；
- Exact / Runtime / Entity / Lexical / Dense / Temporal / SchemaDirect / TopologyWave candidate generation。

Fusion 与 Final Authority validation由 Spec 02负责。

## 2. 核心不变量

1. QueryPlan 必须在 candidate generation 前冻结。
2. lane 是否 enabled、预算、权重不能由候选结果反向改变。
3. 持久 cognition candidate必须使用 exact revision identity。
4. 普通 recall 是 `CurrentOnly`。
5. explicit exact historical revision target才允许历史 revision。
6. lane provider只能声明自身 lane的 relevance，不产生 global score。
7. 禁止 application-side arbitrary first-N object sampling。
8. unsupported hard constraint不得 silent ignore。

## 3. BoundQuery

新增内部类型：

```text
BoundQuery
- query_id: UUIDv7
- subject_id
- source_query
- bound_at_authority_seq
- revision_policy
- exact_bindings[]
- enabled_lanes[]
- lane_budgets
- hard_constraints
- accessibility_policy
- selected_embedding_space?
- topology_required
- fusion_version = "rrf-v1"
- rerank_policy
```

### 3.1 RevisionPolicy

```text
CurrentOnly
ExactHistorical {
    allowed_revision_refs: Set<CognitiveRef>
}
```

普通：

- `QueryTarget::Memory`
- `AnyRelevantCognition`
- Text / Entity / Temporal / Schema discovery

全部 `CurrentOnly`。

只有请求显式携带：

```text
QueryTarget::Exact {
    reference = MemoryRevision(...)
}
```

或：

```text
QueryTarget::Exact {
    reference = CognitiveSchemaRevision(...)
}
```

时，对该 exact ref加入 `allowed_revision_refs`。

### 3.2 Exact mutable object binding

若 exact target是：

- `MemoryId`
- `CognitiveSchemaId`

binding阶段在 Authority 中解析：

```text
object_id
current_revision_id
object_epoch
```

保存为：

```text
ExactBinding
- requested_ref
- bound_revision_ref
- bound_object_epoch
```

执行期间不得重新绑定。

Final validation发现 current head/epoch变化时：

- 不偷偷切换到新 head；
-丢弃 bound candidate；
- diagnostics=`stale_exact_binding`;
-如果 exact target是唯一目标，QueryStatus=Partial；
-调用方可重试 Query。

这保证同一 BoundQuery 的语义稳定。

## 4. QueryPlan budgets

按 `CognitiveEffort`：

| Effort | multiplier | per-lane max | Wave hops | Wave states |
|---|---:|---:|---:|---:|
| Light | 2 | 128 | 1 | 128 |
| Normal | 4 | 512 | 2 | 512 |
| Deep | 8 | 2048 | 3 | 2048 |
| Maximum | 16 | 8192 | 4 | 4096 |

```text
lane_budget = clamp(result_limit * multiplier, 16, per_lane_max)
```

ExactLane 不受 lane_budget 截断，但最终仍受 result_limit。

Final validation预算由 Spec 02定义。

## 5. Enabled lanes

R2 lanes：

```text
exact
runtime
entity
lexical
dense
temporal
schema_direct
topology_wave
```

### exact

有任意 Exact target时启用。

### runtime

以下任一成立：

- `query.session.is_some()`
- `situation.current_refs` 非空

### entity

有 Entity cue 或 entity hard requirement时启用。

### lexical

有 Text / Example cue时启用。

### dense

有 Text / Example cue，且 embedding capability不是 Forbidden。

即使 provider不可用，lane仍属于已计划 enabled lane，运行状态记 unavailable；不得从 denominator中移除。

### temporal

以下任一成立：

- valid / occurred / observed hard constraint；
- Temporal cue。

### schema_direct

有 Schema cue、Schema exact target、SchemaNeighborhood target。

### topology_wave

只有显式 associative intent启用：

- `BoundedAssociative`
- `AroundTag`
- `AroundSchema`
- `ExplainAssociation`
- Relation cue
- EntityNeighborhood
- SchemaNeighborhood

**Deep/Maximum 本身不启用 Wave。**

## 6. LaneProvider contract

统一内部接口：

```text
trait LaneProvider {
    family() -> EvidenceFamily

    generate(
        bound_query: &BoundQuery,
        prior_outputs: &LaneOutputs
    ) -> LaneOutput
}
```

`prior_outputs` 仅供明确依赖前一阶段 seed 的 lane使用；不能修改 QueryPlan。

### LaneOutput

```text
LaneOutput
- family
- status
- generation_ref?
- authority_watermark?
- candidates[]
- diagnostics
```

### LaneCandidate

```text
LaneCandidate
- reference: exact CognitiveRef
- rank: u32          // 1-based
- variants[]
- provider_metadata
```

同 lane 的 rank必须 deterministic。

## 7. 执行顺序

Primary lanes：

```text
Exact
Runtime
Entity
Lexical
Dense
Temporal
SchemaDirect
```

完成后执行 TopologyWave；Wave可以使用显式 seeds以及已产生的 top lexical/dense current-revision candidates作为 promoted seeds。

Fusion只在全部计划 lane完成/降级后运行。

## 8. ExactLane

### 8.1 MemoryId / SchemaId

使用 BoundQuery 的 bound exact revision。

### 8.2 exact revision

使用请求给出的 exact revision，不替换 current head。

### 8.3 other exact CognitiveRef

Entity / Tag / Resource / Observation / source refs可以作为 exact non-cognition result或其他 lane seed，但不能伪装成 Memory candidate。

### 8.4 error

请求的 exact ref在 binding时不存在：

- `NOT_FOUND`
-不要进入“空结果”。

## 9. RuntimeLane

R2 不从 `active_focus_key: String` 推导 cognition ref。

Runtime candidates只来自：

1. `SituationDescriptor.current_refs`
2.指定 Session 的 `resident_refs`

### 9.1 Situation refs

只接受 Subject内有效 exact cognition revision：

- MemoryRevision
- CognitiveSchemaRevision

Mutable Memory/Schema ref必须在 binding时解析成 current revision。

排序优先于 resident refs。

### 9.2 Resident refs

查询：

```text
session_id
ref_kind
ref_value
state='resident'
```

只接受 exact revisions。

排序：

1. Situation current refs 按请求顺序；
2. resident `last_meaningful_use_at DESC NULLS LAST`;
3. `entered_at DESC`;
4. canonical ref ASC。

同 revision只出现一次。

### 9.3 session validation

session：

-必须属于同 Subject；
-必须未 closed；
-否则 binding阶段 `FAILED_PRECONDITION`。

## 10. EntityLane

### 10.1 Memory

直接从 current head aboutness 查询：

```text
memory_objects
JOIN memory_revisions ON current_revision_id
JOIN memory_revision_aboutness
```

不允许先枚举 Memory再过滤。

多个 Entity cue：

- cue discovery 默认 OR union；
- hard `entity_requirements` 是 AND/all-of。

### 10.2 Schema

若 schema applicability aboutness直接命中，可作为 schema candidate。

### 10.3 ranking

Entity cue按输入顺序编号。

candidate key：

```text
matched_requirement_count DESC
best_cue_index ASC
canonical_ref ASC
```

rank由上述排序生成。

## 11. LexicalLane

唯一 relevance authority是当前 Lexical Serving generation。

禁止：

- `representation_text.contains(cue)` 二次 admission；
-没有 index hit时 fallback rank；
-用 DB text自行制造 lexical rank。

对 generation返回的同 revision多个 view：

-保留全部 variant metadata；
- lane rank取最优 rank。

如果 generation stale，可以作为候选来源，但 final validator由 Spec 02保证 current correctness。

## 12. DenseLane

### 12.1 space binding

BoundQuery 只能绑定一个 primary `EmbeddingSpaceSignature`。

选择：

1.显式要求的 space（若协议存在）；
2. configured default embedding provider的 space；
3. provider不存在 -> lane unavailable。

不同 space的 vector score不混合。

### 12.2 candidates

只消费与 bound space compatible 的 generation。

同 revision多 view：

- lane rank取最优 ANN rank；
- variants保留 view信息；
-不按 view数叠加 relevance。

## 13. TemporalLane

TemporalLane必须从 Authority SQL按时间条件生成 bounded candidates，不允许 application-side scan。

### 13.1 fields

- `MemoryRevision.valid_time`
- direct/reachable source occurrence `occurred_time`
- `observed_at`

### 13.2 hard constraints

`valid / occurred / observed` 属于 hard filter。

TemporalLane在存在 temporal cue时还负责 relevance rank。

### 13.3 rank

每个 candidate计算 `temporal_distance_seconds`：

-与 query interval有 overlap -> 0；
-否则取 candidate extent 到 query interval的最小边界距离；
- Unknown -> `+∞`，只在 hard constraint没有要求该字段时保留。

排序：

```text
distance ASC
recorded_at DESC
canonical_ref ASC
```

## 14. SchemaDirectLane

Schema cue / target：

1. 当前 CognitiveSchemaRevision 本身；
2. active `SchemaEvidenceLink(role=support)` 直接引用的 current MemoryRevision / SchemaRevision。

BoundaryCase / Counterexample 不作为 positive SchemaDirect relevance扩展，但可以出现在 diagnostics/evidence。

排序：

1. exact/bound schema revision；
2. direct Support links；
3. canonical ref。

AroundSchema 的更广关联由 TopologyWave负责。

## 15. TopologyWaveLane seeds

Wave在 primary lanes完成后执行。

seed base weights：

| Seed | Weight |
|---|---:|
| exact target | 1.00 |
| explicit schema / relation | 1.00 |
| entity cue | 0.90 |
| runtime situation/resident | 0.85 |
| tag cue | 0.75 |
| lexical promoted | 0.60 |
| dense promoted | 0.60 |
| residual discovery | 0.40 |

R2 residual discovery默认 OFF。

### promoted seed bound

每个 lexical/dense lane最多取：

```text
min(8, lane_budget)
```

作为 Wave seed。

同 node多 seed权重求和，再将全部 seed质量 normalize到 1。

Wave graph/edge语义由 Spec 03定义。

## 16. Hard constraints

### 16.1 Include sets

以下字段统一语义：

```text
include.empty OR candidate_value ∈ include
```

适用于：

- cognitive role
- formation mode
- source class
- modality
- epistemic/evidence class

### 16.2 Exclude sets

```text
candidate_value ∉ exclude
```

### 16.3 Entity requirements

`entity_requirements` 是 AND/all-of：

```text
∀ required_entity: required_entity ∈ cognition.aboutness
```

### 16.4 authority

MemoryRevision / CognitiveSchemaRevision：

`SubjectCognition`

Observation/source：

`Evidence`

DerivedRepresentation：

`Interpretation`

Resource：

`ResourceDescriptor`

### 16.5 modality

Memory / Schema canonical representation：

`Text`

Material evidence使用真实 modality。

### 16.6 source class

对 Memory / Schema：

-由 provenance root closure得出；
- include：至少一个 known root source class命中；
- exclude：任何 known root source class命中则排除；
- root未知不满足 include，但也不触发 exclude。

### 16.7 evidence class

MemoryRevision 对应 `epistemic_class`。

Schema没有 `epistemic_class`；若 query指定 evidence class且 target包含 Schema，则 Schema不匹配。

### 16.8 unsupported

如果协议包含一个 R2没有定义语义的 hard constraint：

binding阶段 `INVALID_ARGUMENT("constraint is unsupported by R2 reference query")`

禁止忽略。

## 17. Accessibility

lane provider可以提前利用 accessibility做候选裁剪，但 final validator必须再次检查。

Exact explicit historical/current ref可绕过 auto accessibility。

不能绕过：

- purge
- Subject ownership

suppressed/revalidation_required只有 management path可见；普通 cognitive query仍排除。

## 18. Diagnostics

每 lane至少记录：

```text
planned
status = disabled | ready | stale | unavailable | truncated
budget
candidate_count
generation_id?
authority_watermark?
detail_code?
```

禁止将 `unavailable` 表述成“Subject不知道”。

## 19. Current code changes

至少涉及：

- `crates/cognitive-runtime/src/query/plan.rs`
- `crates/cognitive-runtime/src/query/orchestrate.rs`
- `crates/memory-service/src/query.rs`
- `crates/authority-store` lane-specific queries
- `crates/serving`
- Proto/TS query model（若 BoundQuery/diagnostics公开）

删除：

- arbitrary first-N Memory candidate source；
- lexical substring admission；
- subject-membership runtime rank。

## 20. Required tests

1. 10k Memory中唯一 entity match位于最后，仍被 EntityLane召回。
2. 10k Memory中唯一 temporal match不受 MemoryId顺序影响。
3. Session A resident的 revision不因 Session B存在而获得 Runtime rank。
4. Situation current ref优先于 resident。
5. multi-role include按 OR set语义。
6. multi-mode include按 OR set语义。
7. entity_requirements保持 AND。
8. Tantivy hit即使不满足 literal substring仍保留。
9. 非 Tantivy hit即使 substring一致也没有 lexical rank。
10. Dense不同 space不进入同 lane。
11. Deep text query无显式 associative intent时Wave关闭。
12. explicit historical revision只在 exact target下允许。
13. exact MemoryId在执行中 head变化不偷偷重绑。
14. unsupported hard constraint明确失败。

## 21. Rejected alternatives

- first-N object scan：拒绝。
- candidate结果动态启用 lane：拒绝。
- current head在执行中自动重绑：拒绝。
- lexical DB substring fallback：拒绝。
- Deep effort自动启用 topology：拒绝。

## 22. Unresolved decisions

None.
