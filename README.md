# Nous Wave

Nous Wave 是一个 pre-production Subject cognition system。本仓库当前实现 Memory-only Reference Profile：TypeScript Core 提供 public host，Rust Kernel 持有认知 Authority、Runtime、Material、Memory 和可重建 Serving。

长期目标语义、已接受决定和研究由 [Architecture-Vault](https://github.com/Heptalogos-Devs/Architecture-Vault) 持有；本仓库维护当前实现与实际验证证据。

## 当前可运行能力

当前主线支持 Artifact/Observation → grounded Memory → public Query/Serving → meaningful UseEvent → restart/rebuild → suppression/restore/purge → provenance trace-back。Self、Social、Motivation、Episode/Journal、Offline Cognition 和 Heptalogos live integration 不属于当前 executable scope。

## 最短运行路径

依赖准备并构建 Kernel 后：

```text
corepack pnpm dev
```

该命令生成 repo-owned dev 配置，选择当前平台的 Kernel binary，使用 private loopback/ephemeral Kernel port，并将 Core discovery 写入 `data/dev/runtime/core.json`。Kernel binary 不存在时会直接提示先运行 `cargo build -p nous-kernel`。

完整 public acceptance：

```text
corepack pnpm qualification:memory-reference
```

它只经过 Core 与官方 TypeScript Client，验证 Memory-only Subject、双 Session、Query、UseEvent retry、restart、suppress/restore、purge 和 provenance trace-back。

## 开发与验证

```text
corepack pnpm typecheck
corepack pnpm test
corepack pnpm check
just verify
```

`proto/` 是唯一 wire-contract source；Rust/TypeScript generated bindings 由 `corepack pnpm generate` 生成。Cargo `target/` 用于增量构建，不作为日常清理对象。

## 导航

- [仓库地图](INDEX.md)
- [实现文档目录](docs/INDEX.md)
- [当前实现架构](docs/architecture/current-implementation.md)
- [当前状态](docs/current-state/CURRENT_STATE.md)
- [当前计划](docs/plans/README.md)
- [当前 Memory Reference Specs](docs/specs/active/memory-reference-profile/README.md)
- [Rust crate map](crates/INDEX.md)
