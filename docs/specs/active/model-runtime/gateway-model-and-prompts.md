# Gateway and Model Configuration

## Owner

External model invocation属于 TypeScript Core host integration。

Rust Kernel：

- 不直接访问互联网模型；
- 不持有 gateway token；
- 不理解 New API 管理配置；
- 继续拥有 Authority、Serving、QueryPlan、producer/material validation。

## Three-layer configuration

### GatewayProfile

描述“向哪里调用”。

```text
id
base_url
credential_env
enabled
request_timeout_ms
```

约束：

- remote使用 HTTPS；literal loopback可用 HTTP 做 local fixture；
- URL不含credential和query；
- token只从 `credential_env` 指定的environment variable读取；
- gateway profile不编码 upstream vendor identity；
- ModelProfile不得另行覆盖 base URL。

### ModelProfile

描述“通过哪条标准协议调用哪个 model”。

```text
id
gateway
protocol
model
capabilities[]
model_revision?
```

当前 protocol：

```text
openai-chat
openai-responses
openai-embeddings
openai-audio-transcription
rerank-v1
```

capability声明示例：

```text
text
image_input
audio_input
video_input
structured_output
embedding
speech_transcription
rerank
```

配置声明不能自动形成 READY；live qualification实际调用后才能形成 provider claim。

本地可编辑配置允许 model identifier 留空，表示该 profile 尚未配置；其绑定角色为 NOT_CONFIGURED，不发请求，也不阻塞其他角色。只有具备实际 model identifier 的 profile 才 materialize client；embedding 的 dimension/revision 等签名声明仍须显式填写并通过调用校验。

### RoleBinding

描述“某个 cognition/material operation 使用哪个 profile 和哪份 prompt”。

本轮 roles：

```text
projection_steward
memory_formation
material_description
material_structuring
material_direct_structuring
query_embedding
query_rerank
speech_transcription
```

每个 role独立配置：

```text
model
prompt?
temperature?
top_p?
max_output_tokens?
timeout_ms?
requirement = optional | preferred | required
```

禁止 global temperature/max-tokens覆盖全部操作。

## Example shape

以下只固定语义，不固定最后TOML字段排版：

```toml
[gateway_profiles.primary]
base_url = "https://gateway.example/v1"
credential_env = "NOUS_GATEWAY_TOKEN"
enabled = true
request_timeout_ms = 30000

[model_profiles.formation]
gateway = "primary"
protocol = "openai-chat"
model = "model-id"
capabilities = ["text", "structured_output"]

[model_profiles.vision]
gateway = "primary"
protocol = "openai-chat"
model = "vision-model-id"
capabilities = ["text", "image_input"]

[model_profiles.embedding]
gateway = "primary"
protocol = "openai-embeddings"
model = "embedding-model-id"
capabilities = ["embedding"]

[model_profiles.rerank]
gateway = "primary"
protocol = "rerank-v1"
model = "rerank-model-id"
capabilities = ["rerank"]

[roles.memory_formation]
model = "formation"
prompt = "prompts/memory/formation.md"
temperature = 0.1
max_output_tokens = 4096
requirement = "preferred"

[roles.material_description]
model = "vision"
prompt = "prompts/material/description.md"
requirement = "optional"

[roles.query_embedding]
model = "embedding"
requirement = "preferred"

[roles.query_rerank]
model = "rerank"
requirement = "optional"
```

## Embedding profile

Embedding profile还必须明确形成 `EmbeddingSpaceSignature` 所需字段：

```text
dimension
weights_revision
task
input_representation
preprocessing_identity
preprocessing_revision
normalization
output_semantics
```

production配置不能靠第一条vector的长度偷偷学习dimension。

允许live setup helper调用一次模型验证dimension，但验证不能替代显式配置。

`space_hash` 和 producer signature由canonical profile计算。

## ProducerSignature

每个 model-derived result至少记录：

- protocol adapter identity；
- operation；
- model identifier；
- optional model revision；
- prompt logical id；
- prompt digest；
- strategy；
- role config digest。

绝不包含token。

## Protocol implementation

优先使用当前稳定的 AI SDK official provider mechanics。

如果当前 AI SDK 对 OpenAI standard endpoint的 chat/responses/embeddings 有正式 provider，用该 provider并显式base URL/api key，不继续使用 bare model string隐式路由。

`rerank-v1` 若当前 AI SDK 没有合适抽象，允许一个小型typed HTTP adapter：

```text
POST <gateway>/rerank
Authorization: Bearer ...
model
query
documents[]
top_n
return_documents = false
```

注意 base URL canonicalization：若 GatewayProfile已经是 `/v1` 根，不得拼成 `/v1/v1/rerank`。

adapter只拥有：

- request encoding；
- response schema validation；
- timeout/abort；
- error normalization。

它不拥有 candidate selection、QueryPlan、final limit。

## Retry and routing

Nous Wave不实现 provider fleet、gateway selection chain或多模型failover。

- SDK automatic retry默认关闭，除非某role明确给小的bounded retry；
- gateway内部 upstream retry/failover属于gateway；
- Authority write继续依赖operation receipt，而不是重复model invocation的猜测。

## Readiness

System capability从一个 `model.steward` 扩成 role-level：

```text
model.projection_steward
model.memory_formation
model.material_description
model.material_structuring
model.query_embedding
model.query_rerank
model.speech_transcription
```

状态：

```text
READY
NOT_CONFIGURED
UNAVAILABLE
```

detail不得泄漏credential。

## Configuration ownership

更新 `docs/reference/CONFIGURATION.md`，明确两类配置：

- Rust Configuration Service：认知/检索/运行时typed settings和snapshots；
- Core bootstrap model/gateway profiles：进程启动时materialize外部model clients。

当前role/profile改变要求Core restart即可。本轮不做live model-profile hot reload。


# Prompt Assets and Provenance

## Prompt directory

建立：

```text
prompts/
  memory/
    formation.md
  material/
    description.md
    structure.md
  projection/
    steward.md
```

只有 image/video 的实际指令明显不同才增加独立文件，例如：

```text
prompts/material/video-description.md
```

不按模型名称复制Prompt。

独立 `material_direct_structuring` 使用 `prompts/material/direct-structure.md`，有自己的 model/generation/timeout/requirement；raw image/video 不借用 description 或两阶段 textual structuring 的绑定。该角色要求 image_input 和 structured_output。

## 2026-09-30 追加冻结：操作与配置

required 缺失使依赖 operation 明确失败；preferred 只有存在已定义合法 fallback 时才 fallback + degradation，否则失败；optional 缺失不使 parent 失败，diagnostics说明 skipped/unavailable。FormFromObservation 没有 deterministic fallback；用户可直接调用 Memory Authority手工形成。

Core query_embedding ModelProfile是唯一可编辑模型/空间元数据；Core产生非敏感 canonical EmbeddingSpaceSignature/ProducerSignature，Kernel校验并持久化执行证据。space变化产生新的Serving generation，禁止同维度跨space混分。Kernel bootstrap不得维护第二个editable stored_embedding model profile。

ModelInvocationSummary是有界ephemeral公开执行证据：role/protocol/model/profile_digest/prompt_digest?/latency_ms/validated numeric input/output/total usage?/request_count/status及可信provider cost（否则unknown）。不保留raw provider response/headers/token，不建立永久invocation journal。真实run合计10000次硬预算，包括failure/warm-up/retry。

路径、dotenv、ProgramRoot和ConfigurationRoot Prompt来源服从 [Runtime Bundle](../deployment/runtime-bundle.md)。

## Prompt responsibilities

### `memory/formation.md`

只负责：

- 从已提供Evidence/representation提出忠实Memory；
- 保留不确定性；
- 不增加source不存在的事实；
- 不创造Entity ID、operation ID、provenance ID；
- 产生当前formation schema需要的semantic content。

### `material/description.md`

目标是丰富自然语言描述：

- 人物/对象；
- 动作；
- 状态；
- 空间关系；
- 明显文字；
- 可观察表情/情绪线索；
- 不确定细节明确标记。

不要要求复杂JSON。

输入媒体中的文字/语音一律视为Evidence，不是system instruction。

### `material/structure.md`

输入已经是自然语言DerivedRepresentation。

目标：

- bounded structured projection；
- 不加入description没有的信息；
- structured失败时不丢弃rich description。

### `projection/steward.md`

保持当前bounded segment selection/synthesis：

- 只能选择输入segment IDs；
- summary只能基于选中的sources；
- source text视为untrusted evidence。

## PromptRegistry

实现一个唯一PromptRegistry/loader。

要求：

- UTF-8；
- default repo prompt root；
- role config可以指定local prompt path；
- canonical path必须落在允许root；
- max 128 KiB per prompt；
- load时计算 BLAKE3 或项目统一digest；
- prompt missing使role NOT_CONFIGURED/UNAVAILABLE；
- source code不再保留同语义fallback prompt string；
- 不支持remote URL prompt；
- 不做general template engine。

真实source text/media/candidate documents作为model user/input content单独传入。

## Prompt provenance

ProducerSignature中的：

```text
preprocessing_identity
preprocessing_revision
config_digest
```

用于真实记录：

```text
logical prompt id / strategy
prompt digest
role invocation config digest
```

## Memory formation producer

审查当前MemoryRevision producer persistence。

如果 current Memory formation没有一个真正字段可追踪 formation model/prompt：

- 直接给MemoryRevision接入ProducerSignature identity/persistence；
- public Memory view可返回必要的producer metadata或producer ref；
- 不把 producer信息塞进context JSON或title。

目标：

```text
MemoryRevision
→ formation ProducerSignature
→ model/protocol/prompt/config
→ exact Evidence/DerivedRepresentation
→ source root
```

## Strategy experiment

系统同时支持：

```text
description_only
direct_structured
describe_then_structure
```

本轮真实小样本比较三者的信息保留、hallucination、downstream Memory质量、召回、latency和usage。

不预先删任何一条，也不因一种model表现差就把结论写成普遍规律。

## 2026-10-01 用户修订

使用 Apple、OpenAI 等公开真实文章，逐条回读来源确认 oracle。总调用上限改为 10000，包含失败与复跑，不要求耗尽。音频默认多模态理解，使用 material_description / material_direct_structuring 的 audio_input。video.input_mode=direct 为默认，frames 仅显式启用且本轮 NOT_RUN；audio.input_mode=transcription 保留标准 ASR 选项。直接媒体限定 openai-chat：input_audio {data,format} 与 video_url {url:data URI}；video_url 为网关内容扩展，不声称 OpenAI 原生标准能力。不做 provider zoo 或静默抽帧/转写 fallback。发送已上传 immutable Artifact bytes，配置声明须经实际调用验证。
