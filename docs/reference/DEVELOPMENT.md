# 更换开发电脑

仓库源码、自动生成的 protocol bindings、配置示例和语料 manifests 都在 Git 中，直接克隆即可。

本机需另外携带：

- `data/dev/config/nous.toml`：手写开发配置；`nous.before-budget-removal.toml` 是用户要求保留的原文件备份。
- `data/dev/secrets/gateway.env` 、`data/research/live/instance/secrets/gateway.env` 与根 `.env.local`：本地 provider 凭据，通过私有渠道传输。
- `data/research/corpus/`：已取得的语料、媒体原件及获取记录。
- `data/research/live/` 中保留的研究结果、人工修正、调用账目与素材：继续分析和比较时使用。

开发数据库已确认为可重建实验实例，无需迁移。恢复时重新安装依赖、构建 Kernel 并准备 PostgreSQL runtime，再运行 `pnpm dev`。源代码中的运行时版本、来源与 digest 定义了重新准备的输入。Windows pack 构建脚本在 `scripts/runtime/`，shipping Kernel 构建在 `scripts/release/`。

`target/`、`node_modules/`、`dist/`、runtime packs/catalog、工具链下载、build/notice caches、Serving 索引、进程 discovery 和 CodeGraph 索引都可重新生成，无需携带。
