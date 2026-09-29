# Cognitive Query 唯一融合与多领域编排

## 1. 问题

当前 `CognitiveContributor::contribute()` 允许领域 owner 自己读取 Serving、自己计算 RRF、自己排序、返回完整 `CognitiveQueryResult`。

Social 加入前必须改掉。

## 2. 唯一排序 owner

```text
Cognitive Runtime
    组织 query
        ↓
shared/domain lane candidates
        ↓
nous-retrieval
    唯一 RRF / aggregation
        ↓
Cognitive Runtime
    按 owner 分组
        ↓
domain batch validation/materialization
        ↓
Cognitive Runtime
    按 fused order 构造最终 CognitiveHit
```

任何 domain service不得再计算 RRF denominator、lane weight、global final score、global result truncation。

## 3. CognitiveContributor

```text
trait CognitiveContributor {
    fn owns(&self, reference: &CognitiveRef) -> bool;

    async fn direct_lanes(
        &self,
        bound: &BoundQuery,
        plan: &QueryPlan,
    ) -> Result<Vec<LaneOutput>>;

    async fn validate_and_materialize(
        &self,
        subject: SubjectId,
        references: &[CognitiveRef],
        bound: &BoundQuery,
    ) -> Result<Vec<MaterializedCognition>>;
}
```

`MaterializedCognition`不包含 `MatchEvidence`。

字段：reference、revision、semantic_role、cognitive_role?、formation_mode?、representation?、authority、freshness、entity_refs[]、evidence[]、materialization[]。

最终 `MatchEvidence`只由 Runtime根据 fusion result附加。

## 4. SharedLaneProvider

Runtime不直接依赖 concrete ServingService。

```text
trait SharedLaneProvider {
    async fn lanes(
        &self,
        bound: &BoundQuery,
        plan: &QueryPlan,
    ) -> Result<Vec<LaneOutput>>;
}
```

Kernel composition提供 adapter，读取 lexical serving、dense serving、以后 shared rerank。

Runtime-owned lanes：Exact、Runtime resident。

Domain lanes：

- Memory：Entity、Temporal、SchemaDirect、TopologyWave；
- Self：SelfDirect；
- Social：SocialRelationDirect、LanguageConventionDirect。

## 5. LaneOutput

唯一 generic contract放在 `cognitive-retrieval`：family/status/generation_ref?/authority_watermark?/candidates[]/diagnostics。

candidate只包含 exact_ref、rank、variant。provider raw score只作 diagnostics，fixed RRF reference path不跨 provider比较 raw score。

## 6. 一次 fusion

1. bind query；
2. fixed QueryPlan；
3. Runtime Exact/Resident lanes；
4. Shared Serving lanes；
5. 各 domain direct_lanes；
6. 同 exact revision多 view聚合；
7. cognitive-retrieval一次 RRF；
8. fused refs按 owner分组；
9. owner batch final validation/materialization；
10. drop invalid/stale；
11. 恢复 fused global order；
12. attach MatchEvidence；
13. truncate result limit。

## 7. owner dispatch

`CognitiveContributors`继续 compile-time fixed composition：memory?/self_cognition?/social?。

不要 dynamic registry/string discovery。

dispatch通过 `owns()`；两个 owner声称同 ref → Infrastructure error；持久 cognition没有 owner → Unavailable/Infrastructure，不能静默 generic hit。

## 8. Exact

mutable object target在 bind时固定 exact revision + object epoch，适用于 Memory、CognitiveSchema、SelfFacet、NarrativeIdentity、RelationshipAssertion、LanguageConvention。

执行中 head/epoch变化 → stale，不自动切 head。

explicit exact revision允许历史读，但 purge仍不可绕过。

## 9. batch materialization

Memory/Self/Social不得逐 candidate执行单 SQL。owner收到同 domain refs后一到有限常数次 SQL获取 state/payload/support。

## 10. Self 删除 owner-side RRF

删除 `self-service/query.rs` 中 default_rrf_plan denominator、本地 weight/final_score、owner global sort。

Self只产生 SelfDirect candidates和 batch materialization。

Lexical/Dense由 SharedLaneProvider统一生成。

## 11. Query diagnostics

Runtime汇总 lane status、candidate counts、generation ids、owner validation drop counts。domain不能覆盖其他 owner diagnostics。

## 12. 验收记录

当前“single fusion owner PASS”在 Self提交后已过时。

实现本文件后，只有当仓库唯一 RRF公式位于 `cognitive-retrieval`，且 mixed-owner fixture通过，才能重新记为 PASS。


## 子系统 disabled 行为

- `AnyRelevantCognition` 只组织当前 Subject 已启用的 contributor；未启用领域不造成整体 degraded。
- 显式 target/cue只属于 disabled domain时返回 `Unavailable`，不能用 empty result冒充“主体没有认知”。
- QueryPlan lane weights来自 `RetrievalPolicyConfig`，本文件不再把 reference weight写成不可修改算法常量。


## Configuration Service 接入

`bind_query()` 开始时获取一次 Subject active ConfigSnapshot。QueryPlan只从该 snapshot解析一次 `RetrievalPolicy`，之后整次 query不重新读配置服务。

RRF `k`、lane weights、effort budgets、validation budgets、topology seed weights全部来自 `RetrievalPolicy`。`cognitive-retrieval`仍是唯一融合公式 owner，但**数值 owner 是已注册配置项**，不是 `match family { literal }`。

Query diagnostics记录 `snapshot.digest_for(RETRIEVAL_POLICY_KEYS)`。Subject override若允许某项高级策略，则只影响该 Subject后续 query。
