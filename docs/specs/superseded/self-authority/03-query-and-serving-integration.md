# 03 — Cognitive Query 与 Serving 接入 Self

## 1. 当前结构问题

当前 `CognitiveRuntimeService::query_with_plan` 接收单一 `memory: Option<&dyn CognitiveContributor>`。

这会迫使 Self：

-塞进 Memory；
-或复制 Query orchestration。

两者都不接受。

## 2. contributor 改为固定多 owner

改成：

```text
CognitiveContributors
- memory?
- self_cognition?
```

或等价的固定 slice。

本次不要做：

-动态 plugin registry；
-字符串 domain registry；
-运行时 service discovery。

只有当前真实 owner。

## 3. retrieval crate 命名

当前 `memory-retrieval` 已经持有通用：

- EvidenceFamily；
- RRF；
- Wave；
- candidate rank types。

Self加入后，Memory命名会造成错误 owner暗示。

直接 rename：

```text
crates/memory-retrieval
→ crates/cognitive-retrieval
```

更新 Cargo workspace / imports。

不保留 alias crate。

## 4. generic lane types

`LaneOutput` / `LaneCandidate` 从 Memory Service移到 `cognitive-retrieval`。

结构继续：

```text
LaneOutput
- family
- status
- generation_ref?
- authority_watermark?
- candidates[]
- diagnostics
```

candidate必须 exact ref。

## 5. 新的 SelfDirect family

增加：

```text
EvidenceFamily::SelfDirect
```

RRF reference weight：

```text
SelfDirect = 2.0
```

用途：

- QueryTarget::Self；
- SelfFacetCue；
- exact Self object binding后的 current revision。

这只是 reference weight，benchmark以后如有证据可调整。

## 6. Query types

新增：

```text
QueryTarget::Self
```

新增 cue：

```text
SelfFacetCue
- kind?
- key?
```

至少一个字段存在。

kind用协议 string映射 SelfFacetKind，未知 kind -> InvalidArgument。

## 7. QueryPlan

`SelfDirect`启用：

- target包含 Self；
-有 SelfFacetCue；
- exact target为 SelfFacet / SelfFacetRevision / NarrativeIdentity / NarrativeIdentityRevision。

Text cue + QueryTarget::Self 同时启用 Lexical。

Self target本身不自动启用 Dense；只有已有 text embedding requirement/normal query rules决定 Dense。

## 8. SelfDirect candidate

### facet cue

Authority SQL直接查询 current active SelfFacet：

- subject；
- kind/key exact match；
- current head。

排序：

1. both kind+key exact；
2. key exact；
3. kind exact；
4. kind固定顺序；
5. key ASC；
6. revision id ASC。

### QueryTarget::Self 无 cue

返回 current active Self facet + Narrative current revision，bounded by lane budget。

固定 kind顺序：

```text
identity
role
limitation
capability
value
preference
tendency
narrative
```

这个顺序只用于 deterministic direct listing，不表示永久价值优先级。

## 9. lexical / dense Serving

Serving document source扩展为通用 cognition document：

```text
CognitiveServingDocument
- exact revision ref
- subject
- domain
- canonical text
- valid time
- authority watermark
```

来源：

- Memory current revision；
- CognitiveSchema current revision；
- SelfFacet current revision；
- NarrativeIdentity current revision。

Lexical / Dense generation不能再假定所有 document都是 Memory。

Memory-specific metadata仍保留在 owner materialization，不塞进 generic Serving record。

## 10. query collection

理想执行：

1. bind query；
2. fixed QueryPlan；
3. generic Serving lanes产生 exact refs；
4. domain-direct lanes产生 exact refs；
5.统一 RRF；
6.按 ref owner批量 final validation/materialization；
7.按 global fused order填充 result。

Memory 与 Self都不得自己再做一遍最终 global排序。

## 11. contributor contract

```text
trait CognitiveContributor {
    fn owns(&self, reference: &CognitiveRef) -> bool;

    async fn direct_lanes(
        &self,
        bound: &BoundQuery,
        plan: &QueryPlan
    ) -> Result<Vec<LaneOutput>>;

    async fn validate_and_materialize(
        &self,
        subject: SubjectId,
        refs: &[CognitiveRef],
        bound: &BoundQuery
    ) -> Result<DomainValidation>;
}
```

Lexical / Dense / Runtime / Exact 等 shared lane由 Cognitive Runtime + Serving处理。

Memory owner继续提供：

- Entity；
- Temporal；
- SchemaDirect；
- TopologyWave。

Self owner提供：

- SelfDirect。

以后 Social / Motivation增加自己的 domain-direct lane，而不是复制 orchestration。

## 12. Memory N+1 顺手清理

`validate_and_materialize()` 对同 domain refs做 batch。

删除：

- per candidate `memory(...)`；
- per candidate resolve SQL；
- lexical/dense 每 hit的单独 subject ownership SQL。

Serving hit先是 exact ref；最终 owner batch validation负责 Subject / lifecycle。

## 13. final validation

Memory：

继续 current/historical/lifecycle/accessibility/hard constraint。

Schema：

修复为与 Memory相同的最后时刻 batch复核：

普通 query：

```text
current
accepted
valid
normal
not purging
```

explicit historical exact：

-可非 current；
-不能 purged；
-不能把 mutable binding偷偷重绑。

Self facet / Narrative：

普通 query：

```text
current
accepted
valid
normal
not purging
```

无 accessibility。

## 14. exact binding

mutable：

- Memory
- CognitiveSchema
- SelfFacet
- NarrativeIdentity

全部在 BoundQuery binding时固定 exact current revision + object_epoch。

执行过程中 head/epoch变化：

- stale exact binding；
-不自动换新 head。

## 15. Runtime resident

ResidentSet允许 exact：

- MemoryRevision；
- CognitiveSchemaRevision；
- SelfFacetRevision；
- NarrativeIdentityRevision。

UseEvent仍然只对“长期 meaningful use”有语义。

Self被放入模型上下文不自动产生长期 use；实际引用/据此行动时才 ReportUse。

## 16. context contribution

Self hit materialize：

```text
semantic_role = "self:<kind>"
authority = SubjectCognition
representation = statement/text
freshness = revision times
evidence = only when requested
```

Narrative：

```text
semantic_role = "self:narrative"
```

不把 Seed原文整份注入模型。

## 17. 与 Memory 混合查询

统一 RRF后允许：

```text
targets = [Memory, Self]
```

不再额外发明 domain priority。

谁排名靠前由 fixed lanes与 query cue决定。

SelfDirect只在 Self明确成为 target/cue时启用，因此不会让所有普通 Memory query突然塞满人格内容。

## 18. 现有未验证语义

修改 lexical / serving 时顺便加入：

- stale lexical旧 Memory revision不能普通返回；
- tokenizer真实 hit不能被 substring逻辑丢掉。

Association producer相关测试在修改 shared support / query时顺手补一个 positive derived producer场景。

不新增大规模 benchmark。
