# Social Authority 与持久化

## 1. migration

fresh schema增加 `0007_social_cognition.sql`，不建立旧 schema兼容迁移。

## 2. Relation Type table

```text
relation_type_definitions
- relation_type_id uuid PK
- subject_id uuid NOT NULL
- key text NOT NULL
- allowed_from_kinds text[] NOT NULL
- allowed_to_kinds text[] NOT NULL
- view_kind text NOT NULL
- inverse_key text NULL
- degree_kind text NOT NULL
- degree_config jsonb NOT NULL
- temporal_kind text NOT NULL
- created_at timestamptz NOT NULL
UNIQUE(subject_id,key)
```

## 3. Relationship tables

`relationship_assertions`：relationship_id、subject_id、relation_type_id、from_kind/from_entity_ref、to_kind/to_entity_ref、current_revision_id、object_epoch、四类 lifecycle state、created_at。

subject endpoint的 entity_ref必须 NULL；其他 kind必须 NOT NULL。

用 expression unique index保证 subject+type+from-kind/ref+to-kind/ref 唯一。

`relationship_revisions`：revision id/object/subject/revision_no/parent/intent/degree_kind/degree_value jsonb/epistemic_class/valid time/formed/recorded/producer。

`relationship_revision_supports`采用 shared support column pattern，加 `seed_path`。

## 4. LanguageConvention tables

`language_conventions`：convention_id、subject_id、key、expression、scope_kind、scope_refs text[]、context_scope、topic_scope、current_revision_id、object_epoch、四类 lifecycle、created_at，UNIQUE(subject_id,key)。

`language_convention_revisions`：revision/object/subject/revision_no/parent/intent/meaning/pragmatic_role/epistemic/valid time/formed/recorded/producer。

`language_convention_revision_supports`采用 shared support columns。

`language_convention_formation_evidence`：convention_revision_id、support_ordinal、evidence_kind、external_actor_ref，主键 `(revision,support_ordinal)`。

## 5. indexes

Relationship：subject+current revision、subject+type、from entity partial index、to entity partial index。

Convention：subject+expression、GIN scope_refs、subject+current revision。

不要提前加 graph database。

## 6. mutation receipts

继续 operation_id + canonical request digest。same-id/same-digest返回原结果；same-id/different-digest Conflict。Relation Type register也必须幂等。

## 7. relationship create transaction

validate input → lock operation → receipt → lock Subject → load exact Relation Type → validate endpoint/degree/temporal/support/producer → triple uniqueness → insert object/revision/support → authority_seq+1 → invalidate Serving → receipt commit → transaction commit。

## 8. relationship revise

lock op/receipt/object → expected epoch → parent=current → active lifecycle → immutable type/from/to → validate degree/temporal/support → insert revision → switch head → epoch+1 → authority_seq+1 → invalidate Serving → receipt → commit。

## 9. convention create/revise

同 Relationship，再增加 scope canonicalization、formation evidence acceptance、external actor/Occurrence核验、provenance independence核验。

## 10. domain-neutral persistence helper

当前 Self `support.rs` 中真正通用的 receipt、temporal encode/decode、exact support ownership、insert/load support、evidence locator ownership移到 `authority-store::cognition`。

Self/Social共同使用。Memory不为本次强制大迁移，只在能直接删重复且不扩大施工面时复用。

## 11. producer

EpistemicClass::Derived/Inferred/Simulated需要 producer signature。Seed/Host explicit/direct Reported/Observed可无 model producer。

## 12. purge

object lock + expected epoch → purging → invalidate Serving → 删除 revision payload/support/formation annotations → 删除 Runtime resident refs/detailed UseEvents → 保留最小 purge receipt/tombstone → object删除或墓碑。

不得删除 Observation/Artifact/Seed/Memory/Self source。

## 13. dependency invalidation

扩展 dependency lookup，使 Social exact revisions既可成为 dependency target，也可作为 dependent current revision被标记 revalidation_required。

只沿 exact revision support graph，不沿 relationship graph path传播 lifecycle。


## Formation policy digest

`language_convention_revisions` 增加：

```text
formation_policy_digest TEXT NOT NULL
```

create/revise开始时固定 Subject ConfigSnapshot，使用 `SOCIAL_FORMATION_POLICY_KEYS` 子集计算 digest。验收判断和数据库写入都使用同一 resolved SocialPolicy。配置在事务执行中变化不能改变当前 operation。

Relationship assertion 当前没有可调 formation acceptance policy，不保存该 digest；以后若引入真正的可调 formation policy，再增加对应字段，不提前泛化所有 cognition revision。
