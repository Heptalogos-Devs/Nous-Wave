# Nous Wave TypeScript Client

[返回目录](../../INDEX.md)

`@nous-wave/client` is the official typed TypeScript Client for the public Core API. It wraps Connect transport and generated Protobuf types; it is not a separate domain Authority.

- [Package manifest](package.json)
- [Current API reference](../../docs/reference/NOUSQL.md)
- [Protobuf source](../../proto/README.md)

Node consumers use `connectNousInstance({ runRoot })` from `@nous-wave/client/node` for authenticated local discovery. The returned client includes `artifacts.uploadFile(subjectId, path, { mediaType })` and `artifacts.uploadBytes(subjectId, bytes, { mediaType })`. File upload streams with backpressure and an exact multipart length; credentials stay inside the transport. Request options support cancellation and an upload timeout (default 300 seconds).

`client.cognition.prepareQuery({ subjectId, nousql, sessionId?, workContextId?, situation? })` 只准备/inspect，返回 `boundQuery` JSON（resolved query、complete representation、source refs、SHA256、profile、ConfigSnapshot digest），不调用 provider 或构建 Serving。`cognition.query` 共用该 preparation，内部固定 token 后生成一份 query embedding。正式 TextCue 拒绝未闭合指代，Identity/Tag 名称继续用现有 Directory resolver。

Research raw-text compatibility 使用 typed standalone expression 与 `textOnlyCompatibility: true`，不带 cognitive context，独立报告结果。

QueryRequest 的 typed `capabilities` 传递 text embedding、multimodal interpretation、residual sensing 与 rerank requirement。`rerank: "forbidden"` 明确关闭 model rerank，适用于 deterministic algorithm/Agent wiring run；`text_embedding: "forbidden"`（Client 为 `textEmbedding`）禁止 embedding provider。Required 需求的失败不静默回退。

Tag owner 操作使用 `concepts.reviseTag/mergeTags/splitTag`。所有 target 带 `tagId/expectedRevisionId`，mutation 带 `operationId/subjectId`；merge/split 带 exact cognition `RevisionSupport`。Tag 响应包含 `currentRevisionId/status/canonicalTagId`。merge 保持 survivor，split 返回 child Tags；历史关联不会被批量改写。

Concept maintenance 的 model suggestion list 由 Core 逐项调用 canonical Tag/Association owner APIs；Client 没有 batch concept/consolidation envelope。Tag lineage supports 接受 owner 验证的 Evidence 或 cognition revision；create/revise/split/association 可携带 structured ProducerSignature。每项 mutation 的稳定 operation ID 保证恢复时精确 replay 已提交结果。
