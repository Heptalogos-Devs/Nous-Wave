# Cognitive Seed 与 Self 收口

## 1. 新 crate

新增：

```text
crates/cognitive-seed
```

职责只有：

- 解析/验证 Cognitive Seed 文档；
- 产生 canonical AST；
- 产生 canonical semantic path。

它不写数据库、不调用模型、不直接修改 Self/Social、不持有 Subject lifecycle。

依赖：

```text
nous-core
serde
toml
```

不要让 `cognitive-seed` 依赖 `self-service` 或 `social-service`。

## 2. Subject Core

`subject-core` 继续只拥有 immutable Seed source/version、Subject adoption history、content hash/artifact identity、adoption idempotency。

`CognitiveSeedInput::validate()` 必须调用 v1 parser；声明 v1 media type却不是合法 v1 TOML时拒绝存储。

## 3. v1 顶层

```toml
schema_version = 1
```

允许顶层：

```text
schema_version
self
social
```

本次 v1 不允许未知顶层字段。

## 4. Self Facet

```toml
[[self.facets]]
kind = "identity"
key = "primary-name"
statement = "Nous"
scope = "global"
```

字段：kind/key/statement required，scope optional default=`global`。

kind固定：identity/role/capability/limitation/tendency/value/preference。

同一文档 `(kind,key)` 不能重复。

semantic path：

```text
self.facets/<kind>/<key>
```

## 5. Self Narrative

```toml
[[self.narratives]]
key = "primary"
text = """
A persistent subject...
"""
```

字段只有 key/text。

v1 Seed Narrative 不携带 reference/support 列表。

形成时：valid time=Unknown，formed_at=adoption timestamp，support=SeedSupportRef，producer=None。

semantic path：

```text
self.narratives/<key>
```

## 6. Self key

```regex
^[a-z0-9][a-z0-9._-]{0,127}$
```

1..128 bytes。

## 7. Self scope

合法：

```text
global
namespace:value
```

<=256 bytes，无 whitespace，namespace/value 均非空，只做 exact equality。

## 8. Seed support

shared core：

```text
SeedSupportRef
- seed_version_id: CognitiveSeedVersionId
- semantic_path: String
```

`RevisionSupport`：

```text
Evidence(EvidenceRef)
CognitionDependency(CognitionDependency)
Seed(SeedSupportRef)
```

semantic path由 parser/import orchestrator生成，1..512 bytes，不允许为空，不从外部 mutation API接受任意 path。

数据库 support row增加 nullable `seed_path`；非 Seed support 必须 NULL。

## 9. Deterministic import operation id

使用 UUIDv5：

```text
namespace = project constant UUID
name = "<subject-id>\n<adoption-id>\n<semantic-path>"
```

得到 domain mutation `OperationId`。

workspace `uuid` 开启 `v5` feature。

## 10. Self import

返回：

```text
SeedImportResult
- created[]
- unchanged[]
- conflicts[]
```

每项：semantic_path、target_ref?、reason?。

facet：不存在 `(kind,key)` → create；同 Seed path + statement/scope相同 → unchanged；已有不同内容/旧 Seed/后续演化 → conflict，不自动 revise。

Narrative按 `(subject,key)` 同样处理。

## 11. Partial import

一个 entry conflict不回滚已成功项；每项 mutation独立幂等。

## 12. Subject creation

对启用 Self 的 Subject：

```text
parse Seed
→ create Subject + immutable Seed adoption
→ import Self entries
```

若 adoption后进程中断，Subject与adoption保留；重试同 adoption，由 deterministic operation id继续未完成项。

不要用跨 Subject/Self 的巨大数据库 transaction掩盖恢复问题。


## Configuration Service 接入

Cognitive Seed parser/import 不从配置系统读取领域语义。只有 Subject capability defaults 来自 Configuration Service；实际 SubjectCapabilities 已持久化后，Seed import按该 capability set决定 create/deferred。Seed format、semantic path、Self key grammar是协议/语义合同，不做 configurable knob。
