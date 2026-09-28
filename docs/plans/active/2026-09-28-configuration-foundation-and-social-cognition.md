# 配置基础服务与社会认知开发计划

日期：2026-09-28
状态：ACTIVE

## 目标

本次施工完成两个连续目标：

1. 建立 Configuration Service 作为所有 subsystem/domain 可注册并读取的基础服务，正式支持配置暴露层级、system/subject scope、持久覆盖、不可变快照与 digest；
2. 在该基础上实现可选 Social Cognition，同时收口 Cognitive Seed、Self 与共享 Query，使 Memory-only 继续作为完整且默认的能力组合。

配置服务先实现，是因为 Social 也会引入 formation/query policy。如果继续在 Social 里直接写常量，会把已经暴露的问题再次复制。

## 施工顺序

### 配置基础

- 合并 Architecture-Vault 目标设计和设计决定；
- 新增 `nous-configuration-service`；
- 建立 registry、typed key、描述元数据、解析、snapshot、persisted overrides；
- 改造 bootstrap，使数据库/对象存储保留最小 bootstrap config，算法/策略进入 Configuration Service；
- 把 Memory accessibility、Cognitive Retrieval、Serving/Topology 当前 reference 参数迁入注册 key；
- Process Capability 改为配置，Subject Capability 改为 Subject Core typed persistence；
- Memory-only 作为默认供给。

### Cognitive Seed / Self / Query 收口

- 完整实现 Cognitive Seed TOML v1 与 semantic path；
- 修正 Self key/scope 与 SeedSupportRef；
- Query 改为统一 lane aggregation + single fusion + owner batch materialization；
- Self 不再自行执行 RRF/shared lexical/dense。

### Social Cognition

- 新增 `social-domain` / `social-service`；
- Relation Type catalog；
- directed Relationship Assertion；
- degree semantics 与 epistemic state 分离；
- LanguageConvention、social scope 与外部社会证据 acceptance；
- Social query/Serving；
- Cognitive Seed Social import；
- Social 可按 Process/Subject capability 独立启停。

### 文档与验证

- 更新 current implementation/reference/index/plan 状态；
- 只增加配置基础、Memory-only、统一 query、Social 关键语义需要的测试；
- 运行 touched scope 检查，最后 `corepack pnpm check`、`just verify`、`just nextest`。

## 明确不做

- 设置 UI；
- 未经认证的公共配置修改 API；
- dynamic plugin config registry；
- arbitrary runtime hot-reload bus；
- 把系统不变量变成配置 boolean；
- Social graph path 自动形成 Authority；
- person identity 自动 merge；
- Motivation / Desired Condition；
- 新图算法 benchmark；
- 覆盖率目标或全仓 mutation testing。


