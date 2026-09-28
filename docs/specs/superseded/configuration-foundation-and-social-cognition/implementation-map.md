# 代码施工映射

本文件不新增语义，只把规范映射到当前代码。

## 新增 crate

```text
crates/configuration-service
crates/cognitive-seed
crates/social-domain
crates/social-service
```

workspace `Cargo.toml` 增加对应 path dependency。

## Configuration Service 建议文件

```text
crates/configuration-service/src/
  lib.rs
  key.rs
  registry.rs
  snapshot.rs
  service.rs
  persistence.rs
```

不要拆成更多 crate。

## Bootstrap / Kernel

修改：

```text
apps/nous-kernel/src/bootstrap.rs
apps/nous-kernel/src/lib.rs
apps/nous-kernel/src/context.rs      # 若 context 构建读取 query policy
```

`bootstrap.rs`：

- `Config` → `BootstrapConfig + raw settings table`；
- open DB/ObjectStore；
- build registry；
- open ConfigurationService；
- resolve process capabilities；
- construct optional cognition services。

`lib.rs`：

- 新增 `configuration`；
- Self改 `Option<SelfService>`；
- 新增 `social: Option<SocialService>`；
- Query contributors按 process∩subject capability组装。

## Subject Core

修改：

```text
crates/subject-core/src/lib.rs
crates/subject-core/...
```

- `CreateSubject.config` → `metadata`；
- typed `SubjectCapabilities`；
- capabilities=None时读取 ConfigurationService默认值；
- dedicated `subject_capabilities` persistence；
- SubjectView返回 capabilities。

Subject Core 可以持有 `ConfigurationService` clone，用于新 Subject默认值；已存在 Subject读取 capabilities不依赖 config。

## Memory

修改：

```text
crates/memory-service/src/accessibility.rs
crates/memory-service/src/lib.rs
```

- `AccessibilityPolicy::default()`仍可作为纯 reference struct；
- use weights / thresholds移入 struct；
- 删除 `use_weight()` literal match；
- service operation从 Subject ConfigSnapshot解析 policy；
- 不在 `MemoryService` 上长期保存一个全局 mutable accessibility policy。

## Cognitive Retrieval

修改：

```text
crates/cognitive-retrieval/src/ranking.rs
crates/cognitive-retrieval/src/graph.rs
crates/cognitive-retrieval/src/wave.rs
```

- `RRF_K` 与 `default_rrf_plan()` literal weights改为 `RetrievalPolicy`参数；
- `WaveConfig::default()`保留 reference default实现，但 production build从配置解析 policy；
- `class_quality()` literal match改为 typed TopologyPolicy；
- seed weights统一从 Retrieval/Topology policy。

## Serving

修改：

```text
crates/serving/src/*
crates/authority-store/src/serving.rs
```

- ServingService持有 ConfigurationService或在 prepare/build获得 snapshot；
- generation config digest使用 relevant config subset；
- SubjectCapabilities决定 source owner；
- Memory-only build不依赖 Self/Social。

推荐：ServingService持有 ConfigurationService clone，只在 build/prepare operation开始取一次 snapshot；artifact builder只接 typed policy。

## Self

修改：

```text
crates/self-domain
crates/self-service
```

- key/scope validation；
- `RevisionSupport::Seed(SeedSupportRef)`；
- remove owner-side RRF/shared Serving search；
- batch validation/materialization。

## Query Runtime

修改：

```text
crates/cognitive-runtime/src/query/*
```

- operation开始获取 Subject ConfigSnapshot；
- QueryPlan解析 RetrievalPolicy；
- single global fusion；
- fixed owner dispatch：memory/self/social options；
- config policy digest进 diagnostics。

## Social

新增：

```text
crates/social-domain
crates/social-service
```

按 Social specs实现。Social service依赖 ConfigurationService获取 Subject snapshot，但不依赖 Memory/Self service。

## Migration 顺序

当前 fresh schema `0001`–`0005` 后：

```text
0006_configuration.sql
0007_social_cognition.sql
```

若当前 checkout migration序号已变化，以“configuration先、social后”的语义顺序重新编号 fresh chain；不创建兼容桥接迁移。

## Proto / TypeScript

当前不把 Configuration mutation暴露成 unauthenticated public RPC。

需要更新的公开合同：

- SubjectCapabilities；
- CreateSubject `metadata/capabilities`；
- Social types/API；
- Query Social targets/cues/situation；
- Seed import result。

生成 Rust/TypeScript bindings，禁止手改 generated files。

## 文档

代码完成后更新：

```text
README.md
INDEX.md
docs/README.md
docs/INDEX.md
docs/architecture/current-implementation.md
docs/current-state/CURRENT_STATE.md
docs/reference/CONFIGURATION.md
docs/reference/CAPABILITIES.md
docs/reference/SELF.md
docs/plans/README.md
```

`docs/reference/SELF.md` 删除旧 Seed gap，并纠正“SelfDirect weight 2.0”这种硬编码叙述，改为 reference default来自配置 registry。
