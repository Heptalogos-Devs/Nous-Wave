# Social Query 与 Serving

## 1. QueryTarget

新增 `QueryTarget::Social`。

## 2. cues

新增：

```text
SocialRelationCue
- relation_type_key?
- from?
- to?
- include_views: bool

LanguageConventionCue
- expression
- scope?
```

Relation cue至少提供 type/from/to之一。LanguageConvention expression trim后非空。

现有 `Cue::Entity` 在 target为 Social/AnyRelevant时也可触发 Relationship direct lane。

## 3. SituationDescriptor social context

```text
SocialSituation
- participants: Vec<EntityRef>
- groups: Vec<EntityRef>
- communities: Vec<EntityRef>
- channel: Option<EntityRef>
- context_tokens: Vec<String>
- topic_tokens: Vec<String>
```

SubjectSelf隐含存在。集合 sort/dedup。

## 4. EvidenceFamily

新增 `SocialRelationDirect` 与 `LanguageConventionDirect`。reference default weight均为 `2.0`，但数值来自 Configuration Service 的 `retrieval.rrf.weights.*` keys；`cognitive-retrieval`只实现统一公式和 typed policy，不重新硬编码数值。

## 5. Relationship direct lane

触发：QueryTarget::Social + Entity cue、SocialRelationCue、Exact social target。

普通 current query只返回 current/accepted/valid/normal/not-purging。

Entity cue匹配 from/to任一 endpoint entity。

Relation cue specificity顺序：type+from+to；from+to；type+one endpoint；both endpoints separately；one endpoint；type only。相同 specificity：recorded_at DESC，revision id ASC。

## 6. inverse/symmetric query view

仅 `include_views=true`。

SymmetricView reverse query返回同一 Authority exact revision；InverseView根据 inverse key在反向查询中返回原 revision。

variant标记 `social:symmetric-view` 或 `social:inverse-view:<key>`。

不创建第二条 Authority object。

inverse key未注册：不返回 view，diagnostics记录 unavailable inverse definition，不使整个 query失败。

## 7. LanguageConvention direct lane

explicit cue使用 expression exact UTF-8 equality；不 lowercase、不 NFKC、不 substring。fuzzy/textual retrieval走 shared lexical Serving。

scope cue给出时 exact equality。

## 8. social scope applicability

Situation存在时：Person必须在 participants；Dyad含 SubjectSelf时另一个 entity必须在 participants，两个 external时两者都在；Group/Community/Channel exact匹配对应 situation。

context/topic scoped convention要求 query token集合包含该 token。

Situation缺失时：explicit exact Convention target仍可读；direct cue只有给 exact scope才返回；lexical generic hit可以返回但 representation必须包含 scope，不能伪装当前适用。

## 9. scope preference

同 expression 且多个 applicable convention 时，scope 主排序顺序来自当前 `SocialPolicy.scope_preference`；reference default 为 `dyad > person > channel > group > community`。随后 context exact scoped优先、topic exact scoped优先、`recorded_at DESC`、revision id ASC。

只是 retrieval preference，不删除更广 scope含义。

## 10. Serving document

Relationship canonical text：

```text
relationship
type=<key>
from=<party>
to=<party>
degree=<value-or-none>
```

不用 LLM生成自然语言 projection。

LanguageConvention canonical text包含 expression、meaning、pragmatic_role?、scope canonical description。

Serving record exact ref为 current revision。

## 11. lexical/dense

shared lexical/dense lane只运行一次，可以返回 Memory/Schema/Self/Social，再按 ref owner batch validation。

Social service自己不得直接搜索 Tantivy/usearch。

## 12. materialization

Relationship semantic_role=`social:relationship:<type-key>`，authority=SubjectCognition，entity_refs为 external endpoints，representation为 deterministic structural representation。

Convention semantic_role=`social:language-convention`，representation包含 expression+meaning+role+scope。

need_evidence=true返回真实 support handles。

## 13. Runtime use

Social hit可进入 context/resident。被检索/呈现不自动产生 meaningful UseEvent；实际引用/使用/据此行动再由调用方报告。

## 14. AnyRelevantCognition

Social不因启用 domain就把全部 social cognition灌入 query。

Direct lane至少需要 Social target、Social cue、Entity cue或 exact Social ref。

Text-only AnyRelevant只能通过 shared lexical/dense召回 Social。

## 15. topology

本次不把 Relationship Assertion自动转换为普通 Wave edge。未来 Social-derived topology必须是 rebuildable Serving projection并另行定义 conductance。


## Serving domain composition

Social projection只在 process capability 与 Subject capability均启用时进入 shared Serving。Memory-only Serving不得依赖 Social tables/object存在。


## 配置快照

同一 query 使用与 `cognitive-query-orchestration.md` 相同的 Subject ConfigSnapshot。`social.query.scope_preference` 从 snapshot解析；Direct lane排序期间不得再次读取配置服务。
