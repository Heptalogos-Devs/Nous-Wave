# Configuration Service 当前参考

应用配置位于 ConfigurationRoot/nous.toml，配置文件必须声明 `config_revision = 1`。Core 启动时解析配置并 materialize model/resource clients；改变 model profile、role 或 Prompt 后重启 Core 生效。

完整字段和可编辑示例见 [nous.toml](examples/nous.toml)。Bootstrap locator 只定位独立运行目录，见 [bootstrap.toml](examples/bootstrap.toml)。

## 配置检查与初始化

`nous init` 和首次 `nous serve` 在 ConfigurationRoot 以 exclusive create 写入最小配置。已有文件保持原样，不自动改写或迁移。配置版本不匹配时启动和检查命令拒绝该文件。

`nous config check --home <instance>` 或 `--locator <bootstrap.toml>` 检查版本、字段、profile/role 引用、Prompt 和显式 executable 文件，输出 JSON 的 `valid`、`config_revision`、`configuration` 与 `issues`。检查不写文件、不加载凭据、不启动数据库，也不调用 provider。

## Configuration Service

Rust Configuration Service 负责 typed registry、解析、覆盖、immutable snapshot、mutation receipt 与 BLAKE3 digest。配置按 reference default → nous.toml `[settings]` → persisted system override → persisted Subject override 解析。每个 key 声明 owner、类型、校验、暴露等级、作用域与应用方式。

当前注册的配置覆盖 process/Subject capabilities、Runtime、Memory accessibility、lexical/dense/topology Serving、retrieval 与拓扑算法策略。配置 key 由其语义 owner 注册；对象 identity、领域 ownership、revision、lifecycle、幂等和 purge 是系统不变量。

Query bind、Serving prepare 和 Authority formation 使用固定的 ConfigSnapshot。其 digest 覆盖实际影响该 operation 或制品的 key subset。标记为 `RestartProcess` 的 mutation 保存 desired value 并报告 `pending_restart`；当前 active snapshot 在重启前保持不变。Subject capability 在创建时展开并持久保存。

Bootstrap 部分提供数据库模式、外部数据库 credential reference、pool 上限和 Artifact 上传上限。上传上限由 Kernel `[bootstrap.object_store].max_upload_bytes` 拥有；Core multipart receiver 与 official Client 使用同一有效值。

## Models 与 Prompts

`gateway_profiles` 指定 endpoint、credential environment variable、enabled state 和 request timeout。Remote endpoint 使用 HTTPS；literal loopback 可使用 HTTP。凭据从 SecretRoot 的 dotenv 文件或进程环境读取，进程环境优先；凭据不进入公开输出。

`model_profiles` 描述 gateway、标准 protocol、model identifier、能力和可选 revision。Embedding profile 同时声明 dimension、weights revision、task、input representation、preprocessing identity/revision、normalization 与 output semantics，组成 EmbeddingSpaceSignature。角色通过 `roles` 绑定 model profile、Prompt、generation parameters、timeout 与 `optional | preferred | required` requirement。

当前 role 包括 projection steward、Memory formation、material description/structuring/direct structuring、query embedding/rerank、speech transcription、episode segmentation、Journal synthesis 和 Memory consolidation。Supported protocol 是 `openai-chat`、`openai-responses`、`openai-embeddings`、`openai-audio-transcription` 与 `rerank-v1`。

Prompt 默认来自 ProgramRoot/prompts；配置可使用 `config-prompts/` 前缀引用 ConfigurationRoot/prompts 下的文件。Prompt 必须是 UTF-8、位于所属 root 内且不超过 128 KiB。Producer identity 包含 Prompt 与 role config digest；token 不进入 ProducerSignature。

## Media 与 External Resource

`audio.input_mode` 为 `direct`（默认）或 `transcription`。Direct 将原始 Artifact bytes 作为 chat 的 `input_audio` 发送；transcription 需要 speech_transcription role。

`video.input_mode` 为 `direct`（默认）或 `frames`。Direct 将原始 Artifact bytes 作为 chat 的 `video_url` 发送；frames 使用有界 FFmpeg 抽帧与可选音轨转写。FFmpeg 来自显式 executable 或当前 RuntimeRoot 已安装 pack。两种 mode 都受来源字节上限约束；frames 另受时长、帧数、单帧、音频与进程时限约束。没有隐式模式回退。frames 的实验观测见 [Research](../research/README.md)。

`resource_profiles.<name>` 当前支持 `adapter_kind = "ragflow"`，配置 endpoint、credential environment variable、enabled state、timeout 与单条 material byte 上限。Resource profile 连接操作者管理的 RAGFlow API；直接输入 Nous 的材料经原生 Material 与 Serving 处理。

`consumers` 按 consumer id/revision 保存 Memory、Runtime、Resource contribution requirements 与 item/text budgets。Consumer policy 为一次调用限定各 owner 可贡献的内容和预算；领域 Authority 仍由对应 owner 持有。

[返回 Reference](README.md)

## 纵向认知配置

这些 typed settings 由 Runtime/Memory registry 定义，支持 registry 声明的 live scope override；实际生效快照由 Configuration Service 读取。

| Setting | 默认值 | 单位/含义 |
| --- | --- | --- |
| `maintenance.enabled` | true | host maintenance 开关 |
| `maintenance.poll_interval` | 30 | standalone loop 的基础设施秒 |
| `maintenance.worker_lease_seconds` | 120 | worker lease 的基础设施秒 |
| `maintenance.max_operations_per_grant` | 4 | 每次机会/standalone tick 的操作数上限 |
| `maintenance.terminal_retention_seconds` | 86400 | terminal need finish replay 的基础设施秒，范围 1..604800 |
| `maintenance.retry_initial_seconds` | 30 | transient retry 初始基础设施秒，范围 1..3600 |
| `maintenance.retry_max_seconds` | 3600 | transient retry 延迟上限秒，范围 1..86400 |
| `maintenance.retry_max_attempts` | 8 | 连续 transient failure 上限，范围 1..32；达到后 blocked |
| `maintenance.max_model_calls_per_tick` | 4 | standalone tick 全局模型调用预算，范围 1..32 |
| `maintenance.max_elapsed_ms_per_tick` | 60000 | standalone tick 全局 elapsed 毫秒预算，范围 1..300000 |
| `episode.soft_idle` | 300 | 认知秒 |
| `episode.hard_idle` | 1800 | 认知秒 |
| `episode.settle_delay` | 300 | semantic review 的认知秒 |
| `episode.max_neighbor_episodes` | 8 | 局部 repair 的 Episode 数 |
| `episode.max_neighbor_span` | 86400 | 局部 repair 的认知秒 |
| `journal.max_episode_count` | 12 | 一次 Journal synthesis 的 Episode 数 |
| `journal.max_span` | 86400 | Journal scope 的认知秒 |
| `consolidation.settle_delay` | 300 | 整合前的认知秒 |
| `consolidation.max_actions` | 8 | 一个原子整合 proposal 的 action 数 |

`GrantMaintenance` 调用同时提供 operation/model-call/elapsed budgets；有效操作数还受当前 registry policy 限制。`poll_interval`、`worker_lease_seconds` 和新增的 retention/retry/tick budget 设置均由 `cognitive-runtime` owner 注册，使用 Developer exposure、SystemOnly scope、Live apply mode 和 Operational semantic effect。其他以上设置允许 Subject override。retry 延迟为 `min(retry_max_seconds, retry_initial_seconds × 2^(连续失败次数−1))`。语义合同见 [纵向认知](../specs/active/cognitive-runtime/longitudinal-cognition.md)。
