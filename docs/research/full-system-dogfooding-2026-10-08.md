# Nous Wave Full-System Dogfooding — 2026-10-08

[Research](README.md) · [Agent manual](../agent/NOUSQL.md)

## Objective

让真实 Codex 把 Nous 用于持续研发，跨 Session 和进程恢复工作。实际任务受阻时在语义 owner 修正，再继续原任务。最终质量判断来自任务、认知内容与恢复结果。

## Decisions

- 一个分支 `codex/full-system-dogfooding`，一个 PR，完成后 squash merge。
- Core、Kernel 与官方 Client 继续拥有现有语义。MCP 属于第一方 CLI consumer，只以参数数组执行 CLI，不添加另一套 cognition 或 Runtime。
- 每个使用 Agent 明确绑定独立 consumer state root；Subject cognition 由同一个 Core 管理。MCP 同一连接内串行执行，避免本地 result/selection/receipt 交错。
- CLI 使用代理只读官方手册与帮助，默认文本输出。另一个全新 Codex 通过 stdio MCP 使用同一实例。
- External Resource 模拟仅存在于研究宿主，注入正常 Core；破坏性管理在可丢弃 Subject/实例内执行。
- 原始输入、操作与模型 trace 放在 ignored `data/research/runs/full-system-dogfooding/`；本报告保存可复用判断和实际结果。

## Actual use and decisions

原生 CLI 使用 Agent 创建 Subject `01a11a5c-1512-7db0-9d2d-caae5b8cc094`，以真实 CLI/Agent 文档完成 formation、WorkContext、NousQL、exact show/trace/read、pin/use、Tag revise/split/merge、Association create/revoke。关闭 Session 后继续原任务，两次 Core 重启后保持 WorkContext、来源和 exact anchors。最终任务 checkpoint 为 `01a11a5d-f8b3-7fd2-a9a1-9ad6d8289614` revision 9，使用 Session 已关闭。

独立 Codex CLI 进程通过官方 SDK v2.3.1 stdio MCP 完成第一轮研究，随后完全新建第二个进程，仅靠 Nous 工具恢复原任务，没有读取先前日志、handoff 或实现源码。它恢复 Subject、WorkContext `01a11a5f-aac8-7b22-b8b7-9d5acba51555`、exact Memory 和 admitted 来源，显式 foreground 新 Session 后完成待解决问题。最终 checkpoint revision 11，Session `01a11a69-c72c-77f0-9477-578d275ec4a1` 已关闭；最终 adopted decision 为 `memory_revision:01a11a70-6325-7133-848c-c3e6db454886`。

主研发 Agent 同样把本轮真实缺陷、owner 原因与待完成工作摄入 Nous，并用 query/pin/meaningful use 继续研发。研发 WorkContext 为 `01a11a72-b88b-76c1-9bca-53372f277ec4`；真实源码调查记录形成 `memory_revision:01a11a73-367a-7303-8709-2c8c36c12fb2`。

共享 Subject/独立 Session/private consumer root 已在同时运行的 CLI 与 MCP Agent 上成立。Consumer selected WorkContext 会进入 query preparation，但新 Session 的 foreground binding 初始为空，必须明确激活。跨进程 continuity 足以完成该研究任务，不能据此承诺任意长度或任意领域的无损恢复。

## Defects repaired while continuing the task

- CLI exact parser 漏掉自身返回的 Material kinds，source_region/artifact 被错误送入 LexicalRef resolution。补齐当前 Material refs，并新增复用 Material Authority 的有界 `read`。第二个 Codex 实际读回 7600-byte README 和 1990-byte来源，均 partial=false。
- 失败的新 query 留下旧 result:N，Agent 曾随后读取了无关旧答案。现在执行新 query 前清除旧索引；prepare/inspect 不改变成功结果。
- Retrieval reader 在 Host embedding 尚未存在的 preparation 阶段缓存失败，execution 收到向量仍沿用该失败。RequestEmbedding 现在只缓存成功。回归先复现原失败，再验证补充 Host material 恢复以及并发成功共享。
- 实际 upstream embeddings HTTP400 指明最多10条，研究配置继承64条。配置修正为10后，Agent prepare 实际 committed41/requests2，无degradation；原封重跑先前普通问题，query `01a11a76-bbef-7010-9e1f-8eda0ecbac92` complete，dense ready、11 dense candidates，两个旧 dense 错误消失。
- Client 的通用30秒 RPC deadline 截断120秒 Host maintenance；wire trace确认两次约29秒取消。官方 Client 的 grant deadline 现在按机会预算加回应余量；model-backed operations 使用较长默认deadline，明确 caller timeout 优先。
- 真实120秒机会耗尽被 Core 错归为 internal_failure 并 blocked。现在机会耗尽/取消保留 deferred/pending，让下一 grant 继续 durable work；provider 自身失败仍按 infrastructure retry。Canonical Grant response 明确区分 disabled/no eligible/processed/exhausted，CLI 无需从另一配置读取猜测结果。
- help 补齐 show/retry；query 默认文本保留公开 diagnostics；时间显示ISO、空值省略、basis缩进修正；Association revoke 返回明确的实际操作回执。

## Cognition quality

Mini 将原文“连续多条问题”转述成“跨多个任务”。Pro 仍把 SHOW/READ 职责混淆，也曾把 committed0/requests1 压成“0 requests”。两位真实 Agent 通过 admitted source 原文识别，记录 negative feedback，并形成经核对的 adopted decision。强模型、accepted/grounded 和 immutable identity 均不能代替来源忠实度。

单独 result_refuted 不自动撤回 Memory，后续 query 仍可返回其 accepted revision，且没有明显 refutation标记。这符合 feedback不等于Authority mutation，但对长期 Agent 的操作自然性仍有风险。主研发 Agent 已按原来源在 Memory owner 对旧 continuity object 做明确 correct，保留同一对象、新 revision `01a11a77-2ddc-76a1-b4ee-cc20d0e1c19a`；重复请求返回同一revision，current普通query排除旧版、包含新版，history保留两版。旧WorkContext exact anchor仍可指向旧revision，后续使用者必须明确更新，而非偷偷重绑。

真实 Pro concept maintenance提交 workflow Tag `01a11a71-ab12-7d23-9005-8689b2e38e23` 和来源支持的 attachment/related Association。概念忠实但宽泛，是导航而不是新的决策规则。广泛 shared Subject/WorkContext候选仍会挤占聚焦Tag附件，dense完整不等于正确回答。

## External Resource trajectory

研究-only `LocalDocuments` Adapter 注入同一 production Core bootstrap；没有第二 Runtime或RAGFlow部署。三份真实 owner文档具有固定entry ID、版本、摘要正文，变化通过ignored JSON控制。

真实 search返回StableExternalRef；选择WorkContext文档后materialize→Observation→Pro formation完成，生成 `memory_revision:01a11a6a-76ec-7da3-a13a-f04d8fb6937e`，同operation重放同Occurrence `01a11a6a-5f45-7e11-ba5d-1eb851209b99`。版本改为2后，旧reference新materialization返回stale；新reference成功产生新Occurrence。撤销access后重新获取返回denied，search不再返回该条；删除后获取返回stale/missing。

已admit的本地快照与来源版本仍可精确读取，外部entry删除不等于本地Memory purge。当前外部reacquisition fence有效；不能把此结果提升为“外部权限撤销自动删除所有已摄入认知”。外部实时访问状态与本地admitted snapshot lifecycle仍需明确产品决策。

## Current Work

当前分支的输入、reader、deadline、maintenance和文本改动已通过相应回归、TypeScript全套、static checks和文档导航；这些检查保护修复，实际成果是上面的任务完成与缺陷续接。研究Core在9472、真实gateway在18002，独立Authority roots为 `data/instances/dogfooding/`，凭据只读既有SecretRoot。

当前一个研发分支和 draft [PR 22](https://github.com/Heptalogos-Devs/Nous-Wave/pull/22) 已建立。Node24.21、PostgreSQL18.6 与 FFmpeg9.0.2 本地 runtime packs 和 shipping LLVM-MinGW 已实际取得，第一版 shipping Kernel 已编译；最新 owner/协议变化后仍需重建并完成 source-less Portable 使用，不能把 pack/build 成功视作实际运行成功。

可丢弃操作 Agent 已完成 Memory 历史/rephrase/accessibility/withdraw/reaccept/suppress/restore、Schema 来源失效与再验证、双 Session foreground/pause/resume、UseEvent replay。原管理任务仍被 Schema lifecycle API 缺失和未配置自身 consumer policy 阻断，待 owner 修复后继续临时 cognition 的 purge 与清理。长期实例继续实际 Pro/Lite 维护、media、Serving 恢复与 Portable；本轮最新 query 出现 dense ready/candidates11 但同时保留 `Host did not supply compatible embedding material` degradation，正在定位两阶段 preparation/execution 的状态归属。

## Stable references in actual Agent use

用户指出正常界面暴露过多 UUID，实际调查确认已有 LexicalRef 未被创建/身份/上下文/Material 输出一致采用，输入也有只认 canonical UUID 的分支。修复复用同一 Authority Directory：Subject、Session、WorkContext、Association 以及当前 cognition/Material kinds 返回持久地址；normal text 将 evidence oneof locator 一并转换，后续命令和全局选择参数解析这些引用。地址获取不改写已有名称/alias，不更改 source text，不截断 UUID。机器 JSON、developer 和 raw 诊断保留 canonical identities。正常 query 的来源提示现包含 `read`，query-linked feedback 使用 consumer-local `query:last`，result:N 自动关联当前 query；失败新 query 同时清除两者。

隔离原生 CLI 使用者实际以 `sub:colossal-attach-imperial-step`、`ctx:camper-probably-hydration-chief`、`session:preview-lair-penny-pupil`、`memrev:dealing-turmoil-clothing-ivy`、`obs:roamer-pardon-scabby-cosigner` 和 `src:outbreak-approval-chant-quail` 完成恢复/show/trace/read，换全新 consumer root 后同一 Subject 的词汇引用仍有效。来源读回1967bytes/partialfalse，完整 admitted README7600bytes/partialfalse。Tag 原名称/旧alias保持BOUND。相同工作上下文已保存词汇 checkpoint revision10，使用者Session关闭。

第三个全新 Codex 进程只通过实际 stdio MCP，以返回的 sub/ctx/session/result:N/memrev/obs 引用完成相同研究续接与来源读取；query 更新后仍用同一 exact memrev 读取，两个名称及稳定 Tag 地址一致。原 MCP WorkContext 保存checkpoint revision13，两个旧 exact anchors保留、追加本次实读来源支持的exact Memory，Session关闭。两个使用者均未输入 UUID 参数、未读本地consumer状态/旧日志/实现源码，没有回退 UUID。历史 Memory/source 中已有 UUID 保持原文，不能据此改写已存认知。原始新操作证据在 ignored `cli-user/operations-lexical.md` 与 `codex-mcp/lexical-session.jsonl`。

主研发 Agent 随后实际 query 后用 `use memrev:product-showcase-shortcut-fiction --query-id query:last --kind referenced` 返回acceptedCount1，再将词汇化 Objective/Decisions/Current Work 保存到 `ctx:outer-zodiac-reps-delicate` revision3。当前 Goal 继续，尚未交付全系统完成或合并结论。
