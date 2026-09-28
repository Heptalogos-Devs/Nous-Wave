# Cognitive Seed 的 Social 初始化

## 1. relation type definitions

```toml
[[social.relation_types]]
key = "friend"
allowed_from = ["subject", "person"]
allowed_to = ["person"]
view = "directed"
temporal = "state"

[social.relation_types.degree]
kind = "ordinal"
levels = ["weak", "moderate", "strong"]
```

degree还支持 none、bounded_scalar(min/max)、typed_state(states)。

inverse使用 `view="inverse"` + `inverse_key`；symmetric使用 `view="symmetric"`。

unknown fields拒绝。

semantic path：`social.relation-types/<key>`。

Relation Type不是 cognition support target；catalog保存 source_seed_version_id/source_seed_path。

## 2. Relationship entry

```toml
[[social.relationships]]
key = "alice-friend"
type = "friend"

[social.relationships.from]
kind = "subject"

[social.relationships.to]
kind = "person"
ref = "entity:person:alice"

[social.relationships.degree]
kind = "ordinal"
value = "strong"
```

`epistemic` optional default reported。

valid_time optional，格式支持 unknown/instant(at)/interval(start/end)。

`key`只用于 Seed path/import diagnostics，不是 Relationship object identity。

semantic path：`social.relationships/<key>`，同 Seed key唯一。

## 3. Convention entry

```toml
[[social.conventions]]
key = "alice-project-x"
expression = "X"
meaning = "the private project codename"
epistemic = "reported"
pragmatic_role = "shorthand"
context = "conversation:project"
topic = "project:x"

[social.conventions.scope]
kind = "dyad"
parties = [
  { kind = "subject" },
  { kind = "person", ref = "entity:person:alice" }
]
```

semantic path：`social.conventions/<key>`。

Seed形成：support=SeedSupportRef，formation evidence=seed_direct，valid_time=Unknown，formed_at=adoption timestamp，producer=None。

## 4. Import ordering

一次 adoption解释顺序：Relation Type definitions → Self facets → Self narratives → Relationships → LanguageConventions。

这只是依赖顺序，不是 Authority优先级。

## 5. Relation Type import

key不存在 → created；same key + definition完全相同 → unchanged；same key但结构语义不同 → conflict，不修改原定义。

## 6. Relationship import

按 relation_type+from+to找 object。不存在 → create。current semantic content等价且含同 seed path → unchanged。其他 → conflict，不自动 revise/evolve。

## 7. Convention import

按 `(subject,key)`。不存在 → create。immutable fields + current meaning/role相同并含同 Seed path → unchanged。其他 → conflict。

## 8. Social disabled

Subject未启用 Social：source保留，不创建 placeholder object；结果标记 `deferred_domain_disabled`，不是 conflict。以后启用 Social可对同 adoption重跑。

## 9. public result

```text
CognitiveSeedImportResult
- created[]
- unchanged[]
- conflicts[]
- deferred[]
```

每项 semantic_path、domain、target_ref?、reason?。

## 10. parser tests

只测试项目 contract：完整 Narrative multiline、unknown field、duplicate keys、scope canonicalization、degree config、malformed endpoint、semantic path稳定。不要测试 TOML library自身。


## 领域独立导入

Seed import按 Subject capability分别调用 Self/Social owner。Social disabled不能阻止 Self/Memory初始化；Self disabled同理。Seed source始终完整保留，未启用领域只记录 deferred import结果。
