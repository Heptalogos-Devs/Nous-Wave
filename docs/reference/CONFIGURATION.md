# Configuration Service 当前参考

当前配置基础服务位于 `crates/configuration`，由 Kernel 在数据库 migration 完成后打开。它只负责 registry、解析、覆盖、快照、权限、幂等 receipt 和 BLAKE3 digest，不拥有 Memory 的领域语义。

## 解析与能力

有效值按以下顺序解析：reference default → deployment `[settings]` → persisted system override → persisted subject override。每个 key 由 owner 注册 typed descriptor，包含 owner、类型、校验、暴露等级、作用域、应用方式和 semantic effect。

当前暴露等级为 `Developer`、`Advanced`、`Standard`；系统不变量没有配置 key。普通配置 mutation API 目前只在 Rust service 内部提供，未暴露未经认证的公共 RPC。

`ConfigSnapshot` 是 immutable operation input。Query bind、Serving prepare 和 Authority formation 使用一个 Subject snapshot；`digest_for` 只覆盖实际影响该结果的 key subset。`RestartProcess` override 会保存 desired value 并返回 `pending_restart`，当前 active snapshot 保持不变。

当前基础 key 包括：

- `capabilities.process.*` 与 `capabilities.subject_defaults.*`；
- `runtime.resident_limit`；
- `serving.lexical.enabled`、`serving.dense.enabled`、`serving.topology.enabled`；topology 是显式 experimental lane，默认关闭。
- `memory.accessibility.*`；
- `retrieval.rrf.*` 与 `retrieval.query.*`；
- `topology.wave.*`，包括 edge quality 和 seed weights；
- `social.language_convention.*` 与 `social.query.scope_preference`。

## 持久化

`0006_configuration.sql` 建立 `configuration_state`、system/subject overrides、mutation receipts 和 `subject_capabilities`。Subject capability 是创建时展开并保存的供给状态，不随默认配置变化。

Bootstrap 文件只保留数据库、对象存储、Server bind 和 Serving root 等启动前参数；算法与策略值位于 `[settings]` registry。
