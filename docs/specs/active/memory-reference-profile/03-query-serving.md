# Query & Serving

状态：IMPLEMENTATION-AUTHORIZING

本 Spec 冻结 Memory Reference 的 public query、lane、fusion、Serving generation 和 final Authority revalidation。它不把实验 topology 提升为默认认知路径。

## BoundQuery

Query 在 candidate generation 前绑定 Subject、exact object/revision、CurrentOnly/ExactHistorical policy、enabled lanes、lane budgets、hard constraints、accessibility、embedding space、fusion version、topology intent 与 rerank policy。候选集合不得反向修改这些定义。

普通 Memory/Schema recall 只返回 current head；只有显式 exact historical revision target 才允许历史 revision。Mutable exact target 在 bind 时解析成 revision + object epoch；执行期间 head/epoch 变化时丢弃并返回 `stale_exact_binding`，不得自动重绑。

## Lanes and fusion

当前 baseline lanes 为 exact、entity、lexical、dense、temporal、runtime，以及适用时的 SchemaDirect。每个 lane 返回 exact revision candidate、deterministic rank、generation/watermark 和 diagnostics；lane provider 不能产生 global score。

- Entity/Temporal/Runtime 必须从 Authority/Runtime typed structure 生成 bounded candidates，不得 application-side arbitrary first-N。
- Lexical relevance 只来自 Lexical Serving hit；不得以 DB substring admission 或 fallback rank 补造 hit。
- Dense 绑定单一 `EmbeddingSpaceSignature`，不同 space 不混合分数。
- Hard constraint 未定义时在 binding 阶段明确拒绝；不得 silent ignore。
- 同一 revision 的多 view 先在 lane 内聚合，再参与跨 lane fusion。

Runtime 持有 QueryPlan、lane/result contracts、budget 和固定 fusion semantics；Retrieval 只实现 lexical/dense/topology serving mechanics。唯一 production fusion owner 由 Runtime 当前 contract 提供，不能在 Memory service 与 Retrieval 之间复制。

## Final authority

Final validator 批量读取 current head、epoch、lifecycle、role/mode、aboutness、temporal、provenance/source class、authority/modality/epistemic 和 accessibility。Serving/lane prefilter 是优化，不是 Authority；不得产生每 candidate 一次 SQL 的 N+1 路径。

旧 generation 可以作为 stale candidate source，但必须标记 degradation，并在返回前丢弃 stale revision、suppressed、purged 或 constraint 不再匹配的 candidate。Unavailable required lane 产生明确 Partial；optional lane 产生 Degraded。Query 诊断区分 budget 不足、projection unavailable、authority 不存在和主体未知。

## Serving

每类 projection 使用 immutable generation，绑定 authority watermark、producer/build identity、configuration digest、artifact checksum 和 vector-space identity。Authority 提交只使 generation 失效；重建生成新的 generation id，但语义结果必须仍指向同一 Authority identity/revision。watermark race 不得发布旧快照为 current。

Topology/Wave/Residual/EPA 只作为显式实验 lane；默认 Reference Query 不因 effort 自动开启 topology。实验结果不改变 baseline fusion、Authority eligibility 或 lifecycle。
