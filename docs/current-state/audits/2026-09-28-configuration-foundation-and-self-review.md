# 配置基础与 Self 实施审查

日期：2026-09-28
核查基线：`28a0212dbbbe3e0eed7a5de969f77476853da01a`

## 结论

Self Authority 的对象/修订主体已经存在，但当前运行组合与配置方式还没有达到目标边界。下一次施工把配置基础服务与 Social Cognition 一起实现，不另开修复轮。

## 已确认的组合问题

当前 Kernel：

```text
memory: Option<MemoryService>
self_cognition: SelfService
```

Memory 已可选，Self 却被无条件创建；bootstrap 只有 `memory_enabled`。这会把完整主体能力反向变成 Memory 使用者的前置成本。

目标应为：

```text
configuration: ConfigurationService
memory: Option<MemoryService>
self_cognition: Option<SelfService>
social: Option<SocialService>
```

Memory-only 必须是默认、完整、长期支持的组合。

## 当前配置碎片化

现有 bootstrap 已能设置数据库、对象存储、`resident_limit`、Memory enable、Serving lexical/dense/topology 开关，并直接反序列化部分 `AccessibilityPolicy`。但以下数值仍散落在 owner 代码中：

- Accessibility meaningful-use weights 与 Normal/Deep/Explicit threshold；
- RRF `k` 与 lane weights；
- Query effort/candidate/validation budgets；
- Wave seed weights、edge class quality；
- Serving 构图时使用的 `WaveConfig::default()`；
- 未来 Social LanguageConvention acceptance 数值。

把这些字段继续塞进 `apps/nous-kernel/src/bootstrap.rs` 的大 `Config` 结构并不能解决问题：其他代码无法独立注册、描述、分层和按 Subject 解析配置，未来 Management API/UI 仍要重新实现一套元数据。

## Configuration Service 缺失

当前没有：

- 配置 key registry；
- owner registration；
- Developer/Advanced/Standard 暴露元数据；
- system/subject scope policy；
- persisted overrides；
- immutable resolved snapshot；
- relevant-key digest；
- config source diagnostics。

因此下一次施工必须建立基础服务，而不是继续扩充 Kernel config struct。

## Subject Core 的 generic `config` 字段

`CreateSubject` / `SubjectView` 当前公开一个任意 `serde_json::Value config`，实际写入 `subjects.metadata`。它会与新的 Configuration Service 形成第二套“Subject config”入口。

处理决定：

- public `config` 改名/收敛为 `metadata`，明确它不是运行策略配置；
- Subject capabilities 使用 typed 字段和 dedicated persistence；
- per-subject 设置覆盖进入 Configuration Service，不塞回 metadata。

## Query / Self 仍需随施工收口

Self 加入后，领域 owner 仍可各自完成 RRF/Serving 搜索并返回完整结果，导致跨领域并非真正一次融合。Social 加入前必须把 query 改为：domain 产生 direct lane + batch validation/materialization，统一 fusion 只存在于 `cognitive-retrieval`。

Cognitive Seed TOML Narrative contract 在上一份规范中被截断，Seed parser/import 尚未形成；本包重新给出完整定义。

## 工具链

本次不继续扩建验证流程。工具只用于防止 AI 编码出现重复、巨大生产文件、死代码、边界破坏和依赖问题。测试保护真实语义与已发现问题，不按覆盖率或数量增长。


