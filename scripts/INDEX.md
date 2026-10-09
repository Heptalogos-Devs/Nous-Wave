# 脚本路由

| 责任                    | 入口                                                                                   | 调用                                                                      |
| ----------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| 开发启动                | [dev/start.ts](dev/start.ts)                                                           | `pnpm dev`                                                                |
| 开发 runtime 准备       | [dev/prepare.ts](dev/prepare.ts)                                                       | `pnpm dev:prepare`                                                        |
| 开发 CLI                | [dev/nous.ts](dev/nous.ts)                                                             | `pnpm nous`                                                               |
| 手写源码长度检查        | [dev/check-code-length.ts](dev/check-code-length.ts)                                   | `pnpm check:length`；包含在 `check:fast`                                    |
| Runtime 构建与打包      | [runtime/](runtime/)                                                                   | `pnpm runtime:pack`                                                       |
| Windows shipping Kernel | [release/build-kernel.ps1](release/build-kernel.ps1)                                   | `just release-prepare`                                                    |
| Notices                 | [release/prepare-notices.ts](release/prepare-notices.ts)                               | `pnpm release:notices`                                                    |
| 离线组装                | [release/assemble.ts](release/assemble.ts)                                             | `pnpm assemble:portable`                                                  |
| Portable 验证           | [release/verify-portable.ts](release/verify-portable.ts)                               | `pnpm release:verify`                                                     |
| 发布归档                | [release/archive.ts](release/archive.ts)                                               | `pnpm release:archive`                                                    |
| Public smoke            | [smoke/](smoke/)                                                                       | `pnpm smoke`                                                              |
| Live research           | [research/](research/)                                                                 | `pnpm research:gateway`、`research:media-live` |
| 临时 PostgreSQL 清理    | [maintenance/cleanup_embedded_postgres.ps1](maintenance/cleanup_embedded_postgres.ps1) | `just clean-test-temp`                                                    |

- [开发脚本](README.md)
- [Dogfooding 本地 Resource 宿主](research/dogfooding.ts)：复用生产 Core bootstrap，读取可变本地资料并注入研究 adapter。

## 模块与其他脚本

| 文件                                                                         | 用途                                  |
| ---------------------------------------------------------------------------- | ------------------------------------- |
| [maintenance/check-doc-navigation.py](maintenance/check-doc-navigation.py)   | 文档链接、返回路径和 INDEX 覆盖检查   |
| [release/bundle.ts](release/bundle.ts)                                       | 应用 bundle 内容缓存                  |
| [release/manifest.ts](release/manifest.ts)                                   | 生成发布文件 digest manifest          |
| [release/notices.ts](release/notices.ts)                                     | 准备/读取依赖许可证缓存               |
| [release/zip.ps1](release/zip.ps1)                                           | 将指定目录压缩为 ZIP                  |
| [research/gateway.test.ts](research/gateway.test.ts)                         | 预算、并发调用和代理 telemetry 合同   |
| [research/gateway-trace.ts](research/gateway-trace.ts) | 有界 capture、媒体描述符与敏感信息清除 |
| [research/gateway-trace.test.ts](research/gateway-trace.test.ts) | trace 边界与凭据清除合同 |
| [research/model-contracts.ts](research/model-contracts.ts) | 导出生产合同与 Prompt identity |
| [research/model-trace.ts](research/model-trace.ts) | 检查单次真实 wire attempt |
| [research/gateway.ts](research/gateway.ts)                                   | run-owned 代理及调用计数实现          |
| [research/cognitive-functional.ts](research/cognitive-functional.ts) | 连接既有 public Core，运行手工认知语料与六项 selected raw-text compatibility |
| [research/longitudinal.ts](research/longitudinal.ts) | 纵向认知模型 proposal 人工研究 |
| [research/media.ts](research/media.ts)                                       | 媒体导入、派生和检索实验              |
| [research/model-call-guard.test.ts](research/model-call-guard.test.ts)       | 预算恢复与耗尽合同                    |
| [research/model-call-guard.ts](research/model-call-guard.ts)                 | 持久化实验调用预算                    |
| [research/serve-gateway.ts](research/serve-gateway.ts)                       | 启动实验代理                          |
| [runtime/build-ffmpeg.sh](runtime/build-ffmpeg.sh)                           | 固定来源构建 FFmpeg/ffprobe           |
| [runtime/build-notices.sh](runtime/build-notices.sh)                         | 取得 MinGW/LLVM runtime notices       |
| [runtime/build-postgresql.sh](runtime/build-postgresql.sh)                   | 固定来源构建 PostgreSQL runtime       |
| [runtime/pack.ts](runtime/pack.ts)                                           | 生成 runtime ZIP、manifest 和 catalog |
| [runtime/postgresql-llvm-setjmp.patch](runtime/postgresql-llvm-setjmp.patch) | PostgreSQL LLVM-MinGW setjmp 构建修正 |
| [smoke/memory.ts](smoke/memory.ts)                                           | Memory/restart/lifecycle public 场景  |
| [smoke/model-material-resource.ts](smoke/model-material-resource.ts)         | 模型/素材/资源 public 场景            |
| [smoke/longitudinal.ts](smoke/longitudinal.ts) | 纵向认知确定性 public 场景 |
| [smoke/runtime-episode.ts](smoke/runtime-episode.ts)                         | WorkContext/Episode public 场景       |
| [smoke/support.ts](smoke/support.ts)                                         | 共享临时实例与 Core 启动              |

- [本地路径与开发 locator](workspace.ts)

- [实际 portable 开发入口](dev/portable.ts)
