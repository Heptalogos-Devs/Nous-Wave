# Reference Consumer

## Owner

`apps/nous-cli` is the first-party reference consumer. It uses the public `@nous-wave/client` package and its Node entry point; consumer behavior is documented in the [CLI README](../../../../apps/nous-cli/README.md).

## Boundary

The consumer connects through Core discovery in RunRoot. It stores the selected Subject, Session and WorkContext under InstanceRoot; these selections are local consumer state, not Nous Wave Authority.

Parallel Agents share Core RunRoot and use independent consumer InstanceRoots. The stdio MCP entry belongs to this same consumer and requires an explicit private state root and stable consumer identity. It exposes help, argv command execution and NousQL query through the official MCP SDK. Terminal and MCP call the same in-process command core and public Client; Terminal alone writes human stdout/stderr and supplies stdin content. MCP stdin remains protocol-only; file `-` input is rejected there.

Consumer state is keyed by stable Core instanceId and consumer identity. Each command freezes its selection, context and input once. Maintained atomic-write and cross-process lock libraries protect only short local state transactions; RPC and provider execution remain concurrent. Selection patches merge independent fields and reject conflicting fields or changed Subject/query context. A successful business operation with a failed selection save returns its result and `STATE_UNSAVED`; the returned exact references remain usable. The `consumer_state` Catalog policy defines receipt capacity, file budget and lock timing and is frozen from active configuration at command start.

Current selection and operation records each carry one format identifier and use a codec that preserves BigInt separately from arbitrary JSON payload keys. The current consumer does not load superseded records. Unknown old requests and their operation associations must be preserved before obsolete state cleanup. Within a current receipt, the frozen request cannot be replaced and a terminal outcome cannot regress to pending or another terminal outcome.

Material references returned by observation/query/trace are accepted as exact canonical references. `show` reads metadata; `read` materializes bounded source/derived text through Material Authority, preserving actual range, total bytes, partial output and evidence. Binary sources direct the caller to derivation instead of rendering raw bytes as text.

`session show [reference-or-name]` reads explicit Session metadata, including closed Sessions, without selecting or reopening Runtime activity. With no argument it reads the consumer-selected Session. Close clears local Session selection but preserves the closed identity for this read. Open and close do not accept a reference argument.

Consumer operations use public Client APIs for Subject and Session lifecycle, Observation, Material derivation, Memory formation, embedding preparation, query, trace, UseEvent and WorkContext. File upload streams Artifact content through the authenticated Core endpoint.

The consumer uses citty 0.2.2 command families, Zod-validated semantic TOML inputs, default semantic text and opt-in versioned JSON. Raw Client DTO diagnostics require --raw --developer. Durable local state saves exact result:N references and operation receipts before RPC; retry replays frozen operation inputs and expected revisions. Context set/pin/unpin/clear/pause/resume/select/foreground use public WorkContext APIs. The consumer is built against the public Client surface. Provenance trace follows public Material and exact Memory/Schema/Episode/Journal revision reads from a revision through its producer and exact evidence references to the source Artifact.

明确拒绝的 RPC 保存 `rejected` 回执，与成功 `complete` 一样可回收；只有结果未知的 `pending` 保留原 request/operation identity 供恢复。`retry` 读回成功回执中的结果而不重复业务调用，已拒绝回执返回 `OPERATION_REJECTED`。Subject 创建的恢复不要求预先选中该新 Subject；其他 Subject-bound 操作继续校验所属 Subject。

正文、title、摘要和操作消息逐字保留，包括完整 UUID 或 `memory:<UUID>`。引用呈现按 typed reference 字段转换。业务已成功而地址呈现失败时，输出原 canonical result 并附 `PRESENTATION_UNAVAILABLE` notice；本地完成回执写入失败则附 `RECEIPT_UNSAVED`，保持真实已知结果。

[返回当前产品合同](../../INDEX.md)
