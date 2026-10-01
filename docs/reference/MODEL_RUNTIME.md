# Model Runtime 当前参考

Core 启动配置用 `GatewayProfile → ModelProfile → RoleBinding` materialize 标准协议 clients。凭据只从 gateway 的 `credential_env` 读取。Kernel 保存经过规范化的 ProducerSignature 与不可变输入，不调用外部模型。

协议名称为 `openai-chat`、`openai-responses`、`openai-embeddings`、`openai-audio-transcription`、`rerank-v1`。SDK generation、embedding 和 transcription 使用显式 endpoint/model；rerank 使用有界 HTTP adapter。SDK retries 为 0，远程 destination 需要无 credential/query 的 HTTPS；literal loopback 可用 HTTP。

各角色分别绑定模型、Prompt、参数、timeout 和 requirement。角色只有在实际协议调用和输出校验成功后才报告 READY；配置完成但尚未验证时为 UNAVAILABLE，未绑定时为 NOT_CONFIGURED。此状态不证明真实 corpus recall，live claim 由 Qualification 单独记录。

Prompt 从仓库 `prompts/` 的 UTF-8 Markdown 加载；custom path 仍须落在允许 root，每份最多 128 KiB。logical id、内容 digest、role config digest 与实际 model/protocol 进入 producer。代码没有等价的 fallback system prompt。

## Material 与 formation

`client.model.deriveMaterial({ subjectId, sourceRegionId, strategy, target, supersedes })` 返回实际 committed `representations[]`、selected representation 和 degradation。Core 的 `material_strategy` 默认 `description_only`，还接受 `direct_structured`、`describe_then_structure`。

- Text-like source：验证 UTF-8 后提交 ExtractedText，不调用模型重写。
- Image：`material_description` 生成 rich ImageDescription；direct structured 使用独立 `material_direct_structuring` model/Prompt/参数。两阶段使用 description 的 exact ref 作为 structuring input。
- Audio：默认 direct 多模态输入 `input_audio` → AudioDescription；显式 transcription 才依赖 speech_transcription。direct structured 与两阶段可用，但模型声明须经真实 conformance 验证。
- Video：默认原始 Artifact 通过 gateway chat 的 video_url 扩展发送；frames 是显式 FFmpeg 选项，本轮 NOT_RUN。记录实际音画覆盖，普通 serve 不下载。

结构化角色使用唯一 `model/schemas/material-interpretation.ts` Zod owner，SDK 与 raw strict 请求共用同一派生 JSON Schema，outputSchemaDigest 进入 invocation、producer 与 derivation identity。自由描述不使用结构化 envelope；结构化表示保存一等 `structuredPayload` 和保留 basis/uncertainty 的 deterministic text projection。

两阶段先提交描述，再由 Material owner 生成 description_segment DerivedRegion（UTF-8 byte span）。第二模型只输出 invocation-local support keys；提交前映射成 payload 中的 stable `supports` refs，Kernel 核验它们属于输入图。字段可通过 `client.material.derivedRegion` 与 `client.material.materialize` 精确回读，支持链可回溯原始 SourceRegion/Artifact。

Structuring 失败保留已提交的 description。普通 recall 不重新解释媒体。显式 `supersedes` 是更新 lineage；同一 lineage/input/producer/strategy request 复用成功结果。

`client.model.formFromObservation({ operationId, subjectId, occurrenceId, representationId?, aboutnessMode?, explicitAboutness? })` 使用指定表示，或按实际媒体策略选择最新描述/转写。operationId 由调用方稳定提供；同 ID/input 重放 outcome，不受后续配置变化影响，不同 input conflict。未解释的 binary source 返回 `material_representation_required`。aboutness 支持 explicit、select_from_resolved_mentions（默认）、none；actor 与全部 mentions 不自动成为 aboutness。

Memory revision 的 `producerSignatureId` 可用 `client.material.producer` 读取。Material 的表示、inputs、DerivedRegion、SourceRegion、Artifact 都有同 Subject 的 public read path。`nous trace` 只经 official Client 展开这条链，保存的 producer 不包含 token。

## 当前验证边界

Windows public local qualification 已验证 structured payload、字段 DerivedRegion/read、失败保留描述、无额外模型调用 replay，以及 consumer restart。此项使用 deterministic local provider，liveModel 为 NOT_RUN。已有真实 corpus/model 测量属于历史 candidate，新的 rich schema/live 媒体复核与 final bundle acceptance 尚未执行；见 [本轮 Qualification](../qualification/2026-09-30-real-usage-retrieval.md)。speech 未配置不阻塞 native audio 模式。

完整配置与语义合同见 [Gateway/model/Prompt Spec](../specs/active/model-runtime/gateway-model-and-prompts.md)、[Material Spec](../specs/active/model-runtime/material-derivation.md)。
