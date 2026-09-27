# 02 — Canonical Fusion and Final Authority Validation

状态：IMPLEMENTATION-AUTHORIZING
目标路径：`docs/specs/active/cognitive-retrieval/02-fusion-and-final-authority.md`

## 1. Scope

冻结：

-唯一 fusion owner；
- revision-level multiview aggregation；
- fixed-plan RRF；
- deterministic tie-break；
- optional second-stage rerank；
- stale Serving semantics；
- batch Final Authority Validator；
- QueryStatus / degradation / diagnostics。

## 2. Ownership

Canonical fusion必须归属：

`crates/cognitive-retrieval`

对外暴露纯函数/纯数据合同。

`memory-service` 负责：

- lane orchestration；
- Authority lookup；
- final validation；
- result materialization。

禁止 `memory-service/query.rs` 和 `cognitive-retrieval/ranking.rs` 各自维护一套权重/denominator/tie-break。

## 3. Fusion input

```text
FusionInput
- enabled_lane_specs
- lane_outputs
- result_limit
```

每个 candidate key是 exact `CognitiveRef`。

同一 revision在同 lane多个 view：

```text
lane_rank = min(view ranks)
```

view数量不产生额外权重。

## 4. RRF v1

固定：

```text
k = 60
```

weights：

| Lane | Weight |
|---|---:|
| exact | 4.0 |
| runtime | 2.0 |
| entity | 2.5 |
| lexical | 1.5 |
| dense | 1.5 |
| temporal | 1.0 |
| schema_direct | 1.5 |
| topology_wave | 1.0 |

对 candidate `c`：

```text
raw(c) = Σ_enabled_lane weight(lane) / (60 + rank_lane(c))
```

candidate在 enabled lane无结果 ->该 lane贡献 0。

归一化：

```text
denom = Σ_enabled_lane weight(lane) / 61
score(c) = raw(c) / denom
```

`denom` 只由 BoundQuery enabled lanes决定。

provider unavailable / stale / zero-result都不能修改 denominator。

## 5. tie-break

排序：

1. score DESC
2. best lane rank ASC
3. exact-match present优先
4. canonical CognitiveRef string ASC

禁止使用：

- candidate family count作为隐式 relevance；
- topology innovation后置 bonus；
- observability bonus；
-直接 seed bonus；
- raw cosine + lexical score线性相加。

## 6. Fusion output

```text
FusedCandidate
- exact reference
- fused_score
- best_lane_rank
- lane_components[]
- variants[]
```

Fusion不读取 DB，不判断 lifecycle。

## 7. Final validation budget

```text
validation_budget = min(
    max(result_limit * 4, 32),
    4096
)
```

从 fusion order依次消费，直到：

-已得到 result_limit个有效 hit；或
- validation_budget耗尽；或
- candidate union耗尽。

不允许重新制定 QueryPlan或开启新 lane。

## 8. Current / historical validation

### 8.1 CurrentOnly

对 MemoryRevision：

```text
candidate.memory_revision_id == memory_objects.current_revision_id
```

否则 drop reason：

`stale_revision`

对 CognitiveSchemaRevision同理。

### 8.2 ExactHistorical

只有 BoundQuery `allowed_revision_refs` 中的 exact revision可绕过 current-head check。

仍必须：

- belongs to Subject；
-未 purge。

历史 exact read不要求 object acceptance=accepted；但普通 cognitive query仍不返回 withdrawn/suppressed历史对象。

管理读取不是本 Query API职责。

### 8.3 Exact mutable object binding

若 BoundQuery由 MemoryId/SchemaId绑定 revision，并记录 epoch：

final validation发现：

- current revision改变；或
- object_epoch改变到会影响 query-visible语义；

drop：

`stale_exact_binding`

不自动重绑。

## 9. Batch Final Authority Validator

新增批量接口，例如：

```text
validate_candidates(
    subject,
    refs[],
    bound_query
) -> ValidationBatch
```

必须：

-单批 SQL/有限批 SQL；
-禁止每 candidate调用 `memory()` 形成 N+1。

### 9.1 Memory batch fields

至少读取：

- revision id
- memory id
- current_revision_id
- object_epoch
- cognitive_role
- formation_mode
- acceptance_state
- integrity_state
- suppression_state
- purge_state
- accessibility_mode
- created_at
- valid_time
- aboutness
- direct/provenance temporal facet
- epistemic_class

### 9.2 Schema batch fields

至少：

- schema revision id
- schema id
- current_revision_id
- object_epoch
- acceptance/integrity/suppression/purge
- applicability valid time
- aboutness

## 10. Lifecycle gate

普通 cognitive query只接受：

```text
acceptance = accepted
integrity = valid
suppression = normal
purge = normal
```

Exact historical ref也不能绕过 `purge`.

suppressed / withdrawn / revalidation_required 的管理读取走 management API，不走普通 Query。

## 11. Accessibility gate

使用当前 R1 reference AccessibilityPolicy。

Exact explicit revision / exact object target：

-可绕过 auto level；
-仍不能绕过 lifecycle/purge。

普通 candidate按 query effort判定。

## 12. Hard constraint revalidation

Spec 01定义的所有 hard constraints在 Final Validator再次执行。

Serving/lane prefilter只是优化，不是 Authority。

## 13. Stale Serving

### 13.1 stale可作为 candidate source

当 current generation watermark < desired watermark：

-如果 rebuild在 request内无法完成，可以使用兼容 stale generation产生 candidate；
- lane status=`stale`;
- response degradation加入 `stale_<family>_projection`.

### 13.2 stale不能突破 Final Validator

stale candidate：

-旧 head -> drop；
-suppressed/purged -> drop；
-role/mode/constraint不再匹配 -> drop。

### 13.3 stale miss

stale generation可能漏掉新 cognition。

同一 query中不重新 plan。

Exact target必须直接 Authority lookup，因此不受 stale exact index miss影响。

## 14. Serving corruption

artifact：

- checksum mismatch；
- implementation mismatch；
- config digest mismatch；
- dense space mismatch；

不得作为 candidate source。

如果 lane optional：

- lane unavailable；
- query Degraded。

如果 caller声明该 capability Required：

- QueryStatus Partial；
- degradation明确说明 required lane unavailable。

## 15. Optional second-stage rerank

Reranker不是 RRF lane。

启用条件：

- explicit query/config `rerank=true`；或
- capability requirement Required。

输入：

```text
top_n = min(50, max(12, result_limit * 4))
```

输入candidate必须已经通过 Final Authority Validator。

每candidate最多 4096 UTF-8 bytes canonical text。

reranker只能：

-重新排列输入 candidates。

不能：

-增加 candidate；
-恢复被 validator drop的 candidate；
-绕过 accessibility；
-写回 Authority confidence。

### provider failure

optional ->保留 baseline order，Degraded diagnostics。
required -> QueryStatus Partial；返回已验证 baseline candidates并明确 degradation。

## 16. QueryStatus

### Complete

-所有 required lanes可用；
-没有 required capability失败；
-final validation正常结束；
-结果数少于 limit仅因为 Authority确实不足，不算错误。

### Degraded

- optional provider/lane unavailable；
-使用 stale compatible projection；
- topology truncated；
- optional reranker失败。

### Partial

- required lane/capability不可用；
- exact mutable binding stale；
- validation budget耗尽且仍有未验证 candidates。

Invalid query直接返回 error，不产生 QueryStatus。

## 17. Diagnostics

Summary：

- enabled lanes
- lane status
- generation/watermark
- candidate counts
- fusion candidate count
- validation budget
- valid result count
- drop reason counts
- stale lanes

Full：

- per-candidate lane ranks
- RRF components
- validator reason
- rerank position
- Wave provenance摘要

drop reason至少：

```text
not_in_subject
not_current
stale_exact_binding
withdrawn
revalidation_required
suppressed
purging
accessibility
role
formation_mode
entity_requirement
temporal
source_class
authority
modality
epistemic_class
```

## 18. Required tests

1. stale lexical generation旧 revision不能普通返回。
2. exact historical revision可以返回。
3. exact MemoryId绑定后 head变化 -> stale_exact_binding，不返回新head。
4. enabled dense lane provider unavailable，RRF denominator不变。
5.同 revision多 dense views只贡献一次。
6. candidate输入顺序随机化，输出完全 deterministic。
7. topology只通过 RRF contribution。
8. final validator batch query无 per-candidate SQL。
9. suppressed candidate stale index hit被drop。
10. revalidation_required被drop。
11. role/mode/entity/temporal在 final validator再次核验。
12. optional reranker失败保留 baseline order。
13. required reranker失败 -> Partial。
14. validation budget耗尽有明确 diagnostics，不 replan。

## 19. Rejected alternatives

- validator drop后重新开启 lane：拒绝。
- stale index当 Authority：拒绝。
- reranker直接取代 eligibility：拒绝。
-候选集合动态 denominator：拒绝。
-两个 fusion owner：拒绝。

## 20. Unresolved decisions

None.
