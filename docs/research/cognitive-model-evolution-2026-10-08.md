# Cognitive Model & Evolution Research — 2026-10-08

[返回 Research](README.md) · [v2 报告](core-cognition-2026-10-08.md) · [本轮稳定 metadata](corpus/core-cognition/results-evolution-2026-10-08.json)

本轮真实研究已完成。基础代码为 fetch 后的 `master` `20e9bf87c2d7b0aab177ff94ec13412e29b7a8d0`，长期语义 Authority 为 Vault `5b96c63da34b0a4c697b6961ae10ba6aa4de3ee1`。新实验使用独立身份和 ignored `data/research/runs/cognitive-model-evolution-2026-10-08/`，既有 v1/v2 结果保留。执行与推断依据 AntiGPT；没有第二 Runtime、Benchmark 平台或全参数矩阵。

结论：更强角色模型和清晰 Prompt 能改善来源保真，并通过既有长期维护自然修订错误 Memory；它们仍会误用时间范围、把模型局部视图当作完整经验、过度修订已有 cognition。当前采用 Pro minimal formation、Pro high concepts、Lite high Journal／consolidation、Pro default segmentation；查询默认直接检索，按需 enrichment／rerank。下一研发重点是继续提高维护输入的完整性语义和修订节制，而不是扩建 Runtime。

## 首批实际模型观测

New API 的启用渠道包含 `doubao-seed-2.1-lite` 与 `doubao-seed-2.1-pro`，原 Nous token 的三个模型 whitelist 未包含它们。已扩展到这两个模型，`/v1/models` 实际列出五项。Lite 的首次调用被本地 token 预扣费额度拒绝，账户额度充足；解除该 token 的额度限制后真实调用成功。失败仍计入 gateway attempt，不能算作模型能力失败。

同一 PEP 703 首单元、原 Prompt 和结构化合同、32768 output budget 的一次 default 对照中：Mini 约 9 秒、Lite 约 49 秒、Pro 约 15 秒。Lite 保留 Created 与 Accepted 的区别、3.13 目标版本与 Council provisos；Pro 也保留这些条件，但把作者 Sam Gross 错写为 Sam Gong。Mini 此次没有复制此前错误接受日期，但省略创建日期和目标版本。单次结果不能支持稳定性判断，后续使用少量重复与 reasoning 对照。

实际回包 model identifiers 为 Mini `doubao-seed-2-0-mini-260428`、Lite/Pro `doubao-seed-2-1-*-260915`；这些是 provider 返回的身份，没有独立 weights 证明。网关给所有模型的通用 endpoint labels 也不是能力实测。速度、上下文与 reasoning 结论以本轮实际 input、wire controls 和 usage 为准。

## Prompt 与 SDK 接线复核

针对 Pro 将 Sam Gross 写为 Sam Gong 的观测，检查了原始 request/response、实际 SDK 调用代码、安装的 SDK message converter 与 New API channel overrides。出错请求是一个独立 system Markdown 指令和一个 user JSON evidence envelope；原文只含 Sam Gross，没有 Sam Gong。错误已存在原始回包。SDK default 重复请求的完整 JSON payload 与出错探测相同；high 请求唯一差异为 reasoning_effort。New API 未配置 system prompt 覆盖，也没有 param/header overrides。没有发现角色混装、证据提升为 system 或 SDK 改写作者的证据。

安装的 `ai@7.0.102` 将 `system` 保留为 `instructions` 的弃用别名，实际实现为 `instructions = system`。Core 已使用推荐的 `instructions` 参数；provider converter 仍负责将独立指令映射为模型支持的 system/developer 角色，user evidence 不变。以安装源码与实际 wire 为当前依据；[SDK generateText 文档](https://ai-sdk.dev/docs/reference/ai-sdk-core/generate-text)也使用 instructions。角色优先级表达指令的地位，structured output 约束输出形状，两者均不能单独证明模型生成的事实正确。

新接口的实际 Pro 调用耗时约 27 秒、4605 tokens，完整 wire JSON 与上一轮澄清 Prompt 的 system 别名调用完全相同，角色仍为 system/user；作者、Created 日期与接受条件均正确。接口迁移没有改变 provider message roles 或输入内容。

旧 formation Prompt 已有 Markdown，但事实保真、时间与压缩规则挤在两段中；concept maintenance/query enrichment 没有章节层次。已按任务输入、来源保真、时间/条件、输出和 selector 边界重新组织，明确专名准确复制、日期归属与重要限制。Journal 同时明确 basis[].key 的 selector namespace。指令继续进入 system，材料进入 user；不将 developer 当作适用于所有兼容 provider 的替代。

固定 PEP 输入、Pro default、相同 schema/output budget 的旧 Prompt 两次中一次改错姓名，另一次正确；澄清后的两次均保留 Sam Gross、创建日期和接受/提案阶段条件。原 Prompt high 一次也正确。样本支持采用更清晰的任务规则，但不能据此认定排版是该错误的唯一原因，也不能将两次正确当作可靠性估计。完整输入/Prompt hashes 与五次输出保存在 ignored prompt-role-audit.json。

六个研究角色的 Prompt 均已复核。Formation、Concept、Journal 明确专名、条件、时间归属与 selector namespace；Query enrichment 使用任务、已有概念选择与权限边界章节。Episode 原有顺序、连续性和 Session 边界规则清楚，Consolidation 已明确继续同一 claim、独立来源、revision dependency cycle 与 skip 语义。没有将本次正确答案写进 Prompt。

后续相同旧输入的纯模型重新生成中，Pro high 首次 Concept Tag 正确保留 PEP 703 作者、3.13 目标及 Council provisos。Lite high CPython Journal 生成 13 个 points，所有 basisKeys 均属于实际输入的 basis[].key，保留 experimental/optional/default 阶段区别；这是合同及输出复核结果，尚不是 Domain Owner 提交或自然演化修复的证据。

Pro default Simon Journal 正确区分 2020 年 4 月开始 TIL 与 2022 年文章报告的 346 篇，8 个 points 的 support selectors 全部合法。该调用约 261 秒、19498 tokens；比单次 formation 慢得多，角色预算不能直接套用 formation 的延迟。Lite high 在约 76512 input tokens 的 CPython consolidation 输入上成功提出限定于 PEP 703/779 的阶段推出 Schema，约 138 秒、85972 total tokens；该 context 是实际处理长度，不是模型最大窗口证明。

## 实际 owner 修正

复现既有 CPython Subject 的 Concept Maintenance producer 读取返回 NotFound。公开 get_producer 的 Subject 引用检查包含 Memory/Episode/Journal/Schema 与 derived representation，遗漏 Tag revision 和 AssociationEvidence。引用范围已由 Material 与 Memory owner 各自负责，transport 只组合结果并读取签名。保留 historical revisions 与 revoked association 的实际 producer 追溯，不改变 cognition 内容。

真实 PostgreSQL concept maintenance 测试验证 Tag-only、Association-only producer 可读且跨 Subject 返回 NotFound；相关 Rust clippy 通过。重启普通 Core 后，official Client 对先前同一个 producer 的读取成功，返回原 Mini/profile/Prompt/config signature。旧 v2 Authority 未被改写。研究 runner 的 maintenance grant 采用当前 API 支持的 300 秒上限，并留出响应传输时间，以容纳此次实测较慢的强模型角色。

## 新 Subject 的持续经历

独立 CPython Subject `0ce739c0-66b4-5a0a-a853-2300400bb307` 的起点显式重放 v2 Mini 的错误 Memory（accepted on 09-Jan-2023）和错误 Tag；两者由 Host 提交，绑定本 Subject 新观察的真实 PEP 单元，原 trace 3/67 身份保留，没有将重放伪装为新的 Mini 推理。首次普通 maintenance 的 Pro 修订了 Tag，准确区分 Created 与接受日期；该阶段原 Memory 仍为 revision 1。

随后两个真实 Council 来源进入第二 Session：`discuss.python.org` post 107496/version 1（2023-07-28 notice）与 post 123112/version 2（2023-10-24 acceptance）。原文、发布时间、post/version 与 hashes 在生成前冻结，模型未收到预写的日期答案。Pro formation 保留通知、接受说明及渐进、实验性、可逆的条件。第三 Session 读取冻结的 PEP 779。新 evidence 与 concept maintenance 提交后，旧 Memory 仍存在；直到 settled Journal 进入 consolidation，候选目录把错误 Memory 的 exact revision／epoch、正文和 eligible basis 提供给 Lite high。

实际 trace 95 的 Lite high 提出 revise_memory／intent=correct，而非另建 Memory。Owner 提交同一 Memory `01a11994-0685-7561-9425-2acd05491342` 的 revision 2 `01a119c7-3122-7b20-895f-e6adf3c11ce7`，支持是本 Subject 的 PEP、7 月 notice、10 月 acceptance 三个 exact occurrences。正文区分创建日期、意向通知与正式接受公告，保留 rollout／rollback 条件。没有 Host 更正文案或负反馈提示。公开 Client 的当前 Tag recall 返回新 head、排除旧 head，修订前 as-of 返回旧 head、没有未来修订；history API 保留两版。新 evidence 并未即时修订 Memory，但现有长期维护机制能够在充分来源和可执行强模型下完成 continuing correction。

同一次自然修订把复合命题的 validTime 起点设为 2023-01-09，这不能由 PEP 创建日期证明。Host 随后仅把 validTime 改为 unknown，提交 revision 3 `01a119d6-f451-7a50-95c2-5e07e8885919`，正文、来源、对象身份保留，没有模型调用。这是显式 Host 更正时间适用范围，不能计作模型自发修复。

独立 Simon Subject `9ffac9c0-6afa-565d-a14d-f10fac39b976` 在三个实际 Session 中读取 TIL advice、link blog 与 beats，三个 Memory 由真实 Pro 调用形成。本轮使用 SystemCognitiveClock 和 reference hard idle 1800 秒、settle delay 300 秒，未注入时间。两个 Subject 自然提交的 Episode 均保留跨 Session 连续性；Pro 对四个 CPython／三个 Simon 有序成员的 semantic review 均返回有内容依据的 no_change，没有把 Session 或文档自动当作边界。

Lite high 的两个 Journal 均经 owner 提交，保留源文作者归属、TIL 数量／日期关系及 CPython 的意向／接受／阶段条件。Journal 有一处把输入 catalog 的 truncation 概括成 Subject 没有观察完整材料；Prompt 已明确 bounded model view 与完整 Subject experience 的区别。

Simon Episode consolidation 实际 create_schema，公开 schema query／exact revision／producer read 均读到相同身份及三个来源链接。Schema 有作者和适用范围边界，未被当作普遍法则；但 validTime 错用支持文章的 2022–2026 日期区间。新 Prompt 固定输入复测返回 unknown validTime（纯重新生成），Host 随后修订同一 Schema、复制三个原来源链接，将当前 validity 设为 unknown。旧 exact revision 保留原时间区间。Journal consolidation 的 trace 96 看到了既有 Schema，未复制 Schema／Memory，提交 elaborates 与 temporal_successor 两条关系。

强模型长程工作需要独立机会：CPython 首次 Episode consolidation 的共享 grant 剩余约 146 秒，调用在 elapsed cap 处被中断，trace 93 保留 incomplete response，不能归为模型不可用。给单次调用完整 budget 后，Journal consolidation 约 235 秒提交自然 correction；Simon 后续整合约 264 秒提交关系。研究 runner 默认一个模型调用／grant，避免在前序耗时后压缩下一调用预算。

## 已执行的检索对照

Qwen `qwen3.7-text-rerank` 对 v2 四个不可变候选池的真实重排：Simon practice 的目标仍第 1；CPython 3.13 的两个目标来源从 1/3 变为 1/4；Rust dyn reference 从第 2 降至第 5；Kafka 3.6 从第 2 升至第 1。候选池、exact revision、text hashes、原 query identity、scores 与 producer 均保留。该调用只改变排序，不能补充池外来源或修正候选中的错误命题。当前选择按需 rerank，不设为所有查询的 required。

新 Subject 的短跟问、WorkContext text／Tag／Entity、显式指称与 model enrichment 已执行。小池内都能覆盖现有 Memory，排序却不同：CPython 的 Tag anchor 把仍含错误日期的旧 Memory 推到第一，显式 query 把 Council acceptance 说明排在第一；Simon Tag anchor 偏向早期 advice，显式 query 偏向后期 beats。因此高 Source Recall 并不说明当前命题正确，Context／Tag 也不是无条件收益。

首次 Serving preparation 被上游拒绝：embedding endpoint 最多 10 条，配置却允许一次 14 条。失败保留，修正本地 max_batch_size=10 并重启后，CPython 14、Simon 11 份材料成功提交；后续比较使用未降级的 lexical/dense 条件。Query Pro 曾把必需字段 novel_concepts 写成 novel_conceptions，被 SDK 合同拒绝；已明确精确字段名。另一输出把 beats 解释为 recurring thematic areas，而 supplied catalog 并未建立这种意义；新 Prompt 要求 novel hypotheses 使用短检索概念或问题，保留未消解词义。相同输入／最终 Prompt 的 Lite 与 Pro 对照均通过合同，Pro 用约 7–10 秒，Lite 约 45–52 秒。它们是 query-local hypotheses，没有写入 Tag Authority。

Native/VCP 在小池内未增加 baseline 未获的有用 Memory。Simon Native 访问 5 个节点、激活 7 条边、最大 2 hops 后报告截断，实际可见 readout 是已有 Tag attachment，尚无新增关联收益。CPython Native 首次报告 generation/profile mismatch；查明 baseline prewarming 将 Native 图错误标成 baseline，而缓存键有意复用同一 Native asset。Retrieval builder 已统一图与 metadata 的 Native asset identity，并提升可重建 artifact revision；baseline／Native 仍复用同一资产，Runtime 查询 profile 保持独立。真实 PostgreSQL query correctness 场景验证 baseline prewarm 和反复 VCP／Native 切换，19 个相关测试通过；修复后公开 Core 两组都执行 Native，lexical/dense ready，剩余诊断为传播 work-budget 截断。

复核保留的 v2 witness：Simon writing-recurrence 经 person Entity 连到另一 blogging Memory；CPython native-boundary 经 project Entity 连到 reference-counting、3.13 howto、GC 等不同主题 Memory，均 complete=false。这些路径展示 activated association 的实际到达，usefulness 按目标命题覆盖判断，route 的合同是 witnessed path。广泛 Entity hub 连接的不同主题需要逐项核对；本轮观察到传播截断与命题错配，选择 direct baseline 作为当前默认。

## 当前配置决定

本轮采用以下配置作为当前部署起点。所有 structured roles 使用 32768 output budget、300 秒执行上限；模型名称是本机 New API 可用资源，选择依据实际输入、source fidelity 和 owner outcome，不是普遍模型排名。ModelRole 的语义与 ExecutionProfile 的 reasoning／预算分别保存。

| Role | 模型与 reasoning | 选择依据 |
| --- | --- | --- |
| Memory Formation | Pro / minimal | 同一澄清 Prompt 的 PEP、Rust dyn、Kafka 3.6 均保留日期归属、Self:Sized 例外、Early Access／禁止 downgrade 等关键条件，约 12／16／21 秒 |
| Concept Maintenance | Pro / high | high 首次 Tag 保留关键条件；真实 Episode／新 evidence 维护创建 scoped concepts，已有错误 Tag 的 Pro default 修订成功 |
| Journal Synthesis | Lite / high | CPython 的 support keys 与阶段叙述正确；原强模型 NOT_RUN 已补齐，实际约 125 秒 |
| Memory Consolidation | Lite / high | 自然 continuing Memory correction、Schema 创建与后续关系提交均成功；时间适用范围仍需 source-faithful review，单次可达 235–264 秒 |
| Episode Segmentation | Pro / provider-default | 两个真实多 Session neighborhood 合法 no_change，按经历用途保持连续，未机械按 Session／文档切分 |
| Query Concept Enrichment | Pro / provider-default，默认 off | 同一最终 Prompt 的输出有界、合法，未观察小池新增检索收益；显式指称／WorkContext 是优先输入 |
| Query Rerank | Qwen rerank，按需 | Kafka 排序改善、Rust 排序退化，避免无条件 required |

费用不是上述选择的主要约束。实际调用包含 schema 拒绝、HTTP 失败及配置失败；usage 与 latency 不只统计成功产物，最终保留 New API credit 扣费和 supplier bill 的可知范围。

## Skip、输入完整性与技术决定

补充对照采用已读回的真实当前 Schema 和两条现有关系，作为明确标记的纯模型 current-state control，未把预期 skip 或更正文案写进输入。Lite high 首次提出重写三项 Memory／Schema，并把局部 source view 缺少的信息描述为完整 cognition 的问题；这些提案未提交。逐字符核对表明本次 candidate text 完整，截断在 member/source view，不能把这次行为归因于 candidate clipping。

修订 Prompt 后，同一 current-state control 加上完整性标记的 Lite high 返回 explicit skip，理由是三项 grounded Memory、现有 synthesized Schema 和两条关系已经覆盖来源，重复创建／改写无新价值。调用约 88 秒、22870 tokens。当前数据的 candidate 标记均为 false；该对照同时改变了 Prompt 和显式标记，结果支持更清晰的输入语义，不分离两者的因果贡献。

Kernel 在配置预算真的截短 candidate 时新增 text_truncated，Protobuf／Rust／TypeScript bindings 同步更新，真实数据库的 64-character bounded scenario 验证标记。Prompt 明确 bounded view 的遗漏不等于原来源／存储 cognition 缺失、不能仅为缩写或未知细节做 correction。Journal 同样区分局部输入与 Subject 完整经验，consolidation 明确 publication span 与 world-valid applicability 不同。这些规则服务一般维护，不含本轮案例答案。

下一项最有价值的研发是让持续 consolidation 更稳定地识别“有新支持／反证的 continuing change”与“完整 cognition 在局部视图中看起来不完整”，并提供有界的现有关系上下文。本轮 current-state control 的 relations 由 public Client 加入，生产 consolidation catalog 尚不包含它们；已有 owner 防止重复关系存储，输入层仍可减少无意义重复提案。该工作沿现有 Memory／Schema planner 与 owner 推进。Native/VCP 当前没有支持全局启用的增益证据，保留 direct baseline 与按需探索。

## 实际调用与费用

共享 research gateway 共 98 attempts，完整接收 94 个 HTTP 200 responses；另有 whitelist/token quota 403、上游 500、embedding batch 400 和一次 elapsed-cap abort。Schema 拒绝仍计入已付费的 200 响应。已知 gateway usage 为 635974 tokens；New API token 1 的对应时间窗账单为 669716 tokens，包含一个未完整收到的额外 Lite completion，其差额 33742 与中断调用相符，但未以 request ID 独立关联。

New API 实际扣除 22262637 quota units；公开 status 的 quota_per_unit=500000、usd_exchange_rate=6.9，对应平台 credit 44.525274 USD、界面折算 307.2243906 CNY。它不是供应商现金账单，supplier bill 未提供。本轮原始 HTTP、失败、模型返回身份、Prompt／schema／config hashes、调用 latency／usage、source IDs、当前与历史读回均保留；不把各 Subject 的共享账单重复求和。

固定模型输入对照复用 [现有 longitudinal 脚本](../../scripts/research/longitudinal.ts) 的 `--invocation`，从既有 gateway capture 只取实际 user input，通过当前 ModelRuntime、PromptRegistry 和 Structured Contract Registry 生成 proposal。它记录 producer/attempt telemetry，不提交 Authority。真实演化经普通 Core 和 official Client；rerank、enrichment、Context 与 Native/VCP 的判断继续使用实际 cognition。

## 验证与交付

相关数据库 scenarios、官方 smoke 与完整 just check 在本机执行；截断 metadata 的新合同已 regenerate、typecheck、proto lint，并再次进入完整检查。PR #21 始终在同一 `codex/cognitive-model-evolution` 分支交付。最终 integration 状态以 GitHub PR／CI 为准；本地检查不作为独立评审或其他平台运行证据。
