# Material Derivation Pipeline

## Goal

真实文本、图片、音频、视频进入 Nous Wave 后先成为Evidence/Material，再形成可追踪的 textual DerivedRepresentation。Memory formation和后续retrieval优先消费textual representation，而不是每次重新处理raw media。

## DerivedRepresentation input graph

将 current singular `source_region_id` ownership改成：

```text
DerivedRepresentation
  derived_representation_id
  subject_id
  representation_kind
  producer
  revision
  payload_text?
  payload_artifact_id?
  quality
  created_at
  supersedes?
  inputs[]
```

Input：

```text
ordinal
reference = SourceRegion | DerivedRepresentation | DerivedRegion
role
```

规则：

- 1..N inputs；
- same Subject；
- exact immutable refs；
- duplicate input reject；
- cycle reject；
- input set immutable；
- `supersedes`表达显式更新lineage，不是derivation input；
- transitive provenance可以回到SourceRegion/Occurrence。

### Persistence

修改 canonical fresh Material schema；推荐关系表 `derived_representation_inputs`。

不要追加一个仅为这次shape变化保留旧schema的额外migration。

### DerivationRecord

如果当前DerivationRecord以 singular source region作为derivation key输入，改成：

```text
subject
canonical input-set digest
representation kind
producer signature
strategy
```

同输入/同producer/同strategy可稳定识别已成功derivation。

当前实现将成功 key 直接保存在 immutable DerivedRepresentation，不另建 scheduler/attempt 状态源。key 包含 Subject、ordered input digest、kind、producer signature、strategy；显式 `supersedes` lineage 作为 refinement discriminator，使同一 lineage request 重试复用成功结果，继续细化时引用上一个实际表示。无 current consumer 的旧 scheduler 删除。

旧singular-only derivation path删除。

## Text

对 `text/plain` 和可靠text-like source：

- verified decoding；
- 可形成 `ExtractedText`；
- 不调用LLM只为了重写一次文本；
- 大文本按稳定SourceRegion coordinate切分；
- 不使用随机chunk ordinal作为Authority coordinate。

## Image

```text
Artifact(image/*)
→ SourceRegion
→ material_description model
→ ImageDescription
```

可选：

```text
ImageDescription
→ material_structuring model
→ StructuredInterpretation
```

`direct_structured`：

```text
image
→ multimodal model
→ StructuredInterpretation
```

若 direct path没有rich description，不伪造一份description。

## Audio

标准默认：

```text
Artifact(audio/*)
→ openai-audio-transcription
→ Transcript
```

可选：

```text
Transcript
→ material_structuring
→ StructuredInterpretation
```

记录language hint（若配置）、model、protocol、prompt/config和usage。

## Video

### Managed or operator-provided FFmpeg

配置：

```text
ffmpeg_executable
max_video_seconds
max_frames
frame sampling policy
max_audio_bytes
```

最终批准的 [Runtime Bundle](../deployment/runtime-bundle.md) 取代本段旧 acquisition 决定：普通 runtime 不下载。完整 multimedia bundle携带LGPL-only FFmpeg pack；显式 `nous runtime install` 可按固定manifest安装。operator executable优先，其次managed pack；仅明确允许的development profile可以PATH lookup。之前Gyan GPL auto-downloader删除。平台只声明实际执行证据。

执行用 `spawn/execFile` + argument array，禁止shell string拼接。

### Decomposition

```text
video
→ bounded frames
→ optional audio track
```

临时frame不要求每一帧永久成为Artifact。

committed SceneDescription的producer/quality metadata至少记录：

- source video region；
- sampling policy；
- sampled timestamps；
- FFmpeg version/invocation identity。

需要长期精确引用某帧时再建立SourceRegion。

### Description

把bounded frame set作为multimodal image input；有transcript时把transcript作为另一个exact input/context。

形成 `SceneDescription`。

可选再形成 `StructuredInterpretation`。

## ModelService public surface

### Whole-workflow formation与付费前reservation

Formation request必须包含caller-stable operation_id。semantic digest包含Subject、Occurrence、显式representation选择、aboutness mode/input；不包含后续配置变化。第一次开始固定resolved model/profile/prompt/config snapshot，不含token。same ID/same input返回原outcome；same ID/different input conflict。

Memory owner保存有界reserved → proposal persisted → Authority committed/outcome状态；lease协调同ID并发，模型在Authority事务外调用。已有proposal/outcome跳过付费调用。provider返回后、proposal落盘前崩溃可能再次付费，但不允许重复Memory。purge receipt不得恢复正文或proposal。

aboutness模式为explicit、select_from_resolved_mentions（默认）、none。explicit refs由owner校验且模型不得修改；自动模式只给已解析候选，模型输出候选key子集，禁止发明EntityRef。actor和all mentions都不自动成为aboutness。

Derivation在付费前计算canonical identity并reserve/check；成功结果直接返回，live lease冲突返回有界busy而不并发重复调用。identity涵盖ordered exact inputs/kind/producer/strategy/prompt/config及显式supersedes。协调只承担当前真实workflow，不恢复已删除通用scheduler/attempt历史shape。description一旦成功立即提交，后续structuring失败保留partial chain。

direct_structured的raw image/video使用独立material_direct_structuring role与direct-structure Prompt，不伪造Description。audio标准路径仍先transcription再textual structuring。

current `InterpretSource`只能表达一次调用和一个representation，不能表达strategy/chain。

替换为能表示derivation的public operation。语义至少包含：

```text
subject_id
source_region_id
strategy?          # configured default when absent
target?            # description / structured / automatic
```

响应：

```text
representations[]
selected_representation_id?
degradation[]
```

要求：

- caller看得到实际committed chain；
- direct和two-stage能区分；
- selected textual representation明确；
- official Client暴露；
- 旧 `interpretSource` current surface删除。

具体Proto message名称按现有命名风格决定。

## Memory formation source selection

扩展 `formFromObservation` 支持 optional explicit `representation_id`。

未提供时：

1. text → verified text / ExtractedText；
2. image → latest accepted ImageDescription；
3. audio → latest accepted Transcript；
4. video → latest accepted SceneDescription；
5. caller明确指定时使用指定representation。

如果binary/non-text尚无可用textual representation，返回明确degradation，例如：

`material_representation_required`

绝不把binary bytes交给TextDecoder碰碰运气。

Memory support：

```text
occurrence_id = source occurrence
locator = exact DerivedRepresentation actually used
support_role = direct | interpretation according to semantics
```

## Explicit refine/update

只有caller明确要求重新解释/细化时执行。

新结果：

- 新 immutable DerivedRepresentation；
- `supersedes`指向旧representation；
- 新 ProducerSignature；
- 原representation保留作provenance history。

普通recall不自动重新interpret旧媒体。

## Material lifecycle scope

本轮不扩大成完整media retention/purge subsystem。

但必须保证：

- Evidence locator不存在时不能静默换raw source；
- materialization失败有明确degradation；
- temp frames/audio和live-run raw media不进入tracked tree；
- secret不会进入representation quality/metadata。
