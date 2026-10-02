# 开发脚本

从仓库根目录执行下列命令。TypeScript 入口依赖 `corepack pnpm install --frozen-lockfile`；源码位置和内部 helper 见 [INDEX.md](INDEX.md)。

## 开发实例

```text
cargo build -p nous-kernel
corepack pnpm dev:prepare
corepack pnpm dev
corepack pnpm nous status
```

`dev:prepare` 准备 PostgreSQL 18.6 runtime，不启动 Core。Windows 从 `data/runtime/packs/` 与 `data/runtime/manifest/runtimes.json` 安装已准备的 pack；其他平台使用 Kernel 的 dev-runtime 下载入口。`NOUS_WAVE_POSTGRES_RUNTIME` 可指定现有安装目录。

`dev` 启动源码 Core 和 debug Kernel，生成路径 locator 并使用 `data/instances/dev/bootstrap.toml` 与 `data/config/apps/nous.toml`。配置缺失时创建最小配置，已有文件保持原样。Kernel 未构建时直接报错；终端退出时关闭子进程。

`nous` 将参数传给源码 CLI，默认连接开发实例。用 `--home <实例目录>` 或 `--locator <bootstrap.toml>` 显式选择实例。操作示例见 [CLI README](../apps/nous-cli/README.md)。

## Runtime 构建与打包

这些脚本生成第三方 runtime，不生成用户配置。Windows x64 的 shipping runtime 使用固定 source digest 和 LLVM-MinGW UCRT。

```bash
bash scripts/runtime/build-postgresql.sh <source.tar> <llvm-mingw-dir> <data/runtime/build 下的输出目录> <data/cache/runtime-build 下的构建目录>
bash scripts/runtime/build-ffmpeg.sh <source.tar> <llvm-mingw.tar> <data/runtime/build 下的输出目录> <data/cache/runtime-build 下的构建目录>
```

PostgreSQL 输出包括 postgres、initdb、pg_ctl、pg_isready 和所需库；FFmpeg 输出包括 ffmpeg、ffprobe。两个构建脚本调用 `build-notices.sh` 收集工具链许可证，需网络；PostgreSQL 使用同目录的 setjmp patch。build-root 是可复用构建目录。

```text
corepack pnpm runtime:pack --component postgresql --version 18.6.0 --source <来源URL> --license <许可证名> --root <runtime目录> --output data/runtime
```

component/version/source/license/root 五个参数必填；component 为 `node`、`postgresql` 或 `ffmpeg`。runtime 目录必须已有必要可执行文件和 license notices。命令写入文件 digest manifest、`data/runtime/packs/` ZIP 和 `data/runtime/manifest/runtimes.json` catalog；当前只支持 Windows x64。`--output` 是 pack/catalog 输出根，默认 `data/runtime/`；runtime 目录和输出根均位于仓库 `data/` 下。

## Portable 发布

先准备 Node、PostgreSQL、FFmpeg packs/catalog，并安装 shipping Kernel 的 LLVM-MinGW 工具链。版本与 payload 合同见 [Runtime Bundle Spec](../docs/specs/active/deployment/runtime-bundle.md)。

```text
powershell -NoProfile -File scripts/release/build-kernel.ps1 -ToolchainRoot <llvm-mingw目录>
corepack pnpm release:notices
corepack pnpm assemble:portable
```

`build-kernel.ps1` 编译 `x86_64-pc-windows-gnullvm` release Kernel，复制私有 C++ runtime DLL 并 strip debug 信息。默认工具链目录为 `data/tools/llvm-mingw-windows/llvm-mingw-20260922-ucrt-x86_64`。

`release:notices` 准备应用 bundle 和依赖/运行时 license notices，需要网络。`assemble:portable` 消费这些缓存、release Kernel 和 packs，离线生成 `data/releases/windows-x64/current/` 与 `current.zip`，替换旧 current。包不包含开发配置或文档示例。`just release-prepare` 与 `just release` 分别封装准备和组装阶段。

```text
corepack pnpm release:verify --bundle data/releases/windows-x64/current.zip
corepack pnpm release:verify --bundle data/releases/windows-x64/current.zip --layout colocated
corepack pnpm release:verify --bundle data/releases/windows-x64/current.zip --layout locator --relocate
```

验证会在 `data/temp/portable/` 下的隔离目录解包并启动 portable Core，通过分发的 Client 操作实例。layout 默认 `home`，也支持 `colocated`、`locator`；`--relocate` 移动独立实例后重新启动，不能和 colocated 合用。`just release-verify` 调用以上三种布局。

`corepack pnpm release:archive` 将现有 current.zip 保存到 `data/releases/archive/windows-x64/<source-head>-<digest>.zip`，不会重新组装，同名归档已存在时报错。`zip.ps1` 是 pack/assembler 共用 ZIP helper，无需单独调用。

## Public smoke

```text
just smoke
corepack pnpm smoke:memory
corepack pnpm smoke:runtime
corepack pnpm smoke:model
```

`just smoke` 先构建 Kernel，再顺序执行三个场景；单独调用要求 debug Kernel 和 PostgreSQL runtime 已准备。

- memory：Memory 创建、检索、使用、重启与生命周期。
- runtime：WorkContext/Session 延续与 Episode exact revision。
- model：本地模型/资源 host 下的 Model、Material、External Resource 组合。

场景创建临时实例，通过正常 Core 与官方 Client 操作。`NOUS_WAVE_KERNEL_EXECUTABLE` 可指定 Kernel；`NOUS_WAVE_POSTGRES_RUNTIME` 可指定 PostgreSQL 安装。`support.ts` 是共享启动 helper。

## Live research

Research 使用真实模型，必须先准备语料、运行实例和分发的 Client 模块。方法与数据说明见 [Research](../docs/research/README.md)。

```text
corepack pnpm research:gateway --ledger data/research/runs/run-ledger.json --max-calls 1000
```

Gateway 默认转发 `http://127.0.0.1:3000/v1`，监听端口 18000；用 `--upstream`、`--port` 覆盖。`--ledger` 与 `--max-calls` 必填，预算计入失败和重试，持久化在 ledger；同路径旁保存 telemetry。将实例模型 endpoint 配为该 gateway 后再运行实验。

```text
corepack pnpm research:retrieval-live import --run-root <实例run目录> --client-module <分发client模块> --track controlled
corepack pnpm research:retrieval-live run --run-root <实例run目录> --client-module <分发client模块> --track controlled --variant baseline --output <结果.json>
corepack pnpm research:media-live --run-root <实例run目录> --client-module <分发client模块>
```

Retrieval 子命令为 `import`、`run`、`audit-formation`；track 为 `controlled` 或 `end-to-end`，variant 为 `baseline`、`model-rerank`、`wave`、`combined`。默认读取 `research/corpus/manifest.json`、`queries.json`、`data/research/corpus/unit-texts.json`，状态位于 `data/research/runs/corpus-state.json`。可用 `--manifest`、`--queries`、`--texts`、`--state`、`--output` 改路径；`--limit` 默认 0 表示全部，`--concurrency` 默认 4。导入可用 `--embedding-batch`（默认 64）、`--embedding-interval-ms`（默认 0）控制批次。

Media 默认读取 `research/corpus/media.json` 与 `data/research/corpus/raw`，将处理状态写到 `data/research/runs/media-state.json`；用 `--manifest`、`--raw-root`、`--state` 覆盖。状态文件用于继续已有实验，不会从头重复已完成操作。

## 维护

```text
python scripts/maintenance/check-doc-navigation.py
powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1 -WhatIf
```

文档检查使用 Python 3.11+，读取 Git tracked 和非 ignored Markdown，报告失效本地链接、无返回链接、孤儿页面及最近祖先 INDEX 未收录的页面。README 和普通文档参与返回/覆盖检查；INDEX、AGENTS 不要求返回，`.agents/` Skills 和作为产品输入的 `prompts/` 不属于人类文档。文档需链接回收录它的目录或其他入口；INDEX 用明确的文件链接收录页面。忽略目录与豁免页面在 [.config/scripts/doc-navigation.toml](../.config/scripts/doc-navigation.toml) 中配置，不需修改脚本；用 `--config <TOML路径>` 指定其他配置。目录和页面路径相对仓库根。发现问题退出 1，否则退出 0；这是按需工具。

PostgreSQL 清理只处理`data/temp/tests/` 中具有 PostgreSQL cluster 标记的孤立测试根，跳过运行中的 PostgreSQL。省略 `-WhatIf` 执行删除；`-MinimumAgeHours <小时>` 限制最小年龄，默认 0。它不清理语料、手写配置或开发实例。

[返回仓库地图](../INDEX.md)
