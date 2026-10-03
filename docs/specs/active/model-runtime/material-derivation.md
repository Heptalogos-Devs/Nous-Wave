# Material Derivation

## Owners

TypeScript Core orchestrates external model calls and operation snapshots. The Material owner validates and persists exact input graphs, representations, regions and provenance.

## Identity 与 provenance

Artifact、ObservationOccurrence、SourceRegion、DerivedRepresentation 与 DerivedRegion 各有独立 identity。相同 Artifact 的重复观察保留独立 occurrence identity。

DerivedRepresentation 保存 ordered exact inputs、strategy、representation kind、ProducerSignature、revision、payload 与 quality。Inputs 可引用 SourceRegion、DerivedRepresentation 或 DerivedRegion，必须属于同一 Subject，不得重复、自引用或形成 cycle。已提交 input set 不可修改。Supersedes 表示显式 refinement lineage，不属于 derivation inputs。

成功 derivation identity 由 Subject、ordered input digest、representation kind、producer signature、strategy 和 supersedes 组成。同一请求重试复用成功表示；改变 lineage 或 inputs 会得到新的 identity。

## Supported derivation

- Text-like source 经严格 UTF-8 解码形成 ExtractedText，不调用模型重写原文。
- Image 可形成 ImageDescription、StructuredInterpretation，或按 describe_then_structure 先提交描述再结构化。
- Audio 默认使用 direct 多模态理解形成 AudioDescription。audio.input_mode 为 transcription 时使用独立 speech_transcription role 形成 Transcript；该模式不支持 direct_structured，describe_then_structure 可继续结构化 Transcript。两种 mode 之间不静默切换。
- Video 默认以原始 Artifact bytes 通过 gateway chat 的 video_url input 形成 SceneDescription。显式 frames mode 使用 FFmpeg 作有界抽帧，可选提取音轨并调用 speech_transcription。处理记录包含 FFmpeg version、采样策略、时间戳、source duration 与 truncation。
- direct_structured 使用 material_direct_structuring role；describe_then_structure 先持久化 description，再以 exact DerivedRegion segments 作为第二阶段输入。

素材策略为 description_only、direct_structured 与 describe_then_structure。结构化结果使用唯一的 Zod schema owner；schema digest 进入 ProducerSignature 与 derivation identity。DerivedRepresentation 可保存 structured JSON payload 和 deterministic text projection。每个结构化 support 都映射到输入 DAG 中可读回的 exact reference。

description segmentation 以 UTF-8 byte coordinates 建立稳定 DerivedRegion。structured model 只返回 invocation-local support keys；Core 映射成 exact DerivedRegion，Material/Kernel 验证归属和输入图。第二阶段失败时保留已提交的 description，并返回该表示及显式 degradation。

description segments 使用 `material.description_segment_bytes` 的稳定 UTF-8 范围，默认 2,048 bytes，并优先在换行处分段。region coordinates 保存 segmentation policy digest。Text segmentation 输入上限为 1 MiB，最多 512 个 DerivedRegion。Video frames mode 使用 FFmpeg 作有界抽帧；实验观测归 [Research](../../../research/README.md)。

## Memory formation

`formFromObservation` 使用调用方提供的稳定 operation_id。初次执行固定 model/profile/Prompt/config snapshot，并将语义输入绑定到 operation identity；相同 operation 与相同输入重放原 outcome，相同 operation 携带不同输入返回 conflict。

调用方可指定 exact representation_id。未指定时，Core 按 Artifact modality 和配置选择可用 representation：text 用 ExtractedText，image 用 ImageDescription，audio 用 AudioDescription 或 Transcript，video 用 SceneDescription。没有可用 textual representation 的 binary input 返回 `material_representation_required`。

Aboutness 支持 explicit、select_from_resolved_mentions 与 none。select 模式使用 Kernel 已解析的 mention candidate keys，由 Model 选择子集；显式引用由 owner 验证。Actor identity 单独保存在 Observation。

形成结果引用实际使用的 occurrence 与 DerivedRepresentation。Material read/materialize 可按 SourceRegion、DerivedRegion 或 representation 精确回读来源链。普通 recall 不重新解释媒体。

[返回当前产品合同](../../INDEX.md)
