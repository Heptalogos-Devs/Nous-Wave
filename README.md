# Nous Wave

Nous Wave 是 pre-production Subject cognition system。TypeScript Core 提供公共 API 与模型/资源编排；Rust Kernel 持有认知 Authority、Runtime、Material、Memory 与可重建 Serving。长期语义由 [Architecture-Vault](https://github.com/Heptalogos-Devs/Architecture-Vault) 持有。

当前支持来源材料、Memory 形成/修订/检索/使用/生命周期、WorkContext 跨 Session 延续、Episode exact revision、结构化媒体派生和 optional External Resource。

## 最短运行路径

```text
corepack pnpm install --frozen-lockfile
cargo build -p nous-kernel
just dev-prepare
corepack pnpm dev
```

开发配置和 discovery 位于 ignored `data/dev/`。`NOUS_WAVE_POSTGRES_RUNTIME` 可指定已准备的 PostgreSQL 安装。普通 serve 只使用已安装组件。

手写开发配置放在 `data/dev/config/nous.toml`，`pnpm dev` 使用已有文件，仅在缺失时创建最小开发配置。工具配置集中在 [.config/](.config/README.md)。产品配置示例见 [配置说明](docs/reference/examples/README.md)。

另一个终端用 `corepack pnpm nous status` 连接实例，具体操作见 [Nous CLI](apps/nous-cli/README.md)。

## 开发入口

`just check-fast` 顺序执行格式、Proto、TypeScript/Oxlint 和 Clippy；`just check` 随后运行 Vitest 与 Rust tests。`just audit` 用于按需依赖/安全/重复审查；`just smoke` 运行正常 Core + official Client 的 Memory、Runtime/Episode、Model/Material/Resource 场景，也可用 `pnpm smoke:memory`、`smoke:runtime`、`smoke:model` 分别运行。

Protobuf 的唯一来源是 `proto/`，修改后运行 `corepack pnpm generate`。

## Portable 与研究

Windows x64：`just release-prepare` 构建 shipping Kernel 并显式准备 notices；`just release` 离线组装 `dist/portable/windows-x64/current.zip`；`just release-verify` 检查当前包。应用 bundle、runtime pack 与 notice cache 可复用；普通构建替换 current。显式 `pnpm release:archive` 才保存不可变发布物。

Portable 不包含用户/开发配置。使用 `bin/nous.cmd init --home <instance>` 创建该实例配置，再编辑 `<instance>/config/nous.toml`；首次 serve 也可初始化，已有文件不会被覆盖。

真实模型和语料实验使用 `pnpm research:gateway`、`research:retrieval-live`、`research:media-live`，方法与观测见 [Research](docs/research/README.md)。

更换电脑时，保留的本地配置、语料与研究素材见 [开发环境迁移](docs/reference/DEVELOPMENT.md)。

## 导航

- [仓库地图](INDEX.md)
- [文档目录](docs/INDEX.md)
- [当前实现架构](docs/architecture/current-implementation.md)
- [当前状态](docs/current-state/CURRENT_STATE.md)
- [当前产品合同](docs/specs/README.md)
- [Memory Reference 里程碑](docs/roadmap/2026-10-27-memory-reference-profile.md)
