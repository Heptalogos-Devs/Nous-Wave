# Gateway、Model 与 Prompt

## Owner

TypeScript Core owns external model invocation and client materialization. Rust Kernel owns Subject Authority, query/serving semantics and validation of persisted producer/material identity.

## Profiles 与 roles

GatewayProfile 描述 endpoint、credential environment variable、enabled state 与 request timeout。Remote endpoint 使用 HTTPS；literal loopback 可使用 HTTP。Endpoint 不包含 userinfo、query 或 fragment。Token 只从配置引用的环境变量读取，不进入 ProducerSignature 或公开输出。

ModelProfile 描述 Gateway、standard protocol、model identifier、declared capabilities 和可选 model revision。Supported protocols:

```text
openai-chat
openai-responses
openai-embeddings
openai-audio-transcription
rerank-v1
```

Capabilities 包括 text、image_input、audio_input、video_input、structured_output、embedding、speech_transcription 与 rerank。角色绑定 model profile、Prompt、generation parameters、timeout 与 `optional | preferred | required` requirement。角色包括 projection_steward、memory_formation、material_description、material_structuring、material_direct_structuring、query_embedding、query_rerank、speech_transcription、episode_segmentation、journal_synthesis 与 memory_consolidation。全局 temperature 或 max-token override 不存在。

角色只有在其 profile、gateway、credential reference、Prompt 和本地 prerequisites 足以发起调用时才为 READY。未完整配置为 NOT_CONFIGURED；缺少本地 prerequisite 为 UNAVAILABLE。HTTP、timeout、cancellation 与输出校验结果由对应 operation 返回。optional/preferred fallback 由该 operation 定义；required role 不可用时 operation 失败。

Embedding profile 显式声明 dimension、weights revision、task、input representation、preprocessing identity/revision、normalization 与 output semantics。Core 据此形成 EmbeddingSpaceSignature；Kernel 持久化 producer identity 并以 embedding-space identity 隔离 Serving generations。

Model profiles 与 Prompt 在 Core 启动时解析；配置变更在 Core restart 后生效。SDK automatic retry 关闭；operation 的重试语义由调用方 operation identity 与其 owner 合同定义。

## Prompt registry

PromptRegistry 从 ProgramRoot/prompts 读取默认 Prompt；配置中的 `config-prompts/` 路径引用 ConfigurationRoot/prompts。文件必须位于对应 root、是非空 UTF-8 文本且不超过 128 KiB。Prompt 内容 digest 与 logical id 进入 ProducerSignature；Prompt 资产缺失或无效时该角色不可执行。Prompt 不支持 remote URL 或通用模板语言。

各角色 Prompt 只包含该任务的指令。来源文本、媒体、Evidence 与 candidate documents 以独立 input content 传入，并保持 untrusted input 身份，不能成为 system instruction。

## Structured Contract Registry

Core 的 `model/schemas/contracts.ts` 按 role 唯一绑定 model-facing Zod owner，拥有 `projection.steward`、`memory.formation`、`material.interpretation`、`episode.partition`、`journal.synthesis` 与 `memory.consolidation` 合同。两种 material structuring role 共享同一合同。Production generation 不接受调用方提供另一 schema；SDK 与 direct-media raw response format 使用相同 provider name 和由 Zod 派生的 draft-7 JSON Schema，其 canonical digest 随 producer 保存。

Schema title/description 说明字段语义；任务策略由 Prompt 拥有。Formation 的 title 与 Steward 的 summary 用 required nullable scalar 表达缺省状态。`inspect:model-contracts` 从同一 registry 和 PromptRegistry 导出实际合同、Prompt 与无敏感信息的角色身份，生成文件只保存在 ignored research data。检查命令及 research gateway trace 见 [开发脚本](../../../../scripts/README.md#模型合同与-trace-检查)。

## 输入语义与 Steward

Formation 使用 evidenceText、resolvedEntityCandidates、aboutnessMode envelope。证据、source dates 和作者观点保持其来源归属。Steward 只取得 policy-filtered segment id/role/text，合同为 task-agnostic faithful compression；没有 task intent 时不声称按 current consumer 任务选择相关性。Memory contribution 在 Kernel owner materialize，并携带 evidence 与实际 revision，受文本预算和领域可访问性规则约束。

## Producer identity

ProducerSignature 标识实际 adapter/protocol、operation、model identifier/revision、Prompt logical id/digest、strategy 与 role configuration digest。Credential、token 与 provider response body 不属于 producer identity。

模型输出形成派生表示或带来源的候选提案；Material 与认知领域 owner 验证并提交，由领域 Authority 持有认知身份与修订。

音频、视频和结构化 Material 的输入模式与提交语义见 [Material Derivation](material-derivation.md)；rerank 的候选、预算与 Authority revalidation 见 [Query/Rerank](../retrieval/rerank.md)。

[返回当前产品合同](../../INDEX.md)
