# Model Runtime 当前参考

Core 启动配置用 `GatewayProfile → ModelProfile → RoleBinding` materialize 标准协议 clients。凭据只从 gateway 的 `credential_env` 读取。Kernel 保存经过规范化的 ProducerSignature 与不可变输入，不调用外部模型。

协议名称为 `openai-chat`、`openai-responses`、`openai-embeddings`、`openai-audio-transcription`、`rerank-v1`。SDK generation、embedding 和 transcription 使用显式 endpoint/model；rerank 使用有界 HTTP adapter。SDK retries 为 0，远程 destination 需要无 credential/query 的 HTTPS；literal loopback 可用 HTTP。

各角色分别绑定模型、Prompt、参数、timeout 和 requirement。角色具备完整可执行配置时报告 READY；未完整配置为 NOT_CONFIGURED，本地 prerequisite 缺失为 UNAVAILABLE。HTTP、timeout 和输出校验失败由当次 operation 返回。

Prompt 从仓库 `prompts/` 的 UTF-8 Markdown 加载；custom path 须落在允许 root，每份最多 128 KiB。logical id、内容 digest、role config digest 与实际 model/protocol 进入 producer。Prompt asset 缺失或无效时对应 role 不可执行。

## Material 与 formation

`client.model.deriveMaterial({ subjectId, sourceRegionId, strategy, target, supersedes })` 返回实际 committed `representations[]`、selected representation 和 degradation。Configuration 的 `material.strategy` 默认 `description_only`，还接受 `direct_structured`、`describe_then_structure`。

- Text-like source：验证 UTF-8 后提交 ExtractedText，不调用模型重写。
- Image：`material_description` 生成 rich ImageDescription；direct structured 使用独立 `material_direct_structuring` model/Prompt/参数。两阶段使用 description 的 exact ref 作为 structuring input。
- Audio：`input_mode=direct` 默认发送多模态 `input_audio` 并形成 AudioDescription；direct_structured 与两阶段使用对应的 material roles。`input_mode=transcription` 使用 speech_transcription 形成 Transcript；该模式不支持 direct_structured，describe_then_structure 可继续结构化 Transcript。
- Video：默认将原始 Artifact 通过 gateway chat 的 video_url 扩展发送；frames 是已实现的显式 FFmpeg 模式，使用有界抽帧与可选音轨转写。观测情况见 [Research](../research/README.md)。

结构化角色通过 `model/schemas/contracts.ts` 唯一选择对应 Zod owner；Material 合同由 `model/schemas/material-interpretation.ts` 拥有，SDK 与 raw strict 请求共用同一派生 JSON Schema，outputSchemaDigest 进入 producer 与 derivation identity。自由描述不使用结构化 envelope；结构化表示保存一等 `structuredPayload` 和保留 basis/uncertainty 的 deterministic text projection。

两阶段先提交描述，再由 Material owner 生成 description_segment DerivedRegion（UTF-8 byte span）。第二模型只输出 invocation-local support keys；提交前映射成 payload 中的 stable `supports` refs，Kernel 核验它们属于输入图。summary 和内容项都保存 stable supports；basis、certainty 与 evidence_channel 分开；证据通道独立于所陈述的 event/state/action，original source text 与视觉 embedded text 分开。字段可通过 `client.material.derivedRegion` 与 `client.material.materialize` 精确回读，支持链可回溯原始 SourceRegion/Artifact。

Structuring 失败保留已提交的 description。普通 recall 不重新解释媒体。显式 `supersedes` 是更新 lineage；同一 lineage/input/producer/strategy request 复用成功结果。

`client.model.formFromObservation({ operationId, subjectId, occurrenceId, representationId?, aboutnessMode?, explicitAboutness? })` 使用指定表示，或按实际媒体策略选择最新描述/转写。operationId 由调用方稳定提供；同 ID/input 重放 outcome，不受后续配置变化影响，不同 input conflict。未解释的 binary source 返回 `material_representation_required`。Aboutness 支持 explicit、select_from_resolved_mentions（默认）与 none；select 模式只使用 Kernel 提供的 resolved mention candidate keys，Actor identity 单独保存于 Observation。

Memory revision 的 `producerSignatureId` 可用 `client.material.producer` 读取。Material 的表示、inputs、DerivedRegion、SourceRegion、Artifact 都有同 Subject 的 public read path。`nous trace` 只经 official Client 展开这条链，保存的 producer 不包含 token。

使用 `corepack pnpm inspect:model-contracts --all` 导出实际 provider schema、Prompt 与 digest；通过 research gateway 的 `--trace-root` 和 `inspect:model-trace` 检查真实 New API wire attempt。命令详见 [开发脚本](../../scripts/README.md#模型合同与-trace-检查)。

完整配置与语义合同见 [Gateway/model/Prompt Spec](../specs/active/model-runtime/gateway-model-and-prompts.md)、[Material Spec](../specs/active/model-runtime/material-derivation.md)。


## 纵向维护角色

`episode_segmentation`、`journal_synthesis`、`memory_consolidation` 使用 text + structured_output profile 和 canonical Zod schema。Core 的 `client.cognition.grantMaintenance({ subjectId, maxOperations, maxModelCalls, maxElapsedMs })` 执行有界机会，Rust owner 验证并提交模型 proposal。Core 在 claim 前只允许可执行角色对应的 maintenance kinds；未配置角色不取得 lease 或增加 attempt count。临时 provider 故障使用 Configuration Service 的有界指数退避。角色配置、Prompt 和超时沿用现有 ModelProfile/RoleBinding 机制。数据身份与重试语义见 [纵向认知合同](../specs/active/cognitive-runtime/longitudinal-cognition.md)。

## 输入与投影

Formation envelope 的 `evidenceText` 是原始来源或已提交表示正文，`resolvedEntityCandidates` 是可选择实体目录，`aboutnessMode` 由 owner 决定。作者观点保留作者归属，观察者身份不自动成为 aboutness。Material text structuring 使用 `evidence_text` 和 `evidence_kind` 区分 original_text 与 committed_representation；后者不提供原始媒体访问。

Projection Steward 接收经 consumer policy 筛选的 id/role/text，执行无任务上下文的忠实压缩。Kernel contribution owner 经现有 ContextResolver materialize Memory 内容，遵守请求文本预算、可访问性与 lifecycle，保存实际 revision 和 evidence；Core 在模型前执行 consumer policy。Steward 不承担未提供 query/Focus 的任务相关性判断。

[返回文档目录](../INDEX.md)

`topology_maintenance` 是独立 structured role，默认 Prompt 为 `prompts/topology/maintenance.md`。Role READY 时才进入 maintenance allowed kinds；generation 沿同一固定 role/config/Prompt snapshot、provider-call reservation、durable proposal/receipt 与 lease/retry 路径。模型输入使用局部 cognition/tag/entity/association/support keys，禁止自由 UUID 或 catalog 外 refs；single noisy occurrence、单纯词法重叠和 exposure 不证明长期 concept。`no_change` 单独输出，新增 Tag 与后置 attachment 在同一 proposal 中表达。
