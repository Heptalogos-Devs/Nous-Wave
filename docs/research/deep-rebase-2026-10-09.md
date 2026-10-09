# 全仓重整实际观察（2026-10-09）

本报告保存本轮实际复现与结果，后续追加同一任务的新观察。任务范围包括 owner 结构、配置身份、执行、consumer、Serving、Portable 与成果保全；当前观察不证明整项重整已经完成。

## CLI 正文与明确拒绝

基线 `09d67d4452eead78346877980f804ac513675d8d`，Windows 上使用当前源码 CLI environment 连接已有 Dogfooding 实例的 authenticated public Core。每个探针使用独立 consumer state root，既有 Subject 与 WorkContext 不改动。调用无效 `CreateSubject(subjectId="invalid-uuid")`；该请求没有创建有效 Subject 或调用模型。

基线连续 257 次尝试：前 256 次 Core 返回 `InvalidArgument`（code 3），却保存 256 个 `pending`；第 257 次本地返回 `RESOURCE_EXHAUSTED`，没有到达 Core。呈现 `{text:"33333333-3333-4333-8333-333333333333"}` 丢失正文。

修复后独立 consumer 的同一真实路径连续 257 次均返回 Core code 3；256 个保留回执均为可回收的 `rejected`，新操作可以继续。完整 UUID 正文保真。焦点检查同时区分 UUID/ref title 与正文、exact evidence locator、未知结果原请求恢复、恢复后的明确拒绝，以及成功结果在呈现失败时仍可读回。成功回执读回不重发业务调用。

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
