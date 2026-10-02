# Structured Material 与 External Resource

状态：CURRENT PRODUCT CONTRACT。

## Schema 与来源

Core 的 `model/schemas/material-interpretation.ts` 是唯一 model-facing Zod owner，供 inferred type、local parse、AI SDK Output.object、raw strict JSON Schema 使用。根与嵌套对象 strict、字段 required、可缺省 scalar 为 null；资源数量/byte/time/support/modality 约束在 Material validator 校验。Prompt 只描述忠实度和任务，不复制 JSON 结构。

输出包含 summary、coverage(visual/audio/embedded_text)、observations、mentions、embedded_text、speech、interpretations、uncertainties。direct 与 two-stage 使用同一 schema。direct 项必须有 support；audio-only 不产生 visual direct observation；image-only 不制造 speech；未知 support key 失败。未完成/refusal/非法输出不能 commit。

规范 provider schema 的 digest 进入 ProducerSignature 和 derivation workflow identity。Schema 改变不复用旧 outcome。DerivedRepresentation 增加一等 structured payload；text 是保留 uncertainty/basis 的 deterministic projection，算法 identity 进入 preprocessing digest。payload_text/payload_json/payload_artifact 至少一个。

description_only 提交自由描述。direct_structured 给模型 invocation-local source catalog。describe_then_structure 第一阶段成功立即提交；Material 对已提交文本做 deterministic UTF-8 byte segmentation，保存 description_segment DerivedRegion(ordinal/start/end/digest)，将 D001 等 keys 提交第二模型。模型只输出 keys；本地映射成稳定引用后保存字段 supports。第二阶段失败保留 description、selected 指向 description、显式 degradation。graph 回到原始 Artifact，shared roots 去重，unknown dependency 不成为独立证据。

## Resource 与续接

`$current` 是 scoped current-authority 约束，随 expression 进入冻结 leaf action；父级 required 不能被子级 none/prefer 放宽。Kernel 内部只在 QueryConstraints 保存该值，删除旧 query-wide ResourceIntent 副本。Resource ticket 本身不授权 model rerank；没有正文本意图或没有 configured rerank role 时仍完成 Resource 续接，跳过 rerank。

selected materialization 的内容上限仍为1 MiB UTF-8，query records 的2 MiB aggregate bound保持不变。重放 proposal/outcome 需要容纳 JSON 最坏6倍转义，完整 workflow JSON 上限为8 MiB；private ModelMaterial transport 额外保留64 KiB envelope 空间。RAGFlow response wire 上限为8 MiB，内容上限由 profile 单独核验。该调整不提高原始材料上限。

按 Authority/lifecycle owner 区分 Material-owned corpus 与 External Resource。直接交给 Nous 的材料由 Artifact/SourceRegion/DerivedRepresentation 与原生 Serving 负责；另一系统持有的知识库通过 ExternalResourceAdapter 访问。RAGFlow 是可选外部 provider，不是本地材料的默认后端，不属于 Runtime Bundle、runtime packs、默认启动或默认运行依赖。Nous 不安装、启动、管理或自动配置 RAGFlow 的 dataset、模型和 Docker 网络；只读取用户已有实例的 API profile 与 resource locator。New API 同样属于共享部署基础设施，Nous 不管理其生命周期。

Kernel Resource descriptor 保存 adapter_kind/provider_profile/provider_locator，网络与 credential 留 Core。Core adapter 负责 describe/search/materialize/checkVersion/checkAccess；首个 provider 为 RAGFlow，vendor wire 只在其 adapter。StableExternalRef 保存 provider/profile digest/resource/entry/version(nullable)/content digest/source locator/retrieved time/access descriptor；无 provider revision 时重新读取 content digest 检查变化。

沿已有 prepare → host action → finalize 的单一 immutable ticket 续接，不另建 pending state machine。Kernel 在 provider call 前固定 Resource action IDs、绑定 descriptor、query intent、limits、materialize/current-authority、budgets、config snapshot。finalize 校验 Subject/action/resource/provider identity、一次提交、数量和 current version/access。不能创建新 action 或改变 lane/query budgets。provider records 保留 provider rank，不与认知 raw scores 相加。

public QueryResponse 的 resource_records 与 cognitive hits 分开；resource_actions 只保留未执行项。未配置/失败 adapter 显式 degradation，local hits 保持原合同。Core finally release ticket，cancellation 传播网络请求。official Client 可管理 Resource、query、读取 records、materialize/admit selected record。

Core profile `max_material_bytes` limits each record up to1MiB; Kernel enforces the same1MiB hard ceiling and2MiB total serialized record budget per query. Current authority policies are none/prefer/required. The unused internal HistoricalOrStale variant had no NousQL/compiler/public producer and is removed. Records marked denied/stale/missing are excluded; current-authority success requires allowed/current status. Historical external snapshots are not an executable query policy in this scope.

未呈现候选不建立 Observation。实际选用时有界 materialize → Artifact/SourceRegion → distinct ObservationOccurrence，external_object_ref 保存 stable ref identity；再次观察保持事件身份。external result 不直接成为 Memory。
