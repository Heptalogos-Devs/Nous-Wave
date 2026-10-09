# 全仓重整实际观察（2026-10-09）

本报告保存本轮实际复现与结果，后续追加同一任务的新观察。任务范围包括 owner 结构、配置身份、执行、consumer、Serving、Portable 与成果保全；当前观察不证明整项重整已经完成。

## CLI 正文与明确拒绝

基线 `09d67d4452eead78346877980f804ac513675d8d`，Windows 上使用当前源码 CLI environment 连接已有 Dogfooding 实例的 authenticated public Core。每个探针使用独立 consumer state root，既有 Subject 与 WorkContext 不改动。调用无效 `CreateSubject(subjectId="invalid-uuid")`；该请求没有创建有效 Subject 或调用模型。

基线连续 257 次尝试：前 256 次 Core 返回 `InvalidArgument`（code 3），却保存 256 个 `pending`；第 257 次本地返回 `RESOURCE_EXHAUSTED`，没有到达 Core。呈现 `{text:"33333333-3333-4333-8333-333333333333"}` 丢失正文。

修复后独立 consumer 的同一真实路径连续 257 次均返回 Core code 3；256 个保留回执均为可回收的 `rejected`，新操作可以继续。完整 UUID 正文保真。焦点检查同时区分 UUID/ref title 与正文、exact evidence locator、未知结果原请求恢复、恢复后的明确拒绝，以及成功结果在呈现失败时仍可读回。成功回执读回不重发业务调用。

后续共享状态检查又复现了一条结果分类缺陷：Authority 明确拒绝后，拒绝消息使终态回执超过配置文件预算，本地 `RESOURCE_EXHAUSTED` 会覆盖原 Authority 拒绝。当前保留原错误，并带原 receipt 与 `RECEIPT_UNSAVED` notice；焦点检查用实际文件预算与可控 Authority 错误保护这一已知结果。

本次验证运行的是修改后的 consumer 与已有 Core；没有将其记作全新数据库、当前 Kernel 或 Portable 验证。

## Query 与纵向检查精简

两份超限 Rust 集成文件按实际风险精简。删除 Query 中重复的 VCP 中间数学/协议字段回放和已有独立数值 oracle 覆盖的脚手架；保留 exact/frozen snapshot、rerank 后 Authority 变化、Material 时间、Schema lifecycle/provenance、Resource forged continuation、预算诊断与扫描边界。纵向检查删除重复 Protobuf/DTO、分页和通道回放；Experience/lease、Episode partition/late arrival、Journal provenance/purge、partial actions、维护恢复与历史仍在各自 owner 场景中验证，共用测试内的实例和来源构造。

保留场景均通过当前 Kernel/临时 PostgreSQL。新增 Serving 场景真实保留在途 reader、建立下一代、验证回收保护，释放 reader 后回收旧资产，再删除缓存并重启，在同一 Authority watermark 召回原对象。该场景按 generation family 定位资产，使用持久 metadata 与实际文件可读性证明重建。

Material 时间场景曾一次未召回预期 region；诊断重跑和移除诊断后的整组重跑均通过，未据此修改领域语义或删除该断言。该一次失败的原因尚未证实，后续综合实际使用继续核对这条时间/来源路径。

当前 `check:length` 覆盖 406 份手写源码，16 warning、0 reject、0 read error。当前全 workspace Clippy、`check:fast`、TypeScript tests 和文档导航通过；这些证据不替代未完成的全仓重整与真实模型/Portable 任务。

## 手写源码长度门禁

门禁从单一 TOML 读取门限，已纳入 `check:fast`。临时 Git 目录验证达到门限的 warning/error、CRLF/LF/孤立 CR/BOM、未跟踪新文件、工作树修改、已删除文件、生成排除及未知配置键的 exit 2。起始基线有两份超限 Rust integration test 与旧研究 runner，已按上述实际风险精简；长度拒绝没有基线豁免。

## 配置规范化与身份

改动前的 owning Rust 检查实际复现了两种身份差异：只更改 descriptor exposure 改变 registry identity；同一 key 集合改变排列或增加重复项会改变 subset identity。Core 的真实 `video` 配置在省略默认字段时，启动交接中的 JSON 与显式 defaults 不一致。

修复后 Rust owner 检查确认 exposure-only 的 Catalog identity 会变，effective identity 不变；subset 按集合计算。Rust typed policy 与 Core Zod policy 在进入 snapshot 前规范化，不采用通用 JSON Schema defaults 解释器。

使用新构建的当前 Kernel 和源码 Core 运行隔离 configuration smoke：同一 operation ID 的 `video={max_frames:4}` 与完整显式 defaults 回放得到同一 revision/desired digest；desired 读回完整 owner 值，active 在 RestartProcess 前仍为原值。停止并重新启动后 active 读到该完整配置。公开 CLI config check/list、scope precedence、回放和 restart 同时通过。该临时实例没有模型调用，退出时停止并清除；既有 Dogfooding 实例保持运行。

## Terminal/MCP 共同执行

移除 MCP 每次命令启动 CLI 子进程的路径。当前 Terminal 和 MCP 在同一命令核心中解释 argv、执行 handler 和格式化结果；Terminal 的 stdout/stderr 与可消费 stdin 由 Terminal 入口持有，MCP stdin 只供协议。

真实 SDK stdio client 启动当前 MCP server，执行 NousQL help、JSON command help 和未知命令。握手及逐次消息均由 SDK 成功解析，help 结构可读，未知命令为 MCP `isError`，没有额外 stderr；不需要启动 Core 或调用模型。这验证协议纯净性，不替代后续真实 Codex 认知任务或长机会执行验证。

共享 consumer 改动另用当前 Core/Kernel 和干净 PostgreSQL 实例实际验证：配置 `consumer_state.receipt_limit=2`，Terminal 创建并保存 Subject，随后真实 CLI 子进程与 SDK stdio MCP 进程共用 state root/consumer 并发打开 Session、创建 WorkContext。两项选择均保留，MCP 能读 Terminal 保存的 Session，Terminal 能读 MCP 保存的 WorkContext；Core 重启后实例身份与 consumer 选择保持一致，MCP stderr 为空。临时实例退出时停止，数据保留在本轮临时目录等待统一清理；该场景没有模型调用。

焦点检查复现并修正旧 BigInt tag 与用户 JSON `$bigint` 键碰撞。四个真实 Node 进程并发写容量为 2 的 pending 空间，恰好两项成功、两项收到容量拒绝；未知请求不回收。同 operation 的不同 frozen input 被拒绝，终态不被 pending/另一个终态覆盖。连续 query 的清除旧结果与保存新结果使用本命令最近成功写入的基线，第二条 query 的 exact `result:N`/UseEvent 对应第二次 Client 返回；该焦点检查使用可控 Client。

[返回研究入口](README.md)

## Resource 宿主与接纳

生产 Core 移除 RAGFlow 专用 wire adapter、配置 owner 和专属测试。宿主在启动时按 `adapter_kind`/`provider_profile` 显式提供唯一 adapter；LocalDocuments 不再继承 registry 或覆盖其解析方法。公开 descriptor、query continuation 与 Material 接纳继续使用原语义 owner。

当前源码 Core 的隔离 Model/Material/Resource smoke 已改为 LocalDocuments，实际通过公共 API 验证接纳、重复 operation 返回同 Observation、新接触保留独立 Observation、版本变化拒绝、权限撤销拒绝、已接纳来源在外部撤销后仍可读，以及 1 MiB 普通/控制字符材料与恢复。模型部分使用本地协议替身，不计作火山方舟真实模型质量或持续任务结果。

恢复检查另用可控 Kernel 替身确认了旧路径的缺陷：已有 proposal 在来源变更或权限撤销后仍会提交。Core 现在在接纳前核验实际 provider identity、当前版本和权限，包括 saved proposal 的恢复；已完成 outcome 则继续复用已接纳快照。该检查保护恢复窗口，真实 Core smoke 保护公共 Resource/Material 行为。

## 本轮真实 Codex 与模型续接

使用本轮独立 PostgreSQL/Artifact/cache/config 主实例、当前 Kernel、源码 Core 和官方 consumer。真实 Codex CLI 通过 Nous stdio MCP 保存两项实际重整来源、Subject 与持续 WorkContext；后续 fresh Codex 进程恢复同一 Subject/WorkContext，打开独立 Session 并继续。

首次非交互 CLI 因本机工具审批默认值与 `never` 冲突未执行认知操作；只为本轮三个已授权 Nous 工具配置官方 per-tool 模式后可调用。首轮观察的 `rebase://` locator 不符合 web source HTTP(S) 合同，改以原文观察保留内部 locator 映射。两次 formation 与 embedding 最初返回降级，query 明确拒绝：新实例复制的历史模型配置仍指向无 listener 的 `127.0.0.1:18002` 研究代理。当前 New API `/api/status` 与认证 `/v1/models` 均为 HTTP 200、所需模型存在；仅本轮实例 gateway 改为当前 `127.0.0.1:3000/v1` 并正常重启。旧实例、operator 原配置与 credentials 未修改，原失败操作身份和失败结果保留。

fresh Codex 进程读回原来源后，使用新 UUID 执行已明确失败且无 Memory 的两项 formation；两项均成功，trace 标明实际 `doubao-seed-2.1-pro`、`pro_minimal` 与 whole-Observation grounding。模型生成正文、exact revision、trace、原始来源和实际检索结果保全在[本轮 consumer 续接结果](corpus/deep-rebase/consumer-continuation-2026-10-09.json)，没有把旧内部回执读入新 consumer 格式。

| 原始 Observation | 形成的 exact Memory revision |
| --- | --- |
| `obs:lihih-titij-popad` | `memrev:sajot-zujup-muhug` |
| `obs:dutob-dasad-pokol` | `memrev:raman-zajos-ruvuj` |

原 Observation 在形成前后均完整读回、原文不变。Embedding preparation 实际提交 6 项、请求 2 次。普通自然语言 query 在 `baseline-rrf`、concept enrichment `off` 下完成，consumer revision 为 rank 1，报告 lexical/dense/language-rerank 参与；一个 referenced UseEvent 接受计数 1、重复计数 0。WorkContext `ctx:raraf-romup-hasal` revision 6 保存 Objective/Decisions/Current Work、exact refs 与失败 ledger，读回确认 open；fresh Session 关闭。当前 Subject `sub:tujap-vibuz-gozod` 保留供后续持续任务与 Portable 续接。

旧实例 consumer 记录中找到的两个 pending（UseEvent 与 formation）已按原始字节复制并逐份 SHA-256 核对，manifest 保留原路径、receipt identity 与 Subject；原目录仍保留。其具体内容位于 ignored 本地研究成果区，不加入公共结果文件。Linux SAUC 独立硬盘提取、全仓剩余 owner 重整、媒体 route/deadline/lease 与 Windows source-less Portable 尚未完成。

## 模型输入、fallback 与执行 owner

两组不同音频能力的实际配置经受控 HTTP 复现了原条件：首选有 audio_input、fallback 没有时，首选返回 503，fallback 的音频观察错误通过并被提交；反向配置则误拒合法音频输出。另一 owning contract 检查复现原始帧与 Transcript 共用 original 标签时，转写内容被允许声明为直接听到。

当前 Model input owner 每次 attempt 按实际媒体和冻结候选 profile 过滤输入、构造实际访问声明；Material interpretation owner 使用这份声明校验。visual/audio/source_text 各自保存 original/representation/unavailable，Transcript 不取得直接听到的身份；description 第二阶段读取实际持久 quality 而非首选模型能力。当前公共 Core/Kernel/PostgreSQL 的 smoke:media 验证两组配置、持久 quality.input_access/payload.evidence_access 一致，以及成功重放不增加 provider 请求。该 fixture 使用 opaque bytes 与受控输出，不声称火山媒体质量。首轮临时 PG setup 曾超时退出；同一检查重跑通过，未修改启动超时，失败未作为成功证据。

实际 HTTP cancellation 检查另复现旧 telemetry 在 fallback 中断时整体丢失；当前保留首个 503 failed 与已传输的第二个 unknown/caller_cancelled attempt，未知 usage 保持未知。完整 SDK/raw-media 响应已知 usage 在输出校验失败时保留。独立 profiles、协议 adapter 与 Material interpretation 移出 invocations；没有依赖其私有状态的 part 类或转发兼容层。协议读取使用 get-stream 的字节上限替换两份 reader 循环。

手工 material projection/validation version 常量已替换为同一个 owning code identity。Source/bundle 探针确认源码与 source-less ESM 程序的 provider/material 实现摘要相同，bundle 内实际含 get-stream、proper-lockfile、write-file-atomic 与 devalue，CLI JSON help 可解析。该证据只覆盖当前 bundle 与执行身份，完整 Windows Portable 连续任务、deadline/lease 统一与其余全仓 scope 仍待完成。

## 执行机会与确认期限

受控 Client transport 时钟复现：120 秒授权工作后仍需要 8 秒确认，旧 `maxElapsedMs + 5000` 在第 125 秒返回 deadline。当前 Core 与 Client 共用 `core_execution.opportunity` typed policy，工作、cleanup、need acknowledgement 和 response margin 分开命名。Client 从 active Configuration 计算实际等待；显式更短 caller deadline 保留。每条 provider route 的 timeout 继续限制单次调用，不能重置工作机会。

Core 的 ordinary formation/derivation/embedding/resource 工作使用同一 work clock，private ModelWorkflow reservation 显式携带该机会的剩余 lease，删除另一个 `model_workflow.lease_seconds` 默认值。maintenance 父 claim 至少覆盖 worker policy 与相同 work/cleanup/ack，child 仍验证实际父 token 并覆盖父期限。取消后 telemetry/save/release 共用有界 cleanup，finish 使用独立 acknowledgement。现行 PG workflow/maintenance 四项场景通过，包括 proposal/terminal replay、父期限覆盖与 runtime rows 回收；30ms 机会到期检查确认 unknown attempt 以有效 cleanup signal 保存，need 返回 pending/deferred。

公共 Core/Kernel/PG smoke 额外以 1 秒 work budget 中断已传输且保持 pending 的 fallback。响应是 Code 4 DeadlineExceeded，cleanup 后相同输入立刻恢复成功，没有遗留 busy lease。该过程使用受控 provider，仍不声称真实媒体质量。持续实例正常重启后 active policy 读回为 work 300000ms、cleanup 10000ms、ack 5000ms、response margin 1000ms，Subject/Runtime/Memory 与当前 model roles READY。TypeScript 33 files/109 checks、check:fast、Knip、依赖边界、docs navigation、Rust workspace check/Clippy 与当前 source-less bundle 探针通过。Query retained execution lease ownership及其公共等待、完整配置 graph/CAS/leaf、Windows Portable 连续任务和成果保全后的清理仍属于原目标未完成部分。

Fresh Codex 第三次在同一 Subject/WorkContext 完成真实续接。531 字节新来源 `obs:bojin-gagof-pujaz` 原文形成前后保留，Doubao Pro/pro_minimal 形成 grounded revision `memrev:bufor-rurov-pupik`，trace 关联 wholeOccurrence。Embedding committed 9/requests 2，指定自然语言 query 召回该 revision 为 rank 1，referenced use accepted 1/duplicate 0。Context revision 7/open、Session closed 读回确认，19 份原始 public 结果保存在 [续接 corpus](corpus/deep-rebase/execution-continuation-2026-10-09.json)。该实际 trace 还暴露 Formation 的 ProducerSignature 仍覆写为手工 SDK 版本常量；原输出及已有 cognition 保留。公共 smoke 加入实际 producer readback 后先失败于这个旧常量，再在 Formation/Material/maintenance 共用 metadata owner 映射后通过。当前执行 snapshot 在发出请求和预算 admission 前核验固定角色的 output schema digest，避免用新合同解释原固定执行。

## Query 同一机会的生命周期

真实 PostgreSQL 检查以 2 秒执行 lease 复现：原 preparation 在 1.4 秒后取出、重新保留，超过原始期限仍能读取。旧 PendingQuery 在每次 retain 使用新 created/default duration，等同于激活延长机会。当前 Runtime QueryReservation 分开持有 BoundQuery 与一个 QueryLease，新的 preparation、execution ticket 和 final validation 继承相同 infrastructure deadline；删除独立 `runtime.query_lease_seconds` 默认值。Core 的编译、Query/model/resource/projection 阶段共用执行机会，Client 也从 active policy 派生等待。原机会截止、验证票据到期、配置/context 冻结、Authority lifecycle/epoch revalidation、Serving reader 回收与公共纵向检查通过。

当前 Core/Kernel 正常重启后，真实 CLI 在同一 Subject/WorkContext 开 fresh Session 执行自然语言 Query。首个结果因 producer identity 重整后的 compatible embedding material 缺失而 degraded；显式 prepare committed 9/requests 2，随后相同 query complete、召回原 `memrev:bufor-rurov-pupik` 为 rank 1，dense/language_rerank 证据保留，Session 已关闭。首次失败和恢复的六份原始 public 输出保存在 [Query 续接 corpus](corpus/deep-rebase/query-continuation-2026-10-09.json)。这次续接不重写原 cognition/history，不替代剩余配置图、全仓 owner 收尾和 Windows Portable 验证。

## 模型配置图与冻结

三项 owning 检查先失败：generation/gateway 默认 tokens/timeout 尚未进入 normalized graph；显式缺失 route 仍被接受；无关 dormant model 和 video budget 改变 formation snapshot identity。当前 `models` 将紧密耦合的 gateway/model/execution/role 作为一个原子 policy 发布；真实输入与 defaults 在该 owner 完成规范化，再交给 Kernel Catalog 冻结。非法引用/协议/推理组合给出精确路径；缺凭据、disabled gateway 和 unset model identifier 保留 capability state。Role snapshot 与 embedding producer 使用同一依赖子图，只捕获相关角色、execution、model 与 gateway；删除调用处补4096的 resolver。

当前 public configuration smoke 验证 sparse graph 与完整默认 graph 同 operation replay、desired normalized tokens/timeout、错误 route 的精确 public 路径及 restart 生效；Model/Material/Resource、相反媒体能力和期限后恢复 smoke 通过。34个TS文件112checks、typecheck/lint/Knip/dependencies通过。独立字段 leaf descriptors、修改 revision CAS、更多实际 effect/apply mode 和完整 Portable 仍须继续，不能从该模型图检查推导全配置已完成。

## 配置修订与回执重放

公开配置 smoke 先复现旧 Kernel 接受过期 expected revision、覆盖整块模型图的缺陷。当前 canonical Proto、Official Client、CLI 和研究调用方共同携带 expected revision；公开 mutation 缺字段返回输入错误，CLI 写入要求先读取 desired revision 并显式提交，避免重试时重新读取修订而改变请求身份。Kernel 在配置事务内比较并递增全局 revision；冲突不写 override、不推进 revision，已完成原请求先读取冻结回执。

实际 PostgreSQL 集成检查验证同一 revision 的两项并发写入只有一项成功、过期 clear 不删除获胜值，后续修改后仍重放原 operation 的 outcome；该 owner 检查同时保留独立字段无条件内部写入的串行完整快照合同。公共 Core/Kernel/PG 与 CLI smoke 验证过期模型图拒绝且 desired 不变、CLI 缺 revision 拒绝、过期 clear 拒绝、Subject 修改后原 CLI operation 可重放，并完成 restart readback。configuration 五项集成检查、TS34文件112checks、check:fast 和 Rust workspace all-targets/all-features check 通过。独立 leaf、effect/apply 与完整 Portable 等原目标仍未完成。
