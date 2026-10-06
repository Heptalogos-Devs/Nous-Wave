# Nous CLI

第一方 reference consumer，只使用 `@nous-wave/client` 和 Node 标准库。Core 的本地 discovery 文件提供连接信息，CLI 只保存当前 Subject/Session/WorkContext 的选择。

先按[根 README](../../README.md)安装依赖、构建 Kernel，并显式运行 `just dev-prepare` 准备开发数据库；再运行 `corepack pnpm dev`。另一个终端：

```powershell
corepack pnpm nous status
corepack pnpm nous subject create
corepack pnpm nous session open
corepack pnpm nous observe text --text "实际来源中的有界原文" --source "https://example.com/source"
corepack pnpm nous observe file ./sample.png
corepack pnpm nous derive <source-region-id> --strategy describe_then_structure
corepack pnpm nous form <occurrence-id>
corepack pnpm nous embeddings prepare --max-batches 16
corepack pnpm nous query '"检索线索" $memory $limit(5)'
corepack pnpm nous trace memory:<memory-id>
corepack pnpm nous use memory_revision:<revision-id>
```

用 launcher 的 `--home <path>` / `--locator <bootstrap.toml>` 选择实例，用 `--json` 输出机器可读 JSON；int64 用十进制字符串。Consumer 接收独立 RunRoot/InstanceRoot，不推测 DataRoot。`subject use <id>` 和 `session show/close` 管理当前选择；`context create --text <purpose>`、`context foreground/show/end` 操作有界 WorkContext。

文件上传使用有界 stream，不整份读取到内存。`--media-type` 可以明确 MIME；`--source` 保存可追踪外部来源。Formation/embedding 需要 Core 的模型角色；凭据仅从配置指定的环境变量读取。当前功能见 [Current State](../../docs/current-state/CURRENT_STATE.md)。

## Agent Tool

`nous help --json` 返回命令、参数和示例；成功 payload 只写 stdout，错误只写 stderr，返回非零退出码。错误含 `code`、`message`、`details`、`candidates`；身份歧义保留候选，闭合失败保留 offending spans。

`--subject`、`--session`、`--work-context` 覆盖本地选择。直接 consumer 调用可只提供 `--run-root` 和显式 Subject；修改本地选择仍需要 `--instance-root`。Query 将选中的 Session 和 WorkContext 一并传给官方 Client。

```text
nous identity resolve --kind entity --name Alice --json
nous identity resolve --lexical-ref ent:<four-words> --json
nous identity bind --kind entity --canonical entity:alice --name Alice --alias A --json
nous tag list --page-size 50 --json
nous tag get tag:<uuid> --json
nous tag search deployment --json
nous tag resolve deployment --json
nous topology neighborhood memory_revision:<uuid> --max-nodes 64 --max-depth 2 --json
nous query prepare '"deployment decision" $memory' --subject <id> --session <id> --work-context <id> --json
nous query inspect --query-file closed-query.nousql --json
```

`query prepare/inspect` 共用公共 PrepareQuery，只返回已绑定 query、canonical representation/digest、source refs、exact/context/topology seeds、profile 和 ConfigSnapshot；`boundQuery` 展开成 JSON object；不执行 retrieval 或模型调用。普通 `query` 使用同一准备协议后执行。

Tag list/search 使用服务端有界分页（默认 50、最大 200）；`--page-token` 继续同一查询。Search 按 label、description 和 Directory display/aliases 匹配，resolve 仍要求 Directory 唯一身份。Neighborhood 是 active AssociationEvidence 的双向有界读取，返回真实 support；默认 64 nodes、2 hops，最大 256 nodes、4 hops、256 edges，达到节点/边上限时报告 `truncated`。隐式 tag attachment/Aboutness 等结构由 Serving topology lane 读取。

`nous topology associate --operation-id <uuid> --association-file association.json --json` 接收官方 Client 的 Association JSON（`from/to` 的 `{kind,value}`、`relationKind`、`polarity`、`supportClass`、`supports`、可选 producer）。CLI 要求显式 operation id 和非空支持；Authority 验证 exact endpoint、Subject 和支持有效性。文件使用 Client 的 discriminated `support: {case,value}` 格式，不能用自由 UUID 替代支持材料。

实际 Agent flow smoke 已在 public Core 上验证 help JSON → ambiguous identity candidates → 选定 LexicalRef → query prepare → query。正式 TextCue 的 closure guard 只拒绝高置信未闭合指代；完整语义输入和配置 snapshot 在 preparation 固定。维护操作使用 bounded grant，概念建议按项提交到 owner。
