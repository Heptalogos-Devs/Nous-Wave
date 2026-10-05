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

Research 使用真实模型，必须先准备语料、运行实例和分发的 Client 模块。方法与数据说明见 [Research](../docs/research/README.md)。

```text
corepack pnpm research:gateway --ledger data/research/runs/run-ledger.json --max-calls 1000
```

Gateway 默认转发 `http://127.0.0.1:3000/v1`，监听端口 18000；用 `--upstream`、`--port` 覆盖。`--ledger` 与 `--max-calls` 必填，显式累计调用上限允许 1–100000，预算计入失败和重试，持久化在 ledger；同路径旁保存 telemetry。将实例模型 endpoint 配为该 gateway 后再运行实验。 可加 `--trace-root data/research/runs/<run>/traces` 保存每次 attempt 的 `meta.json` 和经过敏感信息清除的 request/response。request capture 上限为 96 MiB，response 为 1 MiB；超限只保存大小、digest 和 truncation 状态。大型 data/base64 媒体与 multipart file 保存 media type、byte count 与 SHA-256 描述符；multipart 保留 model/language 等普通字段，binary 不作为文本落盘。Authorization、cookie 和 credential header 不落盘，已知凭据回显也在写盘前清除。trace 文件使用 0600 权限。

```text
corepack pnpm research:retrieval-live import --run-root <实例run目录> --client-module <分发client模块> --track controlled
corepack pnpm research:retrieval-live run --run-root <实例run目录> --client-module <分发client模块> --track controlled --variant baseline --output <结果.json>
corepack pnpm research:media-live --run-root <实例run目录> --client-module <分发client模块> --unit nasa-menon-conversation --strategy direct_structured --derive-only
```

Retrieval 子命令为 `import`、`run`、`audit-formation`；track 为 `controlled` 或 `end-to-end`，variant 为 `baseline`、`model-rerank`、`wave`、`combined`。默认读取 `docs/research/corpus/manifest.json`、`queries.json`、`data/research/corpus/unit-texts.json`，状态位于 `data/research/runs/corpus-state.json`。可用 `--manifest`、`--queries`、`--texts`、`--state`、`--output` 改路径；`--limit` 默认 0 表示全部，`--concurrency` 默认 4。导入可用 `--embedding-batch`（默认 64）、`--embedding-interval-ms`（默认 0）控制批次。

Media 默认读取 `docs/research/corpus/media.json` 与 `data/research/corpus/raw`，将处理状态写到 `data/research/runs/media-state.json`；用 `--manifest`、`--raw-root`、`--state` 覆盖。状态文件用于继续已有实验，不会从头重复已完成操作。

媒体 runner 的 `--unit`、`--strategy` 可重复指定，`--derive-only` 只验证派生与来源图，`--skip-retrieval` 执行派生和 formation、暂不准备 embedding/query。`--allow-degradation <code>` 可重复声明实验预期的降级（例如 frames-only 的 `video_audio_not_interpreted`），结果仍保存全部 degradation；其他降级继续使 pipeline 失败。Receipt 与完整 manifest digest 绑定；更换语料用新的 state。文本 runner 支持 manifest 中显式 author/project entities、出版日期、rights 与 extraction；`--skip-embeddings` 用于小组 formation baseline，`--session` 把观察归入一个实际 Session 并在所选单元成功后关闭。人工评阅应核对实际输出，pattern 命中只覆盖预先声明的事实。Manifest 的 `oracle_patterns_by_representation_kind` 可为 Transcript 等表示声明与其职责相符的事实 oracle，未声明时使用共同 `oracle_patterns`。

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

### 检索评测指标

`research:retrieval-live` 使用 [retrieval-metrics.ts](research/retrieval-metrics.ts) 计算多目标 Recall@1/5/10、hit rate、MRR、nDCG@5/10、Average Precision 与 source-set recall。Recall 的分母为独立 oracle 中全部正相关单位；旧报告将任一相关项命中称为 Recall，需重新评分后才能与新报告比较。Runner 的现有 binary oracle 映射 grade=1；认知 benchmark 可提供 grade=-1/0/1/2/3 及 reason/source/harmfulKind。

正 nDCG 只使用正 grade 的 `2^grade-1` gain；harmful count 与按 rank 折扣的 exposure 单独输出。重复结果保留排名位置但不重复获得 gain/recall；未审定结果单独计数，不自动标为有害。负例-only oracle 没有正相关分母，Recall/nDCG/AP 返回 null。不同类别的结果应分别报告，不能把这个函数输出解释为统一认知总分。

### Longitudinal model research

```text
corepack pnpm research:longitudinal --config data/config/apps/nous.toml --input data/research/longitudinal/plan.json --role journal_synthesis --output data/research/longitudinal/journal-proposal.json
```

`--input` 使用 Kernel `PlanMaintenance` 返回的 ProtoJSON `MaintenancePlan`：包含当前 Subject、exact source revisions、ordered member keys、support/entity/candidate catalogs 和 owner snapshot。输入必须为 `ready` 且不超过 256 KiB；适用的调用方从私有 Kernel 请求取得该快照。`--role` 为 `episode_segmentation`、`journal_synthesis` 或 `memory_consolidation`。配置使用当前 `nous.toml`；gateway credential 来自已设置的配置指定环境变量。`--prompt-root` 默认 `prompts`，`--override-prompt-root` 可选择本地 prompt override。

Runner 通过真实配置角色和 canonical Structured Output 生成一次 proposal，校验 catalog keys，将来源计划、proposal、producer identity 与人工审阅项目写入新的本地文件；已有 output 文件会报错。研究产物放在 ignored `data/research/`。人工审阅使用真实 trace、来源事实和 boundary annotations，分别评估分段边界、Journal point 支持与省略、整合身份和 Schema 泛化。确定性 `smoke:longitudinal` 检查编排与 Authority 合同；质量研究使用这个手动入口。

### Cognitive recall research import

[CC0 corpus importer](../apps/nous-kernel/examples/cognitive-import.rs) 使用正常 Rust semantic owners 和显式 research CognitiveClock 导入独立 PostgreSQL，保存 event/revision 映射。用法及时间/关系映射限制见 [Cognitive Corpus](../docs/research/corpus/cognitive/README.md#研究-importer)。该入口不调用付费模型；embedding 和完整 query/profile runner 另行执行。

### Cognitive embedding material

```text
corepack pnpm exec tsx scripts/research/cognitive-embedding.ts --config <nous.toml> --output data/research/<run>/embedding-config.json
cargo run -p nous-kernel --example cognitive-import -- docs/research/corpus/cognitive data/research/<run> data/research/<run>/embedding-config.json
corepack pnpm exec tsx scripts/research/cognitive-embedding.ts --config <nous.toml> --output data/research/<run>/embedding-config.json --needs data/research/<run>/embedding-needs.json --vectors data/research/<run>/embedding-vectors.json --locator <bootstrap.toml>
cargo run -p nous-kernel --example cognitive-import -- docs/research/corpus/cognitive data/research/<run> data/research/<run>/embedding-config.json data/research/<run>/embedding-vectors.json
```

配置导出复用 production `resolvedEmbedding`，不维护第二份 identity。实际生成复用 `ModelInvocations.embeddingBatch`，凭据通过 locator/SecretRoot 的既有加载方式读取。文本按 SHA-256 去重，成功批次保存缓存，空间/producer 不一致时拒绝复用。批次间隔默认 6000ms，`--interval-ms` 可按实际 provider 限流调整；失败不隐藏，重跑只处理未缓存文本。

Rust harness 验证材料身份后使用普通 `ServingService.commit_embedding`，导出下一轮未就绪 needs。每 Subject 页最多 256，满页会报告 bounded；生成/提交/再导出直到 needs 为空才完成。这个材料阶段尚未运行查询或生成 benchmark 分数。

### Cognitive benchmark scoring

`corepack pnpm exec tsx scripts/research/cognitive-score.ts --input <benchmark.jsonl> --output data/research/<run>/metrics.json` 将当前 CC0 corpus 的 sparse oracle 展开，并复用 retrieval metrics owner 按类别/profile 汇总。默认语料路径 `docs/research/corpus/cognitive`。报告只证明输入行已测量，不凭结果行数确认整个 suite 完成；模型 rerank、其他 benchmark tracks 与消融仍需对应运行。

## External memory suites

```text
corepack pnpm research:retrieval-live prepare --suite longmemeval-s --raw-file data/research/external/longmemeval/longmemeval_s_cleaned.json --prepared-root data/research/external/longmemeval/prepared
corepack pnpm research:retrieval-live prepare --suite locomo --raw-file data/research/external/locomo/locomo10.json --prepared-root data/research/external/locomo/prepared
cargo run -p nous-kernel --example cognitive-import -- data/research/external/locomo/prepared data/research/<external-import-run> data/research/<material-run>/embedding-config.json
```

原文件 URL 与 frozen SHA-256 由 [source manifest](../docs/research/corpus/external-memory-sources.json) 记录，需先下载到上述 ignored raw path。`prepare` 验证摘要后生成供同一个 clock-injected semantic-owner harness 消费的 scenarios/queries/manifest，以及 annotation audit；不访问付费模型。Importer 根据 manifest 加载 scenario，保留各 item/会话隔离，外部 suite 使用官方 session 时间及明确的 research availability 映射。正常导入仅导出 bounded embedding needs；外部 full-run、public-host rerank track 仍需接线，`--suite` 非 legacy 的 import/run 暂时明确报错。

Importer 的 event/revision receipts 使用增量 JSONL journal，Subject/Session 元数据仍以 atomic checkpoint 保存。旧完整 JSON checkpoint 首次 reopen 时原子迁移；写入中断的末尾残行会截去，并按普通 owner operation ID 恢复该事件。已有已提交 revision 必须再次回读核对时间，不能用 checkpoint 代替 Authority。

Cognitive scorer 同时保存 association target recall、chain coverage、causal precursor recall 与 ordered-chain score（@1/5/10）。Chain 从冻结的 directed paths 取得，排除 cue 根节点；ordered-chain 分母为所有前驱对，按 cue→precursor 的路径顺序检查检索排名，未返回节点不获 pair credit。它不测生成叙事顺序。按 category/profile 汇总 harmful kinds、activated edges、seed count、observed max hop、已测 discarded mass 和对 baseline 的 Recall/nDCG/intrusion 差值；VCP 未测 mass 保持 null。没有 paths 的 query 相应指标为 null。

真实向量 cache 使用 `{config, vector_files}` manifest，每个 batch 写入相邻 `.parts/` 目录，随后原子发布 index。Rust importer/benchmark reader 同时支持现有 `{config, vectors}` cache 和新分片格式。生成器首次 reopen 旧 flat cache 会将其转为一个 legacy shard，后续批次不重写全部已生成向量。分片保留原 text/space/producer/vector，不改变 embedding 合同；相同文本通过 SHA-256 去重。已有正在运行的旧生成进程继续写旧格式，需等该 writer 结束后迁移。

## Cognitive paired rerank

```text
NOUS_RESEARCH_RERANK_CONFIG=<research-nous.toml> NOUS_RESEARCH_RERANK_LOCATOR=<bootstrap.toml> cargo run -p nous-kernel --example cognitive-import -- docs/research/corpus/cognitive data/research/<fresh-paired-run> data/research/<material-run>/embedding-config.json data/research/<material-run>/embedding-vectors.json benchmark
```

这两个变量显式启用付费 `query_rerank` track。例子只通过 stdio 调用 [production invocation adapter](research/cognitive-rerank.ts)，adapter 使用既有 credentials loader、`ModelInvocations.rerank` 和 research gateway。每条 query 的 baseline/native 使用相同已验证候选池（pool ceiling=64、正常 validation budget 仍生效），先保存无 rerank 的前十，再由原 Runtime retain/finalize workflow 对真实模型排序进行最终 Authority 复核。模型不能增加候选，少于两个可排序候选时跳过调用。

每个 query 共六个 variant：baseline、baseline+model-rerank、native、native+model-rerank、DTSC、V3。记录固定 rerank binding/profile/config digest、producer、调用数、失败、真实 provider latency、6000ms research throttle 以及 rerank+final-validation 总时长；后两者当前没有逐段完全分开。Rerank operational failure 保留已验证 baseline、添加 `query_rerank_unavailable`，不会当成成功排序。这个受控 Kernel/production-model track 还需与 public Core end-to-end track 分开。

LoCoMo embeddings / CC0 rerank 阶段使用累计上限 2000；全量 LongMemEval 阶段使用 30000（246457 个唯一文本，batch=10，另保留既有累计调用和后续比较容量）。上限按实际阶段输入规模声明，ledger 保留累计计数。Gateway 切换在模型调用间完成，确认新 gateway ready 后继续。

Hard Text 原始来源冻结：`python3 scripts/research/retrieval/hard-text.py`，源目录和 unit texts 位于 ignored `data/research/corpus/hard-text/`，摘要/locator 位于 [hard-text manifest](../docs/research/corpus/hard-text.json)。该命令只准备 source，query 审计和实际检索使用对应研究流程。

## Hard Text IR preparation

```text
corepack pnpm exec tsx scripts/research/retrieval.ts prepare --suite hard-text --raw-file docs/research/corpus/hard-text.json --queries docs/research/corpus/hard-text-queries.json --texts data/research/corpus/hard-text/unit-texts.json --prepared-root data/research/external/hard-text/prepared
```

同一 CLI 使用 source/unit SHA-256 复核文本，生成 manifest/scenarios/queries 供已有 `cognitive-import` 使用。准备本身不调用模型，不改变 oracle 审计状态。第三方正文仅写入 ignored prepared root。所有版本/项目共处一个 Subject，每篇 source 为一个 Session；Tag 来自来源版本/作者，query cue 来自问题 literal surface。Hard Text 的 `explicit_qrels_unjudged_others` policy 保留未审定候选；普通 closed oracle suites 保持原语义。来源文档用 `source_set_unit=document_session` 分组，避免把每个段落误称一份独立 source。
