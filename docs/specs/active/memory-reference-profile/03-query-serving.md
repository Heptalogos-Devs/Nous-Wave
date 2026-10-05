# Query & Serving

[返回文档目录](../../INDEX.md)

## Owner

Runtime (crates/runtime) owns QueryPlan, lane budgets, fusion and result contracts. Retrieval (crates/retrieval) implements serving lanes and generations; Core performs model/resource host actions; Memory validates final Authority.

## BoundQuery

Query 在 candidate generation 前绑定 Subject、exact object/revision、CurrentOnly/ExactHistorical policy、enabled lanes、lane budgets、hard constraints、accessibility、embedding space、fusion version、topology intent 与 rerank policy。候选集合不得反向修改这些定义。

普通 Memory/Schema/Episode/Journal recall 只返回 current head；只有显式 exact historical revision target 才允许历史 revision。Mutable exact target 在 bind 时解析成 revision + object epoch；执行期间 head/epoch 变化时丢弃并返回 `stale_exact_binding`，不得自动重绑。

## Query domains

`domains` 接受 `memory`、`schema`、`episode`、`journal`、`evidence`、`resource`。Memory、CognitiveSchema、Episode、Journal 是四个独立认知域；多个域取并集。未指定域时召回所有可用域，包括四个认知域及 Material/Evidence、Resource 引用；Subject/process capability 决定 owner 可用性。Resource host action 仍由 query resource intent 决定。

父级域限制由子表达式继承，子级显式域与父级取交集；exact target 也受域限制。Lexical 和 dense 在取得有界候选前筛选域，Runtime 在 fusion 前再次筛选。自动整合的相关上下文显式选择 `memory + schema`。

## Lanes and fusion

当前 baseline lanes 为 exact、entity、lexical、dense、temporal、runtime，以及适用时的 SchemaDirect。每个 lane 返回 exact revision candidate、deterministic rank、generation/watermark 和 diagnostics；lane provider 不能产生 global score。

- Entity/Temporal/Runtime 必须从 Authority/Runtime typed structure 生成 bounded candidates，不得 application-side arbitrary first-N。Temporal lane 只接受 occurred、observed、valid、formed、recorded typed axes；unknown 不匹配已知时间约束，多轴是 hard intersection。
- Lexical relevance 只来自 Lexical Serving hit；不得以 DB substring admission 或 fallback rank 补造 hit。
- Dense 绑定单一 `EmbeddingSpaceSignature`，不同 space 不混合分数。
- Hard constraint 未定义时在 binding 阶段明确拒绝；不得 silent ignore。
- 同一 revision 的多 view 先在 lane 内聚合，再参与跨 lane fusion。

Runtime 持有 QueryPlan、lane/result contracts、budget 和固定 fusion semantics；Retrieval 只实现 lexical/dense/topology serving mechanics。唯一 production fusion owner 由 Runtime 当前 contract 提供，不能在 Memory service 与 Retrieval 之间复制。

## Request observation

Retrieval 在同一 bound leaf 内先准备 `PreparedQuerySignals`：query text、一次 query embedding 与只读 lexical/dense base hits。Dense 的兼容 generation 共用这份 embedding，不各自调用 provider；`text_embedding = FORBIDDEN` 时不准备 embedding。缺失 generation/provider 和调用失败保留原 lane 状态，由 Runtime 的 RequirementStrength 决定 Partial/Degraded。

`retrieval.cognitive.profile` 是 Developer/SystemOnly 的 typed fixed registry，使用 Live / QueryPolicy。四个 ID 为 `baseline-rrf`、`nous-node-potential-v1`、`vcp-dtsc-v9.2.1-adapter-v1`、`vcp-rivermemo-v3.1-adapter-v1`。Profile 在 BoundQuery/QueryPlan 中冻结；baseline 不产生 topology lane。当前 native topology readout 默认绑定 `nous-node-potential-v1`，机制为已冻结的 `experimental-node-potential-v1`。`QueryObservation` 以 query ID、bound time、topology generation、profile/config subset digest、source seeds 与五轴时间约束标识本次观测，只持有一份 QueryRiver。候选排名由 readout 计算，不写回 observation；普通 diagnostics 只报告 profile、seed/node/edge 数、实际最大 hop、完整性和 discarded mass。

Wave 的普通边必须支付 `normal_edge_cost`，预算不足时停止；合流状态合并能量和全部 origin，provenance 输出有稳定顺序。这些约束由实际传播执行，不依赖诊断层推断。当前默认数值行为由 frozen golden 保护。Profile 在 query 开始时冻结。DTSC 与 RiverMemo 共用同一 VCP immutable asset，asset digest 绑定 Authority watermark、embedding space/producer、asset policy 与 implementation revision；readout profile 不进入该 digest。Native graph 使用独立 asset contract。prepare 搜索 current 与 retired compatible artifacts，切回相同内容可重新 promote。VCP profile 的 embedding requirement 与 Dense 共用一次 preparation；FORBIDDEN 保持无调用。两个 VCP reference kernel/adapters 已接入 Authority-fenced query lane。

## Final authority

Final validator 批量读取 current head、epoch、lifecycle、role/mode、aboutness、temporal、provenance/source class、authority/modality/epistemic 和 accessibility。Serving/lane prefilter 是优化，不是 Authority；不得产生每 candidate 一次 SQL 的 N+1 路径。

已标记 stale 的 generation 可以提供 candidate source，但必须标记 degradation，并在返回前丢弃 stale revision、suppressed、purged 或 constraint 不再匹配的 candidate。Unavailable required lane 产生明确 Partial；optional lane 产生 Degraded。Query 诊断区分 budget 不足、projection unavailable、Authority 不存在和主体未知。

Episode/Journal 使用 current lifecycle 与适用的 hard constraints，不参与 Memory/Schema 的年龄衰减。结果保留 exact revision、object epoch、有界正文和精确支持引用。

## Serving

每类 projection 使用 immutable generation，绑定 authority watermark、producer/build identity、configuration digest、artifact checksum 和 vector-space identity。Authority 提交只使 generation 失效；重建生成新的 generation id，但语义结果必须仍指向同一 Authority identity/revision。watermark race 不得发布过期快照为 current。

Episode/Journal 的文本 projection 支持 exact、lexical、dense；它们不进入 topology。

Episode canonical text 包含 title、boundary explanation、experience time，以及前 16 个成员中 occurrence 的有界文本片段。文本 Artifact 读取至多 2 KiB 的完整 UTF-8 前缀；媒体 occurrence 选择同 Artifact 的 ready coverage 派生文本，按 created time 与精确 representation ID 确定顺序，每段同样限制为 2 KiB。Serving 和查询正文共用该成员输入；描述更新使文本 projection 失效，不修改 Episode Authority。查询 evidence 保留所用派生描述的 exact ref 与 interpretation role，独立根仍由来源 lineage 决定。

Journal canonical text 包含 title、narrative 和按序 points；查询正文最多 64 KiB，截断保持完整 UTF-8 字符。


Topology/Wave/Residual/EPA 只作为显式实验 lane；默认 Reference Query 不因 effort 自动开启 topology。实验结果不改变 baseline fusion、Authority eligibility 或 lifecycle。

## Serving 复用与回收

Kernel 固定 request-scoped immutable Serving view；并发 profile 切换不替换在途视图。QueryExecution 持有 opaque read lease，retained validation ticket 沿用该 lease；finalize/release/expiry 释放它，Runtime 不依赖 concrete Retrieval。

Query 进入 Serving prepare 前检查 retired collection。`serving.retired_grace_seconds` 为 Developer/SystemOnly/Live，默认 300 秒，范围 0..604800。当前 generation、active query readers/tickets 和显式 research pins 不回收；collection 只跳过被 active reader/ticket 引用的 generation；无关 retired generation 仍可回收，避免持续流量阻塞全部 collection。过 grace 的 unpinned retired artifact 删除物理目录并压缩 metadata 为 checksum/reclaimed audit summary。该 pass 同时清理 owner root 下超过 grace 的未引用 UUID generation 和 orphan staging。配置/watermark 不兼容时不复用；重复并发 publication 复用已发布的相同内容身份并删除多余目录。

## Prepared Query 与 Query Representation

公开 `CognitionService.PrepareQuery` 与官方 Client `cognition.prepareQuery` 编译 NousQL 或 typed expression、解析 exact selectors、读取 Session/active WorkContext，并返回既有 `bound_query` 字段语义的 JSON inspection；不执行 retrieval、Serving build 或 provider。Inspection 包含 resolved CognitiveQuery、representation/version/SHA256/source refs/truncation flags、current refs、topology seeds、exact bindings、profile 与 ConfigSnapshot digest。

正式 query 在 binding 时检查 TextCue/ExampleCue 的 referential closure。中文 pronoun/deictic phrase 与英文 whole token 规则返回 `UNRESOLVED_QUERY_REFERENCE`，detail 保存 offending span、UTF-8 byte offsets 与 pronoun/deictic/temporal_deictic kind；不猜测 referent。Core 的 Entity/Tag/name selectors 仍通过 Identity Directory 唯一解析。

QueryRequest 支持 `work_context_id` 和 `situation`（consumer、current refs、current objects、object descriptions）。显式 context 覆盖 foreground selection，须为同 Subject 的 open WorkContext。Runtime 在 preparation 捕获 ResidentSet references、WorkContext 与 request situation，保留来源；Runtime lane 和 topology 使用冻结 refs，执行时不重新读取另一个 Session/context。Embedding 只消费有界 current descriptors，不复制全部 ResidentSet 或 Session transcript。

Representation 顺序固定：Intent、Temporal orientation、Entities、Concepts、Schemas、Current cognition、Resources、Current objects、Current work、Consumer/task。Entity 使用 display name/必要 aliases，Tag 使用 label/description/kind，cognition 使用 bounded owner text；opaque identity 留在 exact/source refs，不当语义正文。缺失 descriptor 和超界截断显式报告；total budget 保持 UTF-8 完整字符。

`retrieval.query.representation` 是 typed Developer/SubjectOverrideAllowed/Live policy：默认 intent 2048 chars、WorkContext 1024、Entity/Tag descriptor 256、current descriptor 512、最多 16 Entity/Tag/current refs、total 8192。Intent 和 explicit cues 优先于 current descriptors 与 WorkContext。`sha256` 对实际 representation text 计算。

Core 查询先取得一次冻结 BoundQuery 的 bounded preparation token，再为完整 representation 生成最多一份 embedding，Kernel 直接消费该 token，保持 ConfigSnapshot 与 context 一致。Preparation/validation tickets 共用 query slots/lease，single-use、Subject-bound，并在 failure/finalize/release/expiry 清理。Dense、EPA/VCP sensing 与 expression leaves 共享 request embedding（包括 provider failure），lexical leaf 仍使用该 leaf intent；All/Any 的集合语义保持不变。Rerank 接收同一完整 representation 和原 validated candidates。

`text_only_compatibility=true` 是独立研究输入条件，只接受一个 standalone TextCue，不带 Session/WorkContext/situation/exploration。它保留原 query text 的 embedding 和原数据集第一人称表达，不冒充 Prepared cognitive input，结果须独立报告。

QueryRequest 的 typed `capabilities` 传递 text embedding、multimodal interpretation、residual sensing 与 rerank requirement。`rerank: "forbidden"` 明确关闭 model rerank，适用于 deterministic algorithm/Agent wiring run；`text_embedding: "forbidden"`（Client 为 `textEmbedding`）禁止 embedding provider。Required 需求的失败不静默回退。

内部算法比较使用 `BoundQuery::for_profile` 从同一 prepared/context snapshot 生成只读 readout plan。Configuration 的 `query_override` 只允许 Live QueryPolicy，记录 OperationOverride 来源并计算独立 digest；不改变 active/desired/persisted 配置。Serving 根据对应 asset contract 复用 shared artifacts。


Native seed preparation first uses closed exact/context/Entity/Tag/relation anchors that map into the current graph. When no anchor maps into that graph, already prepared ready/truncated lexical and dense lanes can supply weak seeds: at most four distinct graph members per lane, bounded by the graph neighbour budget, with configured `lexical_promoted`/`dense_promoted` weight divided by lane rank. Out-of-generation references and rank zero are excluded. The fallback consumes the shared query signal; it makes no additional embedding request and does not change Wave propagation. Direct graph anchors remain authoritative for explicitly associative queries. Seed origin and weight are included in readout; a promoted direct hit is not a multi-hop witness.


Topology projection includes every current accepted/valid/unsuppressed Memory, Episode, Journal and CognitiveSchema revision when Memory capability is enabled. Existing Episode cognition members/exact supports, Journal sources, Memory dependencies and active supporting Schema evidence links supply `cognition_support` edges in both traversal directions, with one shared exact structure identity. These are source/organization adjacency, not causal or independently learned associations. Historical, withdrawn or suppressed endpoints, revoked Schema evidence, counterexamples and contradiction supports do not create positive source edges. Explicit AssociationEvidence to inactive cognition is excluded from the current graph. The topology implementation revision is 9, so older graph/VCP assets rebuild while compatible vectors remain reusable.


Generic evidence materialization resolves temporal metadata in a bounded Subject-scoped batch. Occurrence times come from the actual Observation; SourceRegion and derived evidence follow their real source artifacts/representation roots. An eligible source row must jointly satisfy requested occurred/observed/recorded predicates; multiple origins cannot satisfy separate predicates by mixing rows. Recorded time belongs to the returned object, and interpretation formation time is distinct from source observation. Unknown time does not satisfy a requested temporal axis, and evidence has no inferred world-valid claim. Initial owner materialization and final validation both enforce these predicates and publish known freshness. Schema owner materialization also applies all five axes and follows exact source lineage, including Memory evidence/dependencies, for occurred/observed metadata.
