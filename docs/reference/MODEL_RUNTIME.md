# Model Runtime 当前参考

Core 启动配置用 `GatewayProfile → ModelProfile → RoleBinding` materialize 标准协议 clients。凭据只从 gateway 的 `credential_env` 读取。Kernel 保存经过规范化的 ProducerSignature 与不可变输入，不调用外部模型。

协议名称为 `openai-chat`、`openai-responses`、`openai-embeddings`、`openai-audio-transcription`、`rerank-v1`。SDK generation、embedding 和 transcription 使用显式 endpoint/model；rerank 使用有界 HTTP adapter。SDK retries 为 0，远程 destination 需要无 credential/query 的 HTTPS；literal loopback 可用 HTTP。

各角色分别绑定模型、Prompt、参数、timeout 和 requirement。角色只有在实际协议调用和输出校验成功后才报告 READY；配置完成但尚未验证时为 UNAVAILABLE，未绑定时为 NOT_CONFIGURED。此状态不证明真实 corpus recall，live claim 由 Qualification 单独记录。

Prompt 从仓库 `prompts/` 的 UTF-8 Markdown 加载；custom path 仍须落在允许 root，每份最多 128 KiB。logical id、内容 digest、role config digest 与实际 model/protocol 进入 producer。代码没有等价的 fallback system prompt。

## Material 与 formation

`client.model.deriveMaterial({ subjectId, sourceRegionId, strategy, target, supersedes })` 返回实际 committed `representations[]`、selected representation 和 degradation。Core 的 `material_strategy` 默认 `description_only`，还接受 `direct_structured`、`describe_then_structure`。

- Text-like source：验证 UTF-8 后提交 ExtractedText，不调用模型重写。
- Image：`material_description` 生成 rich ImageDescription；direct structured 使用同一 vision model 与独立 structuring Prompt。两阶段使用 description 的 exact ref 作为 structuring input。
- Audio：`speech_transcription` 生成 Transcript，之后可以用 `material_structuring` 形成结构化附加表示。Audio direct structured 没有标准直接输入路径，返回明确 degradation。
- Video：FFmpeg adapter 仍在施工，当前返回 `video_preprocessor_required`；不报告视频 capability PASS。

Structuring 失败保留已提交的 description。普通 recall 不重新解释媒体。显式 `supersedes` 是更新 lineage；同一 lineage/input/producer/strategy request 复用成功结果。

`client.model.formFromObservation({ subjectId, occurrenceId, representationId? })` 用指定表示，或默认选择最新未被 superseded 的 ExtractedText/ImageDescription/Transcript/SceneDescription。未解释的 binary source 返回 `material_representation_required`。Memory support 固定实际采用的 representation；source actor 不自动成为 Memory aboutness。

Memory revision 的 `producerSignatureId` 可用 `client.material.producer` 读取。Material 的表示、inputs、DerivedRegion、SourceRegion、Artifact 都有同 Subject 的 public read path。`nous trace` 只经 official Client 展开这条链，保存的 producer 不包含 token。

## 当前验证边界

Windows local public wiring 已验证文本 derivation、结构化 role 不可用时保留 description、producer persistence、trace 和 restart。Image/audio 的代码路径已有标准 adapter，但 live gateway 尚未提供，结果 BLOCKED。Query rerank integration、video adapter 和真实实验仍待验收；见 [本轮 Qualification](../qualification/2026-09-30-real-usage-retrieval.md)。

完整配置与语义合同见 [Gateway/model/Prompt Spec](../specs/active/model-runtime/gateway-model-and-prompts.md)、[Material Spec](../specs/active/model-runtime/material-derivation.md)。
