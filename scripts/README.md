# Repository scripts

`dev.ts` 持有开发入口，`nous.ts` 启动第一方 CLI。完整确定性验收由根 `justfile` 的 `just acceptance` 唯一编排。

发布脚本 `build-windows-kernel.ps1`、`build-postgresql.sh`、`build-ffmpeg.sh`、`build-runtime-notices.sh` 构建固定来源的私有运行时及许可材料；`pack-runtime.ts`、`assemble-portable.ts`、`zip-runtime.ps1` 生成 packs、应用闭包和 ZIP。构建时的显式下载与产品正常启动分开，普通启动只验证已安装 pack。

`research-gateway.ts` 和 `research/` 持有研究专用的持久调用计数与 proxy；生产 runtime 不承担实验次数预算。真实语料实验和 public qualification 位于 `apps/nous-cli/`，只使用 official Client。

`maintenance/` 只清理可证明的开发期生成物，默认不触碰实例数据或依赖缓存。

构建缓存默认保留以复用增量编译；需要释放工作区构建空间时使用 `just clean-build`。嵌入式 PostgreSQL 测试根目录由测试 fixture 自动清理，孤立目录可用 `just clean-test-temp` 按精确 marker 定向清理；脚本会跳过仍有活动进程的根，也可先用 `powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1 -WhatIf` 预览。

`just verify` 只运行一次必要的格式、编译、Clippy、串行 Rust tests 和依赖检查；scale/benchmark、重复 focused runner 和历史 Self/Social gate 不属于默认 verification。
