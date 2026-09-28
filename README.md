# Nous Wave

Nous Wave 是一个 pre-production Subject cognition system。本仓库维护当前 TypeScript Core、Rust Kernel、Protobuf contracts、官方 TypeScript Client，以及 Rust domain/mechanism owners 的实现事实。

长期目标语义、已接受的设计决定、设计理由和研究由 [Architecture-Vault](https://github.com/Heptalogos-Devs/Architecture-Vault) 持有；本仓库不复制第二套目标 ontology。

## 开始验证

在依赖已经准备好的 checkout 中，最短的当前验证路径是：

```text
corepack pnpm check
just verify
```

Rust toolchain 由 [`rust-toolchain.toml`](rust-toolchain.toml) 固定。`just verify` 会先运行格式和 source-shape 门禁，再运行 Self domain 与串行 Self Authority focused tests，最后进入 workspace 编译、Clippy、串行全量 Rust 测试和依赖检查；串行测试避免 embedded PostgreSQL fixture 并发初始化造成资源争用。不要为了日常验证清空 `target/`；只在明确需要释放构建缓存时使用 `just clean-build`。

## 导航

- [仓库地图](INDEX.md)：顶层边界、owner 和入口。
- [实现文档说明](docs/README.md)：文档系统、Authority 和阅读路径。
- [实现文档目录](docs/INDEX.md)：按读者目标组织的维护文档。
- [当前实现架构](docs/architecture/current-implementation.md)：代码 owner 和运行边界。
- [当前状态与差距](docs/current-state/CURRENT_STATE.md)：实现事实、验证证据和未决 gap。
- [当前计划](docs/plans/README.md)：active plan、Executable Spec 和 Qualification 的关系。
- [NousQL 参考](docs/reference/NOUSQL.md) / [Self Authority 参考](docs/reference/SELF.md) / [Configuration 参考](docs/reference/CONFIGURATION.md) / [Capability 参考](docs/reference/CAPABILITIES.md)：当前接口行为。
- [脚本与维护](scripts/README.md)：检查器、配置和有边界的清理操作。
