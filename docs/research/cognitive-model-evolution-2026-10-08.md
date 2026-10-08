# Cognitive Model & Evolution Research — 2026-10-08

[返回 Research](README.md) · [v2 报告](core-cognition-2026-10-08.md)

本轮研究正在执行。基础代码为当前 `master` `20e9bf87c2d7b0aab177ff94ec13412e29b7a8d0`，长期语义 Authority 为 Vault `5b96c63da34b0a4c697b6961ae10ba6aa4de3ee1`。新实验使用独立身份和 ignored `data/research/runs/cognitive-model-evolution-2026-10-08/`，既有 v1/v2 结果保留。

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

## 当前研究问题

继续比较 formation 的日期、归因、限制与否定保留；正确 Memory 的首次概念形成与错误概念复审；真实 Episode 的 Journal、consolidation/Schema 与跨 Session 分段。长期轨迹检查新来源、候选发现、MaintenanceNeed、固定输入、proposal、owner outcome 与 recall，区分自然 revision、明确 Host correction 和纯模型重新生成。

固定模型输入对照复用 [现有 longitudinal 脚本](../../scripts/research/longitudinal.ts) 的 `--invocation`，从既有 gateway capture 只取实际 user input，通过当前 ModelRuntime、PromptRegistry 和 Structured Contract Registry 生成 proposal。它记录 producer/attempt telemetry，不提交 Authority。真实演化经普通 Core 和 official Client；rerank、enrichment、Context 与 Native/VCP 的判断继续使用实际 cognition。

费用不作为主要选择约束。角色/default 配置及必要的 Prompt、RolePolicy、owner/算法修正由实际观测决定，最终结果在本页与现有 Research 体系提交。
