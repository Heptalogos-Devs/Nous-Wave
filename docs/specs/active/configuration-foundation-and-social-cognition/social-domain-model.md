# Social Cognition 领域模型

## 1. crates

新增 `crates/social-domain` 与 `crates/social-service`。

`social-domain`：value objects、validation、Relation Type semantics、scope canonicalization、state transition rules。

`social-service`：persistence、idempotent mutation、evidence/provenance validation、query contributor、lifecycle/purge。

## 2. ids / refs

core新增：

```text
RelationTypeId
RelationshipAssertionId
RelationshipRevisionId
LanguageConventionId
LanguageConventionRevisionId
```

CognitiveRef新增 RelationshipAssertion/RelationshipRevision/LanguageConvention/LanguageConventionRevision。

`RelationTypeId`不是 CognitiveRef；它是 Social semantic catalog metadata。

## 3. SocialParty

```text
enum SocialParty {
    SubjectSelf,
    Entity { kind: SocialEntityKind, entity_ref: EntityRef }
}
```

`SocialEntityKind`固定：person/group/community/channel。不要 `other`。

`SubjectSelf`表示 owner Subject，不伪造 EntityRef。

## 4. Relation Type catalog

Subject-scoped immutable definition：

```text
RelationTypeDefinition
- relation_type_id
- subject_id
- key
- allowed_from_kinds[]
- allowed_to_kinds[]
- view_semantics
- degree_semantics
- temporal_semantics
- created_at
```

key：`^[a-z0-9][a-z0-9._:-]{0,127}$`，同 Subject唯一。

已有 Relationship引用后不修改定义；不同语义注册新 key。

allowed kind：subject/person/group/community/channel；from/to至少一项且无重复。

### view semantics

```text
Directed
SymmetricView
InverseView { inverse_key }
```

Authority永远只保存实际方向。Symmetric/Inverse只产生 query projection，不插入第二条 Assertion。

inverse key查询时若未注册，返回 degraded diagnostic，不伪造。

### degree semantics

```text
None
Ordinal { levels[] }
BoundedScalar { min, max }
TypedState { states[] }
```

Ordinal 2..16 levels；BoundedScalar finite 且 min<max；TypedState 1..32 states。不同 type degree不默认算术比较。

### temporal semantics

```text
State
Interval
Instant
```

State只允许 Unknown/Interval；Interval必须 Interval；Instant必须 Instant。

## 5. Relationship Assertion identity

```text
RelationshipAssertion
- relationship_id
- subject_id
- relation_type_id
- from
- to
- current_revision_id
- object_epoch
- acceptance_state
- integrity_state
- suppression_state
- purge_state
- created_at
```

同 Subject `(relation_type_id, canonical from, canonical to)` 唯一。

relation type/from/to创建后不可修改。A→B与B→A不同 object。

## 6. Relationship revision

```text
RelationshipRevision
- relationship_revision_id
- relationship_id
- subject_id
- revision_no
- parent_revision_id?
- revision_intent?
- degree?
- epistemic_class
- valid_time
- formed_at
- recorded_at
- producer_signature_id?
- supports[]
```

intent：correct/refine/reinterpret/evolve。

`evolve`保留旧 valid-time history。

## 7. degree vs epistemic state

Degree只描述关系本身；认识状态由 EpistemicClass、provenance、contradiction support表达。不加统一 confidence float。

## 8. Relationship support

至少一个 RevisionSupport。允许 SeedSupport、EvidenceRef、exact cognition dependency。

shared validate_exact_supports增加 Social exact revisions。

Derived/Inferred/Simulated relationship需要 producer signature；Seed/direct Reported/Observed可无 model producer。

## 9. SocialScope

```text
enum SocialScope {
    Person(EntityRef),
    Dyad(SocialParty, SocialParty),
    Group(EntityRef),
    Community(EntityRef),
    Channel(EntityRef),
}
```

Dyad两个 party必须不同，按 canonical string排序；顺序不同表示同 scope。

另有可选 context_scope/topic_scope token，regex `^[a-z0-9][a-z0-9._:-]{0,127}$`，只 exact match。

## 10. LanguageConvention object

```text
LanguageConvention
- convention_id
- subject_id
- key
- expression
- social_scope
- context_scope?
- topic_scope?
- current_revision_id
- object_epoch
- acceptance_state
- integrity_state
- suppression_state
- purge_state
- created_at
```

key `^[a-z0-9][a-z0-9._-]{0,127}$`，同 Subject唯一。

key/expression/scope/context/topic创建后不可修改；变化则新 object。

## 11. LanguageConvention revision

```text
LanguageConventionRevision
- convention_revision_id
- convention_id
- subject_id
- revision_no
- parent_revision_id?
- revision_intent?
- meaning
- pragmatic_role?
- epistemic_class
- valid_time
- formed_at
- recorded_at
- producer_signature_id?
- supports[]
- formation_evidence[]
```

meaning trim非空 <=64KiB；pragmatic_role optional <=256 bytes。

## 12. ConventionFormationEvidence

```text
ConventionFormationEvidence
- support_index
- kind
- external_actor?
```

kind：seed_direct / explicit_explanation / explicit_confirmation / external_consistent_use / successful_understanding / repair_sequence / contextual。

seed_direct必须引用 Seed support。

其余外部证据必须引用直接 EvidenceRef，并且 ObservationOccurrence.actor_entity_ref 与 external_actor exact match；external_actor必填。

contextual可以是 cognition dependency，但不能单独形成 accepted convention。

## 13. LanguageConvention 接受规则

create/revision至少满足一条：

A. 存在 seed_direct。

B. 至少一个 explicit_explanation 或 explicit_confirmation。

C. 至少一个 external repair_sequence。

D. `external_consistent_use` 数量至少达到当前 `SocialPolicy.repeated_external_use_min`；这些 supporting occurrences 必须提供对应数量的 known independent provenance roots。reference default 为 2。SameRoot/PartiallyShared/UnknownDependency 不增加独立计数。

E. 一个 `successful_understanding` + 一个时间不早于前者的 `external_consistent_use`，两者非 SameRoot。若 `SocialPolicy.require_same_actor_after_successful_understanding=true`（reference default），external_actor 必须相同；若显式配置为 false，则仍要求二者都是外部 actor 且各自 occurrence actor 校验通过。

只包含 contextual → reject，应留在 Runtime/Offline Hypothesis，不写 accepted LanguageConvention。

## 14. 防止自我确认

Subject自己输出不计外部证据。Host/adapter不得把 Subject输出伪装成 external actor。Subject使用 convention只形成 UseEvent，不增加独立 support。

## 15. lifecycle

Relationship/LanguageConvention均支持 withdraw/reaccept/suppress/restore/mark_revalidation_required/restore_valid/purge。

## 16. dependency invalidation

exact cognition dependency purge/withdraw/revalidation_required或 source support失效 → current dependent Social object进入 revalidation_required；不自动生成 replacement revision。


## 配置注入

`SocialService` 不在构造时冻结一个永久 policy。每次 authority mutation/query 在 operation 开始时从 Configuration Service 获取同一 Subject 的 immutable snapshot，再解析 `SocialPolicy` 并贯穿该 operation。LanguageConvention acceptance threshold 与 direct scope preference来自配置；领域身份、方向性、evidence independence等不变量仍固定。
