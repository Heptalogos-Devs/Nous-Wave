# 全仓重整实际观察（2026-10-09）

本报告保存本轮实际复现与结果，后续追加同一任务的新观察。任务范围包括 owner 结构、配置身份、执行、consumer、Serving、Portable 与成果保全；当前观察不证明整项重整已经完成。

## CLI 正文与明确拒绝

基线 `09d67d4452eead78346877980f804ac513675d8d`，Windows 上使用当前源码 CLI environment 连接已有 Dogfooding 实例的 authenticated public Core。每个探针使用独立 consumer state root，既有 Subject 与 WorkContext 不改动。调用无效 `CreateSubject(subjectId="invalid-uuid")`；该请求没有创建有效 Subject 或调用模型。

基线连续 257 次尝试：前 256 次 Core 返回 `InvalidArgument`（code 3），却保存 256 个 `pending`；第 257 次本地返回 `RESOURCE_EXHAUSTED`，没有到达 Core。呈现 `{text:"33333333-3333-4333-8333-333333333333"}` 丢失正文。

修复后独立 consumer 的同一真实路径连续 257 次均返回 Core code 3；256 个保留回执均为可回收的 `rejected`，新操作可以继续。完整 UUID 正文保真。焦点检查同时区分 UUID/ref title 与正文、exact evidence locator、未知结果原请求恢复、恢复后的明确拒绝，以及成功结果在呈现失败时仍可读回。成功回执读回不重发业务调用。

本次验证运行的是修改后的 consumer 与已有 Core；没有将其记作全新数据库、当前 Kernel 或 Portable 验证。

## 手写源码长度门禁

门禁从单一 TOML 读取门限，已纳入 `check:fast`。临时 Git 目录验证达到门限的 warning/error、CRLF/LF/孤立 CR/BOM、未跟踪新文件、工作树修改、已删除文件、生成排除及未知配置键的 exit 2。当前基线仍有两份超限 Rust integration test 与旧研究 runner；长度拒绝不建立豁免。

## 配置规范化与身份

改动前的 owning Rust 检查实际复现了两种身份差异：只更改 descriptor exposure 改变 registry identity；同一 key 集合改变排列或增加重复项会改变 subset identity。Core 的真实 `video` 配置在省略默认字段时，启动交接中的 JSON 与显式 defaults 不一致。

修复后 Rust owner 检查确认 exposure-only 的 Catalog identity 会变，effective identity 不变；subset 按集合计算。Rust typed policy 与 Core Zod policy 在进入 snapshot 前规范化，不采用通用 JSON Schema defaults 解释器。

使用新构建的当前 Kernel 和源码 Core 运行隔离 configuration smoke：同一 operation ID 的 `video={max_frames:4}` 与完整显式 defaults 回放得到同一 revision/desired digest；desired 读回完整 owner 值，active 在 RestartProcess 前仍为原值。停止并重新启动后 active 读到该完整配置。公开 CLI config check/list、scope precedence、回放和 restart 同时通过。该临时实例没有模型调用，退出时停止并清除；既有 Dogfooding 实例保持运行。

[返回研究入口](README.md)
