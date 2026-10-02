# Repository scripts

`dev.ts` 是开发启动入口，`dev-prepare.ts` 显式准备固定 PostgreSQL runtime，`nous.ts` 启动第一方 CLI。普通检查由根 `justfile` 的 `check-fast/check` 编排。

开发用 `nous.toml` 保存在 ignored `data/dev/config/`。缺失时 Core configuration owner 创建最小开发配置；已有手写配置保留。Release assembler 不读取该配置，也不复制文档示例；portable 的 init/首次 serve 在用户实例创建活动配置。

`smoke/` 通过正常 Core 启动和 official Client 检查 Memory/restart/lifecycle、WorkContext/Episode 以及 Model/Material/External Resource wiring。

`release/` 分离应用 bundle cache、网络 notice preparation、离线 assembly、manifest 和 portable verification。默认只替换 ignored `dist/portable/windows-x64/current`；`release:archive` 显式固化发布物。`build-windows-kernel.ps1` 与 PostgreSQL/FFmpeg build scripts 保留固定 toolchain/source 与增量构建缓存。runtime packs 按版本/平台/架构/digest 复用。

`research/` 与 `research-gateway.ts` 持有真实 retrieval/media 实验与 run-owned 调用预算。结果方法见 [Research](../docs/research/README.md)。

`maintenance/` 清理明确的项目开发生成物。`just clean-test-temp` 支持清理孤立 PostgreSQL 测试根；PowerShell 的 `-WhatIf` 可预览。构建缓存保留用于增量编译。
