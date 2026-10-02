# 脚本路由

| 责任 | 入口 | 调用 |
| --- | --- | --- |
| 开发启动 | [dev/start.ts](dev/start.ts) | `pnpm dev` |
| 开发 runtime 准备 | [dev/prepare.ts](dev/prepare.ts) | `pnpm dev:prepare` |
| 开发 CLI | [dev/nous.ts](dev/nous.ts) | `pnpm nous` |
| Runtime 构建与打包 | [runtime/](runtime/) | `pnpm runtime:pack` |
| Windows shipping Kernel | [release/build-kernel.ps1](release/build-kernel.ps1) | `just release-prepare` |
| Notices | [release/prepare-notices.ts](release/prepare-notices.ts) | `pnpm release:notices` |
| 离线组装 | [release/assemble.ts](release/assemble.ts) | `pnpm assemble:portable` |
| Portable 验证 | [release/verify-portable.ts](release/verify-portable.ts) | `pnpm release:verify` |
| 发布归档 | [release/archive.ts](release/archive.ts) | `pnpm release:archive` |
| Public smoke | [smoke/](smoke/) | `pnpm smoke` |
| Live research | [research/](research/) | `pnpm research:gateway`、`research:retrieval-live`、`research:media-live` |
| 临时 PostgreSQL 清理 | [maintenance/cleanup_embedded_postgres.ps1](maintenance/cleanup_embedded_postgres.ps1) | `just clean-test-temp` |
