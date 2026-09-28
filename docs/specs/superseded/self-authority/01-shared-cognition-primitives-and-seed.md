# 01 — Shared Cognition Primitives 与 Cognitive Seed

## 1. Shared cognition primitives

Self 不得依赖 `memory-domain` 才能使用 cognition lifecycle / evidence support。

将以下真正跨 cognition domain 的类型移入 `nous-core` 的独立模块，例如 `cognition.rs` / `support.rs`：

- `AcceptanceState`
- `IntegrityState`
- `SuppressionState`
- `PurgeState`
- `EvidenceLocator`
- `SupportRole`
- `EvidenceRef`
- `EvidenceRoot`
- `EvidenceRootCertainty`
- `DependencyRelation`
- `CognitionDependency`
- `RevisionSupport`

Memory 修改为直接使用 shared 类型。

以下继续留在 Memory：

- `CognitiveRole`
- `FormationMode`
- Memory `RevisionIntent`
- Accessibility
- MemoryRelation
- CognitiveSchema
- AssociationEvidence

不要新建一个抽象层级很深的“domain framework”。

## 2. 新的身份类型

在 core 增加：

```text
CognitiveSeedVersionId
SelfFacetId
SelfFacetRevisionId
NarrativeIdentityId
NarrativeIdentityRevisionId
```

加入 `CognitiveRef`：

```text
CognitiveSeedVersion(...)
SelfFacet(...)
SelfFacetRevision(...)
NarrativeIdentity(...)
NarrativeIdentityRevision(...)
```

更新：

- Display
- reference_parts
- parse_reference
- Subject ownership validation
- Proto / generated bindings / TypeScript client

Self / Narrative mutable object ref不能作为长期 dependency support；support必须 exact revision。

## 3. Character Seed 退出 active model

删除 active code 中：

```text
CharacterSeedInput
CharacterSeedView
character_seeds
revise_character_seed
```

不建立 alias / compatibility wrapper。

替换为：

```text
CognitiveSeedInput
CognitiveSeedVersion
adopt_cognitive_seed
cognitive_seed_versions
subject_seed_adoptions
```

## 4. Cognitive Seed version

Seed version是 immutable source snapshot。

```text
CognitiveSeedVersion
- seed_version_id
- subject_id
- artifact_id
- format
- provenance
- content_hash
- created_at
```

一个 Subject可以历史上采用多个 Seed version。

这不是 Self 当前状态。

## 5. Subject seed adoption

```text
SubjectSeedAdoption
- adoption_id
- subject_id
- seed_version_id
- kind
- operation_id
- request_digest
- adopted_at
```

`kind`：

```text
initial
import
```

同一个 operation id + same digest幂等。

same operation id + different digest -> Conflict。

历史 adoption不可修改。

## 6. Seed format

自动解释 Self 的 reference format固定为：

```text
application/vnd.nous-wave.cognitive-seed+toml;version=1
```

内容：

```toml
schema_version = 1

[[self.facets]]
kind = "identity"
key = "primary-name"
statement = "Nous"
scope = "global"

[[self.facets]]
kind = "role"
key = "assistant"
statement = "..."
scope = "global"

[[self.facets]]
kind = "capability"
key = "software-architecture"
statement = "..."
scope = "global"

[[self.facets]]
kind = "limitation"
key = "physical-presence"
statement = "..."
scope = "global"

[[self.facets]]
kind = "tendency"
key = "communication-style"
statement = "..."
scope = "global"

[[self.facets]]
kind = "value"
key = "accuracy"
statement = "..."
scope = "global"

[[self.facets]]
kind = "preference"
key = "response-structure"
statement = "..."
scope = "global"

[[self.narratives]]
key = "primary"
text =
