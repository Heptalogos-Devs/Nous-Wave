# Repository scripts

脚本按职责分层；当前实际存在的目录只有：

- `check/`：确定性的单项检查器。每个检查器独立返回验证结果，不负责组合其他工具。
- `check/config/`：检查器专属的声明式策略值；阈值和扫描范围不混入扫描实现。
- `maintenance/`：只针对可证明的开发期生成物执行机械清理；默认不触碰仓库数据或依赖缓存。

未来只有在出现真实的跨工具组合或共享机械代码时，才新增 `gate/`、`verify/` 或 `lib/`；目录名本身不是扩展理由。日常命令和验证顺序由根 `justfile` 唯一编排。

当前实际脚本为 `check/source_shape.py`，通过 `just structure` 调用。它默认扫描仓库内未被策略排除的 Rust 文件，也支持定向扫描和 JSON 输出：

```text
python scripts/check/source_shape.py --path crates/core/src
python scripts/check/source_shape.py --format json
```

根目录不保留同一检查器的兼容副本，避免调用方继续分叉。

构建缓存默认保留以复用增量编译；需要释放工作区构建空间时使用 `just clean-build`。嵌入式 PostgreSQL 测试根目录由测试 fixture 自动清理，孤立目录可用 `just clean-test-temp` 按精确 marker 定向清理；脚本会跳过仍有活动进程的根，也可先用 `powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1 -WhatIf` 预览。

`just verify` 的顺序是：格式检查、source-shape、快速 Self domain 测试、串行 Self Authority focused integration test，然后才进入 workspace 编译、Clippy、串行全量 Rust 测试和依赖审计。这样不会在结构性小错误或本轮语义回归已经可判定时先消耗完整编译/测试时间，也不会让 embedded PostgreSQL fixture 并发初始化争用资源。
