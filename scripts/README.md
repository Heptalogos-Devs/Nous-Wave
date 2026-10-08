# 开发脚本

[返回仓库地图](../INDEX.md)

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

## 配置检查与真实 portable 开发

完整配置说明保存在 [examples/nous.toml](../docs/reference/examples/nous.toml)，含当前版本与注释。修改 `data/config/apps/nous.toml` 后用当前源码离线检查：

```text
corepack pnpm nous config check
corepack pnpm dev:portable config check
corepack pnpm dev:portable serve
corepack pnpm dev:portable status
```

`dev:portable` 默认使用 `data/releases/windows-x64/current/` 中的 Node 和 launcher，进而使用包内 Core/Kernel/Runtime；可用 `--bundle <解包目录>` 指定另一实际包。配置和密钥直接引用 `data/config/apps/`、`data/config/secrets/`，不复制或覆盖。生成的 locator 位于 `data/instances/portable/bootstrap.toml`，Authority/对象/身份与源码开发实例独立。无参数等同 `serve`；关闭规则由正常 launcher 承担。

包尚未组装时该命令明确失败。配置字段与 `config_revision` 必须符合所选 Core package 的合同；版本不匹配时，按该 package 合同更新配置，或选择与配置匹配的 package。

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

`release:notices` 准备应用 bundle 和依赖/运行时 license notices，需要网络。`assemble:portable` 消费这些缓存、release Kernel 和 packs，离线生成 `data/releases/windows-x64/current/` 与 `current.zip`，替换当前输出。包不包含开发配置或文档示例。`just release-prepare` 与 `just release` 分别封装准备和组装阶段。

`assemble:portable --runtime-root <packs/catalog 根目录>` 可显式选择独立的 shipping packs；该目录包含 `manifest/runtimes.json` 与 `packs/`。默认仍为 `data/runtime/`。开发与 shipping runtime 内容不同时，应使用独立 catalog，避免更换正在运行实例所依赖的 manifest identity；组装只将选中的 catalog 与 runtime 放进发布包。

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
corepack pnpm smoke:longitudinal
```

`just smoke` 先构建 Kernel，再顺序执行四个场景；单独调用要求 debug Kernel 和 PostgreSQL runtime 已准备。

- memory：Memory 创建、检索、使用、重启与生命周期。
- runtime：WorkContext/Session 延续与 Episode exact revision。
- model：本地模型/资源 host 下的 Model、Material、External Resource 组合。
- longitudinal：Session Observation → automatic Episode → Journal → Memory consolidation，随后重开服务、exact/lexical 查询、WorkContext continuation 和 meaningful UseEvent 重试。模型 proposal 使用确定性 stub；该场景检查编排与 Authority 语义。

`smoke:longitudinal` 调用 Rust test harness 启动临时 PostgreSQL 和真实 Kernel gRPC；TypeScript 场景托管真实 Core HTTP 并使用官方 Client。ManualCognitiveClock 通过测试子进程的 stdin/stdout 控制，未增加产品 RPC。该测试也由 `just check` 的 workspace tests 执行。

场景创建临时实例，通过正常 Core 与官方 Client 操作。`NOUS_WAVE_KERNEL_EXECUTABLE` 可指定 Kernel；`NOUS_WAVE_POSTGRES_RUNTIME` 可指定 PostgreSQL 安装。`support.ts` 是共享启动 helper。

## Live research

手动媒体与模型研究使用真实模型，必须先准备语料、运行实例和分发的 Client 模块。自动功能验收使用 deterministic provider。方法与数据说明见 [Research](../docs/research/README.md)。

```text
corepack pnpm research:gateway --ledger data/research/runs/run-ledger.json --max-calls 1000
```

Gateway 默认转发 `http://127.0.0.1:3000/v1`，监听端口 18000；用 `--upstream`、`--port` 覆盖。`--ledger` 与 `--max-calls` 必填，显式累计调用上限允许 1–100000，预算计入失败和重试，持久化在 ledger；同路径旁保存 telemetry。将实例模型 endpoint 配为该 gateway 后再运行实验。 可加 `--trace-root data/research/runs/<run>/traces` 保存每次 attempt 的 `meta.json` 和经过敏感信息清除的 request/response。request capture 上限为 96 MiB，response 为 1 MiB；超限只保存大小、digest 和 truncation 状态。大型 data/base64 媒体与 multipart file 保存 media type、byte count 与 SHA-256 描述符；multipart 保留 model/language 等普通字段，binary 不作为文本落盘。Authorization、cookie 和 credential header 不落盘，已知凭据回显也在写盘前清除。trace 文件使用 0600 权限。

```text
corepack pnpm research:media-live --run-root <实例run目录> --client-module <分发client模块> --unit nasa-menon-conversation --strategy direct_structured --derive-only
```

Media 默认读取 `docs/research/corpus/media.json` 与 `data/research/corpus/raw`，将处理状态写到 `data/research/runs/media-state.json`；用 `--manifest`、`--raw-root`、`--state` 覆盖。状态文件用于继续已有实验，不会从头重复已完成操作。

媒体 runner 的 `--unit`、`--strategy` 可重复指定，`--derive-only` 只验证派生与来源图，`--skip-retrieval` 执行派生和 formation、暂不准备 embedding/query。`--allow-degradation <code>` 可重复声明实验预期的降级（例如 frames-only 的 `video_audio_not_interpreted`），结果仍保存全部 degradation；其他降级继续使 pipeline 失败。Receipt 与完整 manifest digest 绑定；更换语料用新的 state。人工评阅应核对实际输出，pattern 命中只覆盖预先声明的事实。Manifest 的 `oracle_patterns_by_representation_kind` 可为 Transcript 等表示声明与其职责相符的事实 oracle，未声明时使用共同 `oracle_patterns`。

## 模型合同与 trace 检查

```text
corepack pnpm inspect:model-contracts --all
corepack pnpm inspect:model-contracts --role memory_formation --config data/config/apps/nous.toml --output data/research/inspection/formation
corepack pnpm inspect:model-contracts --role material_description --prompt-path material/video-description.md --adapter gateway-chat-media-v1 --config data/config/apps/nous.toml --output data/research/inspection/video
corepack pnpm inspect:model-trace --root data/research/runs/<run>/traces --attempt 1
```

合同检查消费 production invocation 同一 Structured Contract Registry 和 PromptRegistry，导出 `contract.json`、实际 provider `schema.json` 与 `prompt.md`；未使用 Structured Output 的角色没有 schema 文件。配置可选；提供时导出无凭据的 model/protocol、profile 与 role configuration digest。`--prompt-path`（须配 `--role`）选择实际 invocation 的 Prompt override，`--adapter` 计入生产 invocation 的 adapter identity；两者沿用 production digest owner。该命令不调用模型，也不要求读取 credential 值。生成文件只能写入 ignored `data/research/`，不形成第二份 schema source。

Trace 检查显示 endpoint、model、状态、usage、wire digest、capture 状态及文件路径，并从捕获的 Prompt/schema 确定性匹配当前角色。`--prompt-path` 与 `--override-prompt-root` 可用于 custom Prompt；视频默认 Prompt 自动参与匹配。embedding/rerank/transcription 按实际 endpoint 识别角色。模型 JSON 输出可以对当前匹配的 Zod owner 复验；原 invocation 的 owner commit/degradation 仍由对应研究 runner 的实际结果确认。更换 Prompt/schema 后，旧 trace 不会伪称匹配当前合同。

## 维护

```text
python scripts/maintenance/check-doc-navigation.py
powershell -NoProfile -File scripts/maintenance/cleanup_embedded_postgres.ps1 -WhatIf
```

文档检查使用 Python 3.11+，读取 Git tracked 和非 ignored Markdown，报告失效本地链接、无返回链接、孤儿页面及最近祖先 INDEX 未收录的页面。README 和普通文档参与返回/覆盖检查；INDEX、AGENTS 不要求返回，`.agents/` Skills 和作为产品输入的 `prompts/` 不属于人类文档。文档需链接回收录它的目录或其他入口；INDEX 用明确的文件链接收录页面。忽略目录与豁免页面在 [.config/scripts/doc-navigation.toml](../.config/scripts/doc-navigation.toml) 中配置，不需修改脚本；用 `--config <TOML路径>` 指定其他配置。目录和页面路径相对仓库根。发现问题退出 1，否则退出 0；这是按需工具。

PostgreSQL 清理只处理`data/temp/tests/` 中具有 PostgreSQL cluster 标记的孤立测试根，跳过运行中的 PostgreSQL。省略 `-WhatIf` 执行删除；`-MinimumAgeHours <小时>` 限制最小年龄，默认 0。它不清理语料、手写配置或开发实例。

### Longitudinal model research

```text
corepack pnpm research:longitudinal --config data/config/apps/nous.toml --input data/research/longitudinal/plan.json --role journal_synthesis --output data/research/longitudinal/journal-proposal.json
```

`--input` 使用 Kernel `PlanMaintenance` 返回的 ProtoJSON `MaintenancePlan`：包含当前 Subject、exact source revisions、ordered member keys、support/entity/candidate catalogs 和 owner snapshot。输入必须为 `ready` 且不超过 256 KiB；适用的调用方从私有 Kernel 请求取得该快照。`--role` 为 `episode_segmentation`、`journal_synthesis` 或 `memory_consolidation`。配置使用当前 `nous.toml`；gateway credential 来自已设置的配置指定环境变量。`--prompt-root` 默认 `prompts`，`--override-prompt-root` 可选择本地 prompt override。

Runner 通过真实配置角色和 canonical Structured Output 生成一次 proposal，校验 catalog keys，将来源计划、proposal、producer identity 与人工审阅项目写入新的本地文件；已有 output 文件会报错。研究产物放在 ignored `data/research/`。人工审阅使用真实 trace、来源事实和 boundary annotations，分别评估分段边界、Journal point 支持与省略、整合身份和 Schema 泛化。确定性 `smoke:longitudinal` 检查编排与 Authority 合同；质量研究使用这个手动入口。



## 小型认知功能验证

### Core Cognition Semantic Qualification

`corepack pnpm research:core-cognition` 连接已有普通 Core；[方法](../docs/research/core-cognition-semantic.md)与[实际结果](../docs/research/core-cognition-2026-10-07.md)由 research docs 维护。示例：

```text
corepack pnpm research:core-cognition --run-root data/instances/core-cognition-qualification/run --manifest docs/research/corpus/core-cognition/simon.json --phase formation --output data/research/runs/core-cognition-2026-10-07/simon-new --identities data/research/runs/core-cognition-2026-10-07/identities.json --ledger data/research/runs/core-cognition-2026-10-07/gateway-ledger.json --trace-root data/research/runs/core-cognition-2026-10-07/traces --max-model-calls 256 --max-elapsed-ms 600000
```

要求 ignored source cache 已就绪、model traffic 指向 `research:gateway`。`--manifest`、`--output`、`--identities`、`--ledger`、`--trace-root` 与两个正整数预算为必填；除 `acquire-check` 外还要求 `--run-root`。阶段为 `acquire-check / ingest / formation / review-export / retrieval / feedback / revision / all`，默认 `all`；首次 `all` 没有人工接受的 `review.json` 会在 retrieval gate 停止。`--through-checkpoint` 可限制 formation 的来源阶段。恢复使用同 output/manifest；未知非幂等 paid effect 不自动 replay。

`identities.json` 必含 model、prompt、schema、embedding、config、active_config、vault、product_head、kernel_binary；值为 SHA/commit identity，可通过 production `inspect:model-contracts` 和实际配置生成。sealed live phase 额外要求 `--sealed-lock <ignored JSON>`：`calibration_status=PASS`、`calibration_results` 中 Simon/CPython result SHA256、`identities` 共同冻结 keys、`packs[pack_id]` 的 manifest/oracle/source keys；整组值必须与当前执行一致。失败 calibration 不能签发 passing lock。本轮已形成的失败/反馈证据不应再使用 `all` 重跑。

[手工功能语料](../docs/research/corpus/functional/README.md)和[六项 text-only 选择](../docs/research/corpus/text-compatibility-selection.json)定义本轮范围。功能 runner 通过 public Client 连接已经运行的 Core，不负责数据库、Kernel、clock、embedding cache 或 Serving lifecycle。结果写入 ignored `data/research/`。全量外部 benchmark、付费 rerank/provider 比较与 RAGFlow 不在本轮执行。


### 小型认知功能验证

`corepack pnpm research:cognitive-functional --run-root <现有 Core 的 RunRoot> --profiles baseline-rrf,nous-node-potential-v1,vcp-dtsc-v9.2.1-adapter-v1,vcp-rivermemo-v3.1-adapter-v1` 使用 official Client，不启动 PostgreSQL、Kernel 或 Core。它先提交三个手工场景，明确 grant maintenance，检查实际 Tag/Association/identity 形成结果，再 prepare/query；每个 profile 的 semantic representation 必须一致。JSON 输出在 ignored `data/research/cognitive-functional/`。`--max-model-calls`（默认 128）限制 formation/maintenance 模型调用，`--max-elapsed-ms`（默认 60000）取消超时 public requests；`--compat-input` 可指定 ignored raw-text cache，执行六项选择的四 profile smoke。

`--compat-input <本地 raw source JSON>` 可附加六项 selected text-only smoke。该输入仅含 manifest 指定的 source pools，源码 text 必须通过对应 SHA256；query 只有原问题、Memory domain 和 `textOnlyCompatibility=true`，不注入 Entity/Tag/WorkContext。完整来源不进入 tracked corpus。

本地 automatic smoke 使用现有 `longitudinal_smoke` Core/Kernel fixture，`NOUS_FUNCTIONAL_SMOKE=1 cargo test -p nous-kernel --test longitudinal_smoke -- --nocapture` 开启 deterministic model 与 embedding provider，外部 provider/rerank calls 为零。fake vectors 用于验证线路与资产复用，不能说明语义排名质量或算法优胜。一般 public runner 会使用所连接实例的模型配置；自动验收连接 deterministic fixture。

CLI Agent smoke 在同一现有 Core fixture 中执行实际 CLI 子进程，验证 JSON help、`AMBIGUOUS_REFERENCE` 候选、选定 LexicalRef、public prepare/query。OpenCode v2.0.23 已安装，但当前 `opencode models` 返回空列表，本轮未运行外部 OpenCode 模型，也未配置付费 provider。
