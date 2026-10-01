# Configuration Service 当前参考

当前配置基础服务位于 `crates/configuration`，由 Kernel 在数据库 migration 完成后打开。它只负责 registry、解析、覆盖、快照、权限、幂等 receipt 和 BLAKE3 digest，不拥有 Memory 的领域语义。

## 解析与能力

有效值按以下顺序解析：reference default → deployment `[settings]` → persisted system override → persisted subject override。每个 key 由 owner 注册 typed descriptor，包含 owner、类型、校验、暴露等级、作用域、应用方式和 semantic effect。

当前暴露等级为 `Developer`、`Advanced`、`Standard`；系统不变量没有配置 key。普通配置 mutation API 目前只在 Rust service 内部提供，未暴露未经认证的公共 RPC。

`ConfigSnapshot` 是 immutable operation input。Query bind、Serving prepare 和 Authority formation 使用一个 Subject snapshot；`digest_for` 只覆盖实际影响该结果的 key subset。`RestartProcess` override 会保存 desired value 并返回 `pending_restart`，当前 active snapshot 保持不变。

当前基础 key 包括：

- `capabilities.process.*` 与 `capabilities.subject_defaults.*`；
- `runtime.resident_limit`；
- `serving.lexical.enabled`、`serving.dense.enabled`、`serving.topology.enabled`；topology 是显式 experimental lane，默认关闭。
- `memory.accessibility.*`；
- `retrieval.rrf.*` 与 `retrieval.query.*`；
- `topology.wave.*`，包括 edge quality 和 seed weights；
- `social.language_convention.*` 与 `social.query.scope_preference`。

## 持久化

canonical fresh migrations `0001_foundation.sql`–`0004_indexes.sql` 建立 `configuration_state`、system/subject overrides、mutation receipts 和 `subject_capabilities`。Subject capability 是创建时展开并保存的供给状态，不随默认配置变化。

Bootstrap 文件只保留数据库、对象存储、Server bind 和 Serving root 等启动前参数；算法与策略值位于 `[settings]` registry。

Artifact upload 的唯一部署上限为 Kernel `[bootstrap.object_store].max_upload_bytes`。Core 从 Kernel 读取同一有效值配置 multipart 接收；`MaterialService.GetLimits` 向 official Client 提供此值，Node uploader 每次上传先检查它。Core TOML 与 Client 不再维护第二份 8 GiB 上限。此 bootstrap 值改变后重启实例。

## 可选 External Resource

`resource_profiles.<name>` 配置已有外部系统的 API；当前 `adapter_kind="ragflow"`，`base_url` 指向 `/api/v1` 根，`credential_env` 引用 SecretRoot/gateway.env 或进程环境中的 key。`enabled=false` 禁用 profile；timeout 与单条 material byte 上限是有界网络策略。Resource descriptor 的 `provider_profile` 引用该名称，`provider_locator` 保存 JSON selector（`dataset_ids` 与可选 `document_ids`）。Nous 不管理 RAGFlow dataset、模型、安装或 Docker 网络；它不属于默认 runtime 或 acceptance 依赖。直接交给 Nous 的本地材料使用原生 Material/Serving。

## Direct audio/video and model-call budget

`model_profiles.<name>.embedding.max_batch_size` controls the number of inputs per actual embedding request (1–64, default64). Set it to1 for a gateway that accepts only a single input. Preparation and query batching use the same profile. Every emitted request reserves budget independently, and successful summaries aggregate actual request counts and reported usage; failed batches are not silently retried with another shape.

The user configuration entry remains ConfigurationRoot/nous.toml. `model_budget.max_calls` accepts1..10000 (default10000); reservations are persisted at InstanceRoot/model-budget.json, including failed requests. Increasing the limit preserves the accumulated count.

`audio.input_mode` is `direct` (default) or `transcription`. Direct uses material_description or material_direct_structuring with audio_input capability and openai-chat content; transcription requires the separate speech_transcription role. `audio.max_source_bytes` bounds raw bytes (default16MiB, maximum24MiB).

`video.input_mode` is `direct` (default) or `frames`. Direct requires video_input plus gateway video_url content-extension support; it sends the bounded uploaded Artifact, without client-side frame extraction. Frames explicitly selects the FFmpeg frame path; max_frames/frame_bytes/audio_bytes/process_timeout apply to that path. Source-byte bounds apply to both modes. There is no automatic mode fallback. Configure model identifiers and declared capabilities in ModelProfile; readiness requires an actual validated invocation. Gateway aliases do not establish the underlying model version.
