# 配置基础服务与社会认知实现规范

本目录直接约束当前代码施工。

## 阅读顺序

1. `configuration-service.md`
2. `capability-composition.md`
3. `cognitive-seed-and-self-closure.md`
4. `cognitive-query-orchestration.md`
5. `social-domain-model.md`
6. `social-authority-and-persistence.md`
7. `social-query-and-serving.md`
8. `seed-social-import.md`
9. `implementation-map.md`
10. `verification-and-maintenance.md`

## 核心边界

- Configuration Service 是基础设施，不是 cognition domain。
- 配置元数据由 owner 注册，配置值统一解析；配置服务不拥有 Memory/Self/Social 语义。
- Memory、Self、Social 相互独立；Memory-only 是正式且默认组合。
- system invariant 不注册成可修改配置。
- 一次 operation 使用同一 immutable ConfigSnapshot。
- leaf algorithm 不允许通过 global singleton/string key 临时读取配置；operation boundary 将 snapshot 解析成 typed policy 后传入算法。
- Social 仍遵守目标设计中 directed relation、degree/epistemic separation、LanguageConvention evidence 等已接受语义。

未决实现决定：无。
