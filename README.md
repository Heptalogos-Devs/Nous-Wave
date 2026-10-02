# Nous Wave

Nous Wave 是一个 pre-production Subject cognition system。TypeScript Core 负责公共 API、模型与资源编排；Rust Kernel 负责认知 Authority、Runtime、Material、Memory 与可重建 Serving。长期认知语义和已接受的设计决定由 [Architecture-Vault](https://github.com/Heptalogos-Devs/Architecture-Vault) 持有。

当前实现提供 Memory-only Subject、材料观察与派生、Memory 形成和生命周期、WorkContext、Episode 基础、Query/Serving、官方 Client/CLI，以及可选 External Resource。能力状态见[当前状态](docs/current-state/CURRENT_STATE.md)。

## 本地开发

前置条件：Node.js 24+、Corepack 与 Rust 1.98.1（由 rust-toolchain.toml 指定）。

```text
corepack pnpm install --frozen-lockfile
cargo build -p nous-kernel
corepack pnpm dev:prepare
corepack pnpm dev
```

开发配置和密钥位于 ignored 的 `data/config/apps/` 与 `data/config/secrets/`；实例位于 `data/instances/dev/`。另一个终端可运行 `corepack pnpm nous status`。开发与 portable 操作见[脚本说明](scripts/README.md)。

`just check-fast` 与 `just check` 是完整的项目检查入口，具体步骤见 [justfile](justfile)；使用这两个 target 需要安装 just。Protobuf 的唯一来源位于 `proto/`；修改后运行 `corepack pnpm generate`。

## 项目导航

- [仓库地图](INDEX.md)
- [当前文档目录](docs/INDEX.md)
- [当前产品合同](docs/specs/INDEX.md)
- [配置、接口与使用参考](docs/reference/README.md)
- [研究方法与观测](docs/research/README.md)
- [第一方 CLI](apps/nous-cli/README.md)
