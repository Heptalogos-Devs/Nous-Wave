# Structured Material 与 External Resource

状态：IMPLEMENTATION-AUTHORIZING；2026-10-01 用户执行合同。本文替换旧 Material `{text,facts}` 结构、文本字段内 JSON、normal runtime 调用计数限制，以及旧 qualification 将 duplication 排除为完成条件的条款。

## Schema 与来源

Core 的 `model/schemas/material-interpretation.ts` 是唯一 model-facing Zod owner，供 inferred type、local parse、AI SDK Output.object、raw strict JSON Schema 使用。根与嵌套对象 strict、字段 required、可缺省 scalar 为 null；资源数量/byte/time/support/modality 约束在 Material validator 校验。Prompt 只描述忠实度和任务，不复制 JSON 结构。

输出包含 summary、coverage(visual/audio/embedded_text)、observations、mentions、embedded_text、speech、interpretations、uncertainties。direct 与 two-stage 使用同一 schema。direct 项必须有 support；audio-only 不产生 visual direct observation；image-only 不制造 speech；未知 support key 失败。未完成/refusal/非法输出不能 commit。

规范 provider schema 的 digest 进入 invocation evidence、ProducerSignature 和 derivation workflow identity。Schema 改变不复用旧 outcome。DerivedRepresentation 增加一等 structured payload；text 是保留 uncertainty/basis 的 deterministic projection，算法 identity 进入 preprocessing digest。payload_text/payload_json/payload_artifact 至少一个。

description_only 提交自由描述。direct_structured 给模型 invocation-local source catalog。describe_then_structure 第一阶段成功立即提交；Material 对已提交文本做 deterministic UTF-8 byte segmentation，保存 description_segment DerivedRegion(ordinal/start/end/digest)，将 D001 等 keys 提交第二模型。模型只输出 keys；本地映射成稳定引用后保存字段 supports。第二阶段失败保留 description、selected 指向 description、显式 degradation。graph 回到原始 Artifact，shared roots 去重，unknown dependency 不成为独立证据。

## Resource 与续接

按 Authority/lifecycle owner 区分 Material-owned corpus 与 External Resource。直接交给 Nous 的材料由 Artifact/SourceRegion/DerivedRepresentation 与原生 Serving 负责；另一系统持有的知识库通过 ExternalResourceAdapter 访问。RAGFlow 是可选外部 provider，不是本地材料的默认后端，不属于 Runtime Bundle、runtime packs、默认启动或默认验收依赖。Nous 不安装、启动、管理或自动配置 RAGFlow 的 dataset、模型和 Docker 网络；只读取用户已有实例的 API profile 与 resource locator。New API 同样属于共享部署基础设施，Nous 不管理其生命周期。

本分支完成通用 External Resource contract、optional RAGFlow adapter 和 deterministic qualification。live RAGFlow 仅为手动集成验证；用户确认未配置时记 BLOCKED，不阻塞默认 runtime 或 deterministic acceptance。本分支不新增 MaterialCollection、不大规模施工 Native Material Corpus；后续独立研究先审计当前 Material/Tantivy/USearch/Serving substrate。

Kernel Resource descriptor 保存 adapter_kind/provider_profile/provider_locator，网络与 credential 留 Core。Core adapter 负责 describe/search/materialize/checkVersion/checkAccess；首个 provider 为 RAGFlow，vendor wire 只在其 adapter。StableExternalRef 保存 provider/profile digest/resource/entry/version(nullable)/content digest/source locator/retrieved time/access descriptor；无 provider revision 时重新读取 content digest 检查变化。

沿已有 prepare → host action → finalize 的单一 immutable ticket 续接，不另建 pending state machine。Kernel 在 provider call 前固定 Resource action IDs、绑定 descriptor、query intent、limits、materialize/current-authority、budgets、config snapshot。finalize 校验 Subject/action/resource/provider identity、一次提交、数量和 current version/access。不能创建新 action 或改变 lane/query budgets。provider records 保留 provider rank，不与认知 raw scores 相加。

public QueryResponse 的 resource_records 与 cognitive hits 分开；resource_actions 只保留未执行项。未配置/失败 adapter 显式 degradation，local hits 保持原合同。Core finally release ticket，cancellation 传播网络请求。official Client 可管理 Resource、query、读取 records、materialize/admit selected record。

Core profile `max_material_bytes` limits each record up to1MiB; Kernel enforces the same1MiB hard ceiling and2MiB total serialized record budget per query. Current authority policies are none/prefer/required. The unused internal HistoricalOrStale variant had no NousQL/compiler/public producer and is removed. Records marked denied/stale/missing are excluded; current-authority success requires allowed/current status. Historical external snapshots are not an executable query policy in this scope.

Each private external result includes typed provider_evidence (profile digest, actual initiated HTTP request count, elapsed milliseconds) and bounded diagnostic codes. The Core adapter increments at fetch initiation, including failed transport attempts; unconfigured adapters initiate zero requests. Evidence is call-scoped, never shared mutable adapter state. Kernel rejects nonfinite/negative latency, more than65 requests per action, malformed profile identity, record/evidence digest mismatch, or more than8 diagnostic codes (64 bytes each, lowercase/underscore). These bounds cover the current16-dataset synopsis and1-search-plus64-validation paths. Provider bodies, credentials and content are excluded from diagnostics.

Public `resource_invocations` retains evidence for successful and failed actions in the fixed QueryPlan action order, independently of candidate scores. Descriptor drift reports discarded execution evidence and preserves the original Authority fence. The official Client returns this collection directly. None of these provider timings or counts imply external model usage/cost is known.

未呈现候选不建立 Observation。实际选用时有界 materialize → Artifact/SourceRegion → distinct ObservationOccurrence，external_object_ref 保存 stable ref identity；再次观察保持事件身份。external result 不直接成为 Memory。

## 执行与证据

normal runtime 不创建 persistent model-call count guard；research/qualification 可显式注入 run-owned guard，失败/warm-up/retry 按真实 request attempt 计数，restart 延续同一 ledger。timeout/concurrency/media/output bounds 继续是产品策略。

当前 research composition 在标准 GatewayProfile 前显式启用 `research:gateway`，以 HTTP wire attempt 为计数边界。它只转发既有标准模型 endpoints，不管理 credentials、channels 或 provider；普通 serve、ModelInvocations 与 nous.toml 已移除研究预算。单一 run-owned proxy/ledger writer 串行 reservation，研究实例显式使用其 loopback endpoint；重启不清零。

唯一完整 deterministic 验收为 `just acceptance`，包括 generated diff、pnpm check、just verify、Rust/TS dupes、OSV source scan、Kernel build、Memory/Runtime/Episode 与新增 public cognitive-io-resource qualification。Ready/manual workflow 只安装固定工具、依赖并调用入口。真实模型/RAGFlow 手动运行；无 endpoint/key/dataset 时 live provider BLOCKED。

本分支只记录 VCP route/source audit 和现有 experimental topology 的实际能力。完整 VCP topology 与 CognitiveClock/longitudinal 是后续独立授权任务；不在本分支创建对应实现或分支。
