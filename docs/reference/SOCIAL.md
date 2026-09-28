# Social Cognition 当前接口

Social owner 由 `crates/social-domain` 与 `crates/social-service` 组成；它不依赖 Memory 或 Self service。当前 public/private protocol 与 official Client 提供 Relation Type、Relationship、LanguageConvention 的 create/revise/lifecycle contracts。

## Authority objects

- `RelationTypeDefinition` 是 Subject-scoped immutable catalog definition，声明允许的 party kind、directed/symmetric/inverse view、degree semantics 和 temporal semantics。
- `RelationshipAssertion` 的 identity 是 `(subject, relation_type, from, to)`；A→B 与 B→A 不合并。revision 持有 degree、epistemic class、valid time、supports 和 producer。
- `LanguageConvention` 的 identity 是 `(subject, key)`；expression、scope、context/topic scope 创建后不变，含义通过 revision 演化。
- `SocialScope::Dyad` canonicalize party 顺序；scope preference 与 LanguageConvention acceptance threshold 来自 Configuration Service。形成 revision 保存 `formation_policy_digest`。

## Query/Serving

`QueryTarget::Social`、`SocialRelationCue` 和 `LanguageConventionCue` 产生 Social direct lanes。Runtime 统一 RRF 后由 Social owner batch materialize；Relationship/Convention projection 使用确定性结构化文本，Serving 仍是可重建 projection。

检索命中、呈现和 Subject 自身重复输出不自动产生长期 meaningful use。Social lifecycle/purge 会失效 projection；source Observation/Artifact 不随 cognition purge 删除。

## Verification status

当前 dedicated focused test 已覆盖 directed Relationship identity、revision materialization、Social cue query 和 Memory+Social capability composition。revision/lifecycle/purge 的 full failure/restart/dependency Qualification 尚标记 `NOT_RUN`，见 active verification Spec。
