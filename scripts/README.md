# Repository scripts

脚本按职责分层；当前实际存在的目录只有：

- `check/`：保留仍有当前信息价值的确定性检查器。每个检查器独立返回验证结果，不负责组合其他工具。
- `check/config/`：检查器专属的声明式策略值；阈值和扫描范围不混入扫描实现。
- `maintenance/`：只针对可证明的开发期生成物执行机械清理；默认不触碰仓库数据或依赖缓存。

未来只有在出现真实的跨工具组合或共享机械代码时，才新增 `gate/`、`verify/` 或 `lib/`；目录名本身不是扩展理由。日常命令和验证顺序由根 `justfile` 唯一编排。

根目录不保留同一检查器的兼容副本，避免调用方继续分叉。

构建缓存默认保留以复用增量编译；需要释放工作区构建空间时使用 `just clean-build`。嵌入式 PostgreSQL 测试根目录由测试 fixture 自动清理，孤立目录可用 `just clean-test-temp` 按精确 marker 定向清理；脚本会跳过仍有活动进程的根，也可先用 `powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1 -WhatIf` 预览。

`just verify` 只运行一次必要的格式、编译、Clippy、串行 Rust tests 和依赖检查；scale/benchmark、重复 focused runner 和历史 Self/Social gate 不属于默认 verification。
