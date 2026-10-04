# Query & Serving

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

`retrieval.cognitive.profile` 是 Developer/SystemOnly 的 typed fixed registry，使用 ServingRebuild。四个 ID 为 `baseline-rrf`、`nous-node-potential-v1`、`vcp-dtsc-v9.2.1-adapter-v1`、`vcp-rivermemo-v3.1-adapter-v1`。Profile 在 BoundQuery/QueryPlan 中冻结；baseline 不产生 topology lane。当前 native topology readout 默认绑定 `nous-node-potential-v1`，机制为已冻结的 `experimental-node-potential-v1`。`QueryObservation` 以 query ID、bound time、topology generation、profile/config subset digest、source seeds 与五轴时间约束标识本次观测，只持有一份 QueryRiver。候选排名由 readout 计算，不写回 observation；普通 diagnostics 只报告 profile、seed/node/edge 数、实际最大 hop、完整性和 discarded mass。

Wave 的普通边必须支付 `normal_edge_cost`，预算不足时停止；合流状态合并能量和全部 origin，provenance 输出有稳定顺序。这些约束由实际传播执行，不依赖诊断层推断。当前默认数值行为由 frozen golden 保护。Profile 身份进入 topology Artifact、generation metadata 与 Serving config digest，profile 切换由正常 prepare 发布新代；query 与已加载 generation 身份不一致时返回 unavailable。VCP profile 的 embedding requirement 与 Dense 共用一次 preparation；FORBIDDEN 保持无调用。两个 VCP reference kernel/adapters 仍在当前 topic 中实现，其当前执行明确返回 unavailable。

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

[返回文档目录](../../INDEX.md)
