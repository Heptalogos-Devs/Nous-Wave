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

## 独立配置字段与实际消费者

Catalog 检查先确认 `video.max_frames` 等真实叶路径不存在。当前 audio/video、Material inputs、Core execution/opportunity 和 official consumer state 的独立字段直接使用原 Zod owner 发布 descriptor，不再注册重叠 parent；模型引用图保持一个完整原子 policy。Core 从 Kernel 已解析叶值重组 typed group，Client deadline 与 CLI consumer state 读取同一 active 叶值；离线检查和研究合同 inspection 共用 registered owner 路径提取部署值，删除重复 material 字段拼装。单位随字段发布，媒体/Material 标注 authority formation、consumer composition/rerank candidate budget 标注 query policy，实际仍需要重启的参数继续返回 RestartProcess。

FFmpeg executable reference 使用 null 作为已安装 runtime pack 的明确选择；string 是显式引用，空 string 非法。公共配置检查实际验证 max_frames 的独立范围、另一个 input_mode 保留默认、旧 parent 不可 describe、单字段 override 重启生效，以及清除该 override 后 desired 回落到部署值而 active 保留原值。公共模型/材料/资源、两种媒体能力、work deadline cleanup/resume 与纵向 Query/维护检查通过，TS34文件112checks、check:fast、Knip 与依赖检查通过。

只对本轮持续 Core 正常停机并换用 SHA-256 核对的当前 Kernel 副本后重启；实例 ID 保持 `ffe03b02-8997-476e-b8d1-370e88aac1a2`，新 endpoint 为 `http://127.0.0.1:38930`。active opportunity 四个叶值读回 300000/10000/5000/1000ms，Subject/Runtime/Memory 和11个 model roles READY；atomic models 仍为6个 model/11个 execution profile。真实 CLI 指定原 Subject/WorkContext 后读回 revision7/open。首次未指定 Subject 的默认 consumer 查询被明确拒绝，没有修改选择或认知状态。旧实例和 Linux 硬盘未清理；完整 Portable、只读批量地址呈现、其余 owner/fresh SQL 重整与原目标仍待完成。

## 地址目录的只读批量入口

当前原 CLI friendly renderer 和 Core Query 呈现仍会调用逐项 Bind；检查同时确认多数地址在首次显示时才分配。新的 canonical `GetIdentityAddresses` / Official Client `identity.addresses` 提供单次目录 read，按 Subject visibility 隔离，保留目标顺序；缺失地址保持缺失，不分配、不改名、不增添 visibility。Tag 当前 lineage 从目录的 canonical Tag 规则解析，未知和 tombstoned 引用返回不同状态。

实际 PostgreSQL 的地址分配/冲突检查扩充验证批量读前后 lexical bindings、visibility、Authority history 行数相同；不存在和跨 Subject 的引用不给地址，tombstone 不给可操作地址，合并 Tag 与 survivor 得到同一规范地址。现行 concept lineage/Query/Serving 场景和公开 Core/Kernel/Client smoke 通过，公开 read 后原 display name/aliases 不变。该入口只是 owner 能力；schema 明确的字段呈现、创建/显式地址服务的分配和原逐项 Bind 删除仍须完成，不把新增 read RPC 当成呈现已经迁移。

本轮持续 Core 正常换用 SHA 核对的当前 Kernel 后重启，instanceId 不变，endpoint 为 `http://127.0.0.1:37081`。早期 discovery 尚未生成，原启动进程确认仍活跃后再次观察 READY，没有因此重新启动。新 public batch RPC 读回原 `sub:tujap-vibuz-gozod` 与 `ctx:raraf-romup-hasal` 的相同 lexical addresses；原 WorkContext revision7/open。Subject/Runtime/Memory 与11model roles READY，opportunity 四叶保持原值。该轮没有新增付费模型调用或改写认知历史。

## 协议引用呈现与创建事务

Client JSON 保真检查先复现 `$typeName`/`$unknown` 及嵌套内容被当作协议字段删除。当前 data codec 按真实 protobuf descriptor 处理 Value/Struct、oneof、list/map，保留 JSON opaque leaves；schema 使用 WeakMap，consumer 的 devalue 类型记录保存并恢复实际 schema，包括 timestamp dependencies。任意 JSON 键不获得协议身份。配置值与 schema/default 的三套单独转换收敛到同一 owner，CLI 继续只依赖 Official Client。

Canonical Proto 标注实际引用字段；CLI renderer 删除 UUID/字段名推断和逐项 Bind，按声明的引用去重只读查询。Source locator、正文、title 和 JSON 数据保持原值。Query diagnostics 使用 owning command 的明确投影，developer trace 保留。创建 owner 在同一 Authority 事务内发布对象、immutable revision 与 Material 地址，保留已存在的名称/aliases；Core Query 同样只读取得 exact preferred revision 地址，地址服务故障不抹掉已取得的 Query 结果。Subject seed adoption 与 Journal revision write 使用各自领域 payload，不以 receipt kind 推断分配，也不添加通用 CRUD 状态机。

公开 CLI 首次创建检查先在旧 Kernel 下返回 UUID而失败；当前 Kernel 下返回稳定 `sub:` 地址并通过完整 configuration smoke，包含真实 provider-options JSON 同名键的 normalized graph/readback/replay。独立 receipt 检查验证 schema、BigInt、timestamp 和同名 JSON 键往返。35个TS文件112checks、check:fast、Knip/dependencies、workspace Clippy通过；16项 Native Authority/Material/Memory/Schema/Journal/Episode/Tag/preparation 场景与公开 Model/Material/Resource、相反媒体能力、期限后恢复及纵向 smoke通过。这轮受控 provider 不计作真实模型质量；当前持续实例升级与完整 Portable 等原目标仍待继续。

持续实例已换用 SHA 核对的当前 Kernel 并正常重启，instanceId 保持不变，endpoint 为 `http://127.0.0.1:13751`。当前 public batch 读回原 Subject/WorkContext 的同一 lexical refs，Context revision7/open；普通文本 CLI 的 `context show` 实际返回 `ctx:raraf-romup-hasal`，结果原样保存供续接。Subject/Runtime/Memory 与11model roles READY，opportunity 四叶保持原值；没有新增付费 formation 或改写旧 cognition。

## 初始数据库与 Portable 恢复

现行数据库 schema 收敛到 `0001_foundation.sql`–`0004_indexes.sql`，直接声明 WorkContext text/anchors、Formation basis、ProducerSignature 字段与 workflow execution telemetry。删除三份旧增量 schema 及其递归 JSON 改名逻辑；Core/Kernel 同时删除没有独立语义的私有 bootstrap `bundle_revision`。认知 revision、epoch、来源身份与配置 revision 保留。首次检查发现 Schema evidence 的 `support` 判断被机械改名，恢复原语义后 configuration5、historical2、memory5、model_workflow1 场景通过。

原持续实例先保存完整 PG archive，正常停机固定数据，再将 data-only 恢复到由四份当前 schema 初始化的新库。新库保留自己的 `_sqlx_migrations`，没有重写旧库迁移账本或加入旧 payload decoder。70张业务表、196行逐表 SHA-256、列合同与序列位置全部相同；原数据库、Blob、operator configuration、Secret 和两份完整备份仍保留。停机备份摘要为 `bbc34f0cbdd87e6db85b1dc630a7081ed76b14e84773e97305ccba1b8657c0f5`；本地 archive、数据摘要和恢复 manifest 位于 `data/research/results/deep-rebase-preservation/fresh-schema/`。新库公开读回同一 instanceId、Subject/Context revision7/open 与三个原 Memory trace。

Windows GNULLVM release Kernel、两个 private DLL 与 source-less Portable 已重新构建。独立 home、colocated、独立 locator 后移动安装目录三种布局均通过 ZIP inventory SHA、SBOM/许可证、实际 boot/restart、固定数据库端口和缺失 PostgreSQL pack 拒绝检查。受控 wiring 结果不计作真实模型质量。该包的2333个文件摘要再次核对后，使用独立 locator 连接恢复库、原 instance identity 和 operator roots；只用 bundled Node/CLI/Client，Core 的 PATH 仅含 Windows System32。公开读取继续返回原三个精确 Memory trace、Context revision7/open、Subject/Runtime/Memory 与11个 model roles READY。

此前8f28074的 Windows CI通过，Linux CI在 Resource continuation 发现 nullable `display_label` 被地址分配路径按必填 String读取。Resource owner改为先解码 typed descriptor，使用可空 label 发布地址；原失败场景及 Query correctness五项通过，workspace all-targets/all-features测试、Clippy、check:fast和格式／文档导航检查通过。

Fresh Codex 经编译后的 Portable MCP，在同一 Subject/Context 完成第四次真实模型续接：545字节 `obs:lalof-sarus-muzis` 形成 `memrev:jazop-nakis-tipum`，实际 Doubao Pro/pro_minimal、wholeOccurrence trace；embedding committed12/requests2，指定自然语言 Query complete、该修订rank1，referenced use accepted1/duplicate0。Session关闭，Context revision9/open。随后正常停机、用同一 locator 和 bundled runtimes 重启，公开读回 instanceId 不变、Context9/open；三个原 Memory trace 的 JSON字节与重启前完全一致，摘要留在本地实例。

重启后的另一 fresh Codex恢复原Context与已关闭Session，未新增 formation、embedding preparation 或 use；同一自然查询 `01a12061-8db3-7ec0-968f-64a92d599a3d` complete，四个 Memory精确修订依次召回，新修订rank1，lexical/dense ready。新Session正常关闭，Context保存并独立读回 revision10/open。两次实际任务的原始命令结果保存在[Portable续接 corpus](corpus/deep-rebase/portable-continuation-2026-10-09.json)。初始 projection wildcard unavailable和上下文 truncation也保留，没有将其替换成全部READY的叙述。

Windows只读盘点确认独立物理盘1的分区2为Linux filesystem GPT类型、无Windows盘符。只读 `ro,noload` WSL挂载及执行权限重试均返回 `WSL_E_ELEVATION_NEEDED_TO_MOUNT_DISK`；没有挂载或修改该盘。SAUC提取仍需管理员挂载或从Linux侧导出，相关旧材料继续保留。全仓其余原范围与最终CI／清理尚未完成。

## 旧 synthetic 功能执行器退役

按任务包第03项核对 Vault Target/Decisions 与当前 longitudinal、Authority/provenance、Runtime/Use、Query/Serving、WorkContext Specs 后，退役 `cognitive-functional.ts`、`NOUS_FUNCTIONAL_SMOKE` 分支、专属文本匹配 provider、CLI 子进程脚手架、package/Knip/script入口及活动说明。手工三场景、22事件、15 intents、四 profile 和 selected raw-text source/oracle 保持原文件；五份语料及历史说明另按原字节/SHA保全到 `data/research/results/deep-rebase-preservation/functional/`，未清理旧 raw 数据。Spec 中的旧实验执行状态移出活动合同，其原事实继续留在研究语料说明。

纵向 public smoke仍实际执行有界维护、模型 proposal、owner提交、重放和Core/Kernel重启。Concept局部catalog/Accretion、Tag lineage/历史/目录与 typed Core partial-commit/transport-resume 的独立检查保留：受影响Native5项、Core9项通过；check:fast、Knip、依赖边界、文档导航和diff检查通过。脚本从902行降至452行，删除一份687行旧runner；长度门禁422手写文件、16warnings、0reject。这轮没有削减 production功能或改动领域语义，仍需按完整目标完成其他实际操作与成果保全／清理。

eadb42a的Windows CI通过；Linux未进入测试，显式runtime准备调用第三方GitHubrelease API返回403。已核对安装的 `postgresql_archive` 实现，其支持 `GITHUB_TOKEN` Authorization；CI仅给runtime准备步骤传递现有contents-read job token。没有跳过准备或降低验证；新head的实际CI结果另行确认。

9187d7c的Linux/Windows CI均通过，Linux明确完成runtime准备及完整`just check`。随后通过Windows正常管理员确认，以核验过身份的独立物理盘1/分区2执行`ro,noload`挂载、保全并卸载，解决此前非管理员进程无法读取的问题。实际Linux来源为`/home/arsvine/Services/new-api/`，补丁是独立的`volcengine-asr-transcription.patch`；36,724字节，原文件和Windows提取文件SHA-256均为`e2867c0b7a8558bb6e1ed542bcf0c6580b84a5217174e6133f3db1a8b6ada426`。源码、Git目录、构建/部署文件和私有operator配置同存于 ignored 保全区，原盘未改动且已卸载。

Linux源码checkout为clean，HEAD`271976055fcbd75a7180d6ca8389ec8c80fa4437`，已经含SAUC/WebSocket实现。Windows恢复的Git HEAD与原盘一致，`HEAD:relay/channel/volcengine/asr.go`与工作文件归档逐字节一致，SHA`b62239f8c9bf27eccf36d55a1b9361734f91cc4f65c16a6f14ddfbab3752946e`。原补丁包含多段增量及Responses修正，直接对当前已修改源码再次单次apply-check不通过，未据此改写补丁。该仓库是partial clone，全refs bundle因缺promisor对象失败；完整本地Git目录和当前源码已按字节归档，没有下载未在原盘保存的上游history。补丁和当前已提交实现的可读保全已经验证，Windows New API运行服务未修改或部署。

已有真实SAUC成功结论、Agent Plan channel、24kHz→16kHz规范化、Menon40秒WAV约7.2秒转写、MP3及production Transcript/formation/Query结果继续由[2026-10-04研究记录](observations.md#asr-与-responses)和原source manifest拥有，本轮不重新命名为当前运行结果。本地精确补丁、配置与archive/verification位于`data/research/results/deep-rebase-preservation/linux-sauc/`；密钥值没有进入Git或报告。

## Linux 接续与 Serving 身份

Linux 从最新 `master@09d67d4` 核对后接续同一分支的 `d5ca936`；Windows checkout 工作树干净，现有 Draft PR 仍为 #24。Windows 两个数据分区以只读方式挂载，保全区复制到 Linux 私有成果目录。独立 Linux SAUC 补丁与 Windows 提取副本的字节摘要相同，运行中的 New API 未修改。Windows 的现行 Portable、Subject、Context revision10/open 与实际模型输出继续保留；不把此前完整任务重跑为新的成功记录。

剩余 owner 审查发现 Serving 仍保存手工实现计数，并在结构化 record 与 metadata 中复制同一身份。真实 PostgreSQL 回归将 record 的实现身份置为过期、保留原 metadata，原路径仍复用旧资产；修复后在相同 Authority watermark 上重建。实现身份改为投影代码、输入投影和锁定依赖的构建内容摘要，兼容判断只读取 owning record。当前与历史资产安装合并，Native Wave 的策略注册与图构建分开。独立目录中的实际构建脚本探针确认：投影源码变化改变身份、只改 Wave exposure 保持身份、LF/CRLF 与源码根移动保持身份。

参考默认族直接使用当前 JSON 文件和 identity，删除 `-v1` 文件名及重复 revision 包装；当前所有读取方同步替换，没有旧格式解析。数值未改变，历史研究身份和 corpus 保持原样。Linux `check:fast`、35份 TypeScript 测试、Rust workspace/all-features 测试、Clippy 通过；长度门禁424份手写文件、15个警告、无拒绝。配置、历史 Authority、同水位重建和 reader 回收的真实数据库检查通过。干净 Linux 实例配置已通过离线 owner 校验，实际 Codex/模型任务、剩余配置与 owner 审查、成果提取和运行材料清理继续进行，整项重整尚未完成。

干净 Linux 实例随后实际启动，公共 CLI 读回 Subject/Runtime/Memory READY 与11个配置角色 READY。新的 fresh Codex 仅通过 Nous MCP 完成一项认知任务：1125字节来源 `obs:govig-fumoh-rokad` 原文读回不变，实际 `doubao-seed-2.1-lite` / `memory_formation_text` 形成 `memrev:gizoz-zonas-hivin`，grounded/accepted/valid，trace 与源区域保留。Embedding committed3/requests2，自然查询 `01a120ec-d973-7b72-82b1-7ed92ea84d51` complete、该修订rank1；referenced use accepted1/duplicate0。Subject `sub:bizor-fusin-vakav`、WorkContext `ctx:supud-pasun-kopud` revision3/open 保存待续工作，Session 已关闭。首次 Use 错误传入 connection-owned consumer，被 MCP 明确拒绝；移除该参数并沿用同一 event identity/timestamp 后接受。34份实际命令输入/结果及 consumer 结论保存在 [Linux续接 corpus](corpus/deep-rebase/linux-continuation-2026-10-09.json)。

Linux 续接审查将 concept activation 的最大保留数、最小 cosine 相似度与 model catalog 数交回 Runtime 的三项 Advanced/SubjectOverrideAllowed/Live/QueryPolicy 叶。Retrieval activation 与模型 readout 读取同一冻结 ConfigSnapshot，删除直接静态 JSON 解析和未读 strength mapping；真实数据库检查验证先绑定1/1、更新3/3后旧请求保持1/1，新请求读3/3，并按 similarity 过滤正交 Tag。provider 响应的结构边界仍由模型合同独立拥有。

实际重启发现两处生命周期缺陷：Core 在收到 SIGTERM 后仍持有启动方 stdin，资源已关闭但进程不退出；移除该进程拥有的信号/end listeners 并暂停 stdin 后，开放 stdin 下直接 SIGTERM 正常退出且端口释放。Kernel 在 embedding 绑定前后各刷新全部 Serving，导致启动阶段不可用告警及无需求建资产；删除两处启动刷新，按请求准备及显式管理刷新保留。缓存丢失的真实 PostgreSQL 回归先复现启动即建资产，修复后启动目录为空、首次查询在相同 Authority watermark 重建并召回原两条修订。请求、当前及历史快照现在共用 owning asset install。

重复代码扫描使用配置的 apps/packages/crates/scripts 范围，移除覆盖该范围的命令行点路径；研究 corpus 的真实重复输出不作为代码克隆。原生 workspace/all-features、Clippy 与完整35份/112项 TS 检查通过；并行负载下两项子进程检查超时，负载结束后原检查通过，未放宽 timeout。
