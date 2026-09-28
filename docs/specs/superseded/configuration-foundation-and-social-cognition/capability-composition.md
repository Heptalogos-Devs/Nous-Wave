# 能力组合与 Memory-first 运行合同

## 1. 定位

Memory 是最小完整认知能力。Self、Social 及以后 Motivation 等高级/拟人化领域均为 optional cognition owner。

共享 Configuration/Runtime/Query/Serving 不意味着领域必须共同存在。

## 2. ProcessCapabilities

由 Configuration Service 的 Developer/SystemOnly/RestartProcess keys决定：

```rust
pub struct ProcessCapabilities {
    pub memory: bool,
    pub self_cognition: bool,
    pub social: bool,
}
```

启动 snapshot解析后再 composition。

默认：

```text
memory=true
self_cognition=false
social=false
```

这使一个普通 Agent 接入 Nous Wave 时默认只获得长期 Memory，不会被强迫建立人格/社会世界。

## 3. SubjectCapabilities

Subject Core新增 typed：

```rust
pub struct SubjectCapabilities {
    pub memory: bool,
    pub self_cognition: bool,
    pub social: bool,
}
```

持久化到 dedicated table，不塞入 `subjects.metadata`：

```sql
CREATE TABLE subject_capabilities (
    subject_id UUID PRIMARY KEY REFERENCES subjects(subject_id) ON DELETE CASCADE,
    memory BOOLEAN NOT NULL,
    self_cognition BOOLEAN NOT NULL,
    social BOOLEAN NOT NULL
);
```

该表放入 `0006_configuration.sql`，因为能力供给与 Configuration foundation 同轮建立。

## 4. CreateSubject

当前：

```text
config: serde_json::Value
```

改为：

```text
metadata: serde_json::Value
capabilities: Option<SubjectCapabilities>
```

如果 capabilities=None：

- 从 active system ConfigSnapshot 读取 `capabilities.subject_defaults.*`；
- 保存展开后的 typed flags。

Create 前验证：

```text
requested subject capabilities ⊆ process capabilities
```

不满足直接 Invalid。

`metadata` 不允许承载运行 policy/settings；per-subject settings只进入 Configuration Service overrides。

## 5. 已创建 Subject 不随默认值变化

`capabilities.subject_defaults.*` 只影响以后新建 Subject。

修改默认值：

- 不修改已存在 `subject_capabilities`；
- apply mode=`NewSubjectsOnly`。

以后要启停既有 Subject domain，应建立明确 Subject management operation，而不是改默认 config。

## 6. Kernel composition

```rust
pub struct NousRuntime {
    pub configuration: ConfigurationService,
    pub store: AuthorityStore,
    pub subjects: SubjectCoreService,
    pub cognition: CognitiveRuntimeService,
    pub memory: Option<MemoryService>,
    pub self_cognition: Option<SelfService>,
    pub social: Option<SocialService>,
    pub material: MaterialService,
    pub serving: ServingService,
}
```

所有 optional owner构造由 ProcessCapabilities决定。

`SelfService` 不再无条件创建。

## 7. 领域 crate 依赖

必须保持：

```text
memory-service  !-> self-service/social-service
self-service    !-> memory-service/social-service
social-service  !-> memory-service/self-service
cognitive-runtime !-> concrete cognition service
configuration-service !-> cognition services
```

跨域 support使用 `CognitiveRef` 和 shared support/provenance。

Social 引用 MemoryRevision 是数据依赖，不是 crate/service 启动依赖。

## 8. Query contributor selection

对 Subject：

```text
effective contributors = process capability ∩ subject capability
```

`AnyRelevantCognition`：只执行有效 contributor，disabled领域不导致 degraded。

显式 target disabled domain：

```text
Error::Unavailable("<domain> capability is disabled for subject")
```

不要返回 empty。

## 9. Serving source selection

Serving build按 SubjectCapabilities决定 source owner。

Memory-only Subject：

- lexical/dense/topology只从 Memory/CognitiveSchema 等 Memory capability来源构建；
- 不要求 Self/Social table中存在 object；
- Self/Social disabled 不产生 degradation。

## 10. Seed

Seed source永远完整保存。

如果 Seed含 Self/Social section，而 Subject没有对应 capability：

- source保留；
- import result为 deferred；
- 不创建 placeholder cognition。

以后显式启用 domain时可以对原 adoption重跑该 domain import。

## 11. 状态

RuntimeStatus列出：

```text
configuration
subject_core
cognitive_runtime
memory
self_cognition
social
serving
```

Process-disabled：`Unavailable / disabled in process composition`。

Subject capability不是 process RuntimeStatus；在 SubjectView/管理读取里单独展示。

## 12. 最小正式组合测试

必须证明：

### Memory-only

```text
process: memory=true,self=false,social=false
subject: memory=true,self=false,social=false
```

可以：create Subject → material/observation → form Memory → query → ReportUse → restart → Serving rebuild → purge。

### Memory + Self

Social关闭不影响 Self/Memory。

### Memory + Social

Self关闭不影响 Social/Memory。Social source可以来自 Seed/Evidence，不得因为缺 Self启动失败。

### Full currently implemented

Memory/Self/Social同时启用，统一 Query可以混合召回。

不要求 self-only/social-only 产品组合；但 crate owner边界不得阻止未来支持。
