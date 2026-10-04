# VCP source conformance

## 来源与状态

本轮冻结 `lioensky/VCPToolBox@e03b891d42055cdf3cab5dc961cc71dd5facd65a`。精确文件 blob 与研究基线由 [source manifest](corpus/cognitive-retrieval-sources.json) 拥有。本页描述已读 production Rust 合同及当前差异；独立 reference kernel 已开始实现，完整 fixture matrix 与 Nous adapter 尚未完成，不能将源码研究写成 `reference-parity`。

已完整研究 `memo_pipeline.rs`、`memo_sensing.rs`、`memo_artifact_builder.rs`、`memo_dtsc.rs`、`rivermemo_topology_v3.rs` 与 `rag_params.json`。已进一步读取 lib.rs 中 EPA density sampling/basis 构建与 intrinsic residual compute owner。TagMemoEngine 控制面、要求的完整文档与 unified geometry probe 仍需完成。参数须以 frozen effective config 为准：Rust fallback defaults 与 rag_params 的 alpha、support count、contact thresholds、language penalty 有差异。

VCP 为 CC BY-NC-SA 4.0，Nous 为 MIT。本页只记录独立提取的数学合同；Nous 实现不包含第三方源码。Reference fixture 将保存数值输入、输出、浮点容差与来源身份。

## 分项矩阵

`Label` 描述当前 Nous 实现与该项 reference 的关系；未完成的新 reference/adapter 明确标记待实现。参数相同不代表语义一致，排序相同也不能替代中间数值 parity。

| Component | Frozen VCP owner | Exact contract | Nous current | Reference kernel | Nous adapter | Label |
| --- | --- | --- | --- | --- | --- | --- |
| EPA | memo_pipeline analyze_epa；lib.rs EpaBasisTask | f32 query centering，basis dot 的平方能量概率，归一化熵、dominant axes 与 bridge resonance；query analysis 不读取 cached training energies | weighted representative PCA 与当前 EPA policy | query analysis 首个 native 数值 fixture 通过；basis builder 待实现 | 待实现 | uncertain |
| Residual | memo_pipeline analyze_pyramid / orthogonal_projection；lib.rs IntrinsicResidualTask | 每层 residual ANN，按返回序正交化 Tag，dependent axis contribution 为零；f32 投影累积，解释量归一到原 query energy | bounded residual search 与 Nous own residual contract | 待实现 | 待实现 | nous-native |
| graph build | memo_artifact_builder build_fact_matrix / build_transport | 有序 file-tag 全 pair；位置/距离/semantic/reverse anchor 调制；log evidence、target inflow hub 校正与预算内 wormhole reserve | CognitiveRef/AssociationEvidence，support class quality 与独立 provenance root max dedup | neutral ordered graph builder 首个 native fact/CSR/provenance fixture 通过；完整 graph matrix 待补 | 待实现 | nous-native |
| bounded Sense | memo_sensing sense_typed | energy、momentum、hop、previous-node state；normal/wormhole decay、立即回流抑制、FIR、邻居/state 上限 | Wave 的固定 outbound、步数预算与 state 上限；无 VCP wormhole/decay 合同 | neutral Sense 数值与状态 matrix 通过；单一 lineage 边界单独记录 | 待实现 | nous-native |
| query river | memo_sensing SenseOutput | node potential 与实际注入 edge flow；source field 为最终 retained FIR 分布；edge 记录先于 momentum/state admission | QueryRiver 包含 source/potential/edge/provenance/mass；native source 为初始归一 seed | Sense node/edge/source field 与逐跳 transfer matrix 通过；VCP 单一 lineage 不作为稳定合同 | 待实现 | nous-native |
| dual fields | memo_pipeline solve_dual_fields | 同一 source/transport 的两个 scaled resolvent，独立 L1 convergence、mass-ratio domains 与 vector projection | 尚无该 VCP profile 的双场 readout | neutral 双 resolvent 首个 native 数值 fixture 通过；vector projection 待实现 | 待实现 | vcp-derived |
| DTSC | memo_dtsc score_curve / run | ordered curve 的 exact/interpolated field contacts、coverage、continuity、action、closure、D/S/T、受限 reward floors；low trust 保留原输入顺序 | 当前 native 以 node potential 排序 | 待实现 | 待实现 | absent |
| morphology | rivermemo_topology_v3 compute_query_morphology | candidate/text 无关的 river 统计，三个 logits stable softmax，按样本/complete confidence 向 uniform prior 收缩 | 无三形态混合 | 待实现 | 待实现 | absent |
| Ω | rivermemo_topology_v3 compute_omega | edge activation、emergence、正 raw flow entropy 的 epsilon-floored geometric mean，乘完整度因子 | native complete/discarded mass，尚无 Ω | 待实现 | 待实现 | absent |
| Direct Anchor | rivermemo_topology_v3 compute_anchors | hop-0 seed/core exact 或高阈值 semantic contact，mass/specificity/closure/pool rarity noisy-OR；缺少 lineage 的 fallback 限 reliability | exact lane 与 native seeds，不等于 VCP 独立 anchor reward | 待实现 | 待实现 | absent |
| conditional innovation | rivermemo_topology_v3 assign_v3_scores | 条件 peer Gaussian 期望、variance/ESS uncertainty，正 innovation 经 role cap 与 Ω gate；独立 batch anchor activation | native topology rank 经 Runtime fixed fusion，无此读出 | 待实现 | 待实现 | absent |
| TimeDecay | rag_params 与实际 JS caller 待完整确认 | native V3 的 time score 只影响 candidate superset；不能声称它直接进入 pure final score | Runtime own temporal hard constraints/age preference，不声明 VCP parity | 待实现 | 待实现 | uncertain |

## 当前数值 conformance

独立 checkout 的 frozen native 模块已编译。只读 research export 暴露同一 cached observation，不修改算法；原 blob、export module 与 binary checksum 存在 ignored `data/research/vcp-reference/e03b891d42055cdf3cab5dc961cc71dd5facd65a/source-manifest.json`。

首个三维图 probe 实际执行 EPA publish、intrinsic residual、artifact build、统一 pipeline、DTSC 与 V3，导出 mean/basis/energy、residual/anchor、CSR、Sense 转移/node/edge、双场、morphology、Ω 与 candidate components/order。Tracked [数值 fixture](../../crates/retrieval/tests/fixtures/vcp-epa-dual-fields.json) 只包含生成的中间数值与来源身份，独立 [reference kernel](../../crates/retrieval/src/reference/fields.rs) 使用中性 DTO。

当前通过 query EPA 的 entropy/depth/resonance/axis energy，以及双场每个 node mass、支持域、迭代次数、convergence 与 L1 residual；absolute/relative tolerance 均为 `1e-10`。该 EPA/双场组仍只覆盖一个 component fixture。另有 [Sense graph matrix](../../crates/retrieval/tests/fixtures/vcp-sense.json)，由 frozen `sense_typed` 实际执行生成 chain、fork、merge、立即回流、hub、两独立 roots、wormhole bridge、disconnected distractor、weak/strong edges、state truncation、empty seeds 和 transition-record truncation 共 12 个 case。

独立 [Sense kernel](../../crates/retrieval/src/reference/sense.rs) 逐值比较 source field、node energy/normalization/hop、edge flow/max/normalization/conductance、逐跳 transfer、抑制质量与状态截断数量，absolute/relative tolerance 为 `1e-12`，离散 membership/flag exact match。数组以 node/edge key canonicalize，保留全部中间数值。Frozen 合流 state 的单一 seedId/originType 由首次 HashMap encounter 保留，不能宣称稳定唯一 lineage；reference 数值 DTO 不输出这份任意 lineage，candidate adapter 需要区分实际来源。

独立 [ordered graph builder](../../crates/retrieval/src/reference/graph.rs) 以 file/tag/position、pair similarity、anchor gain 和 typed config 为中性输入。[graph fixture](../../crates/retrieval/tests/fixtures/vcp-graph.json) 对照 frozen native 的 30 条归一化前 fact entries、CSR 节点/行/目标/conductance、wormhole 集合、inbound mass 与逐文件 provenance，容差 `1e-12`。Fact export 只调用原 private builder，算法函数没有修改。

图构建的 reverse anchor/semantic/distance 影响 fact mass；log evidence、hub correction 与 wormhole reserve 影响预算内 transport。Provenance 使用单独的 file/direction/distance mass 合同，独立计算，避免把 transport 权重当来源质量。首个 fixture 通过不替代完整配置边界与 graph matrix。

完整 vector/candidate matrix、EPA basis builder、field vector projection、DTSC/V3 readout 与 adapter utility 继续实现。

## 需要保留的实现边界

### 观测与完整度

VCP 同一 request 的 Sense、增强向量和双场保存在同代 runtime observation 中，DTSC/V3 复用它。不同 artifact signature 清除 cache；同 signature publication 幂等，handle 有 generation fence、容量 256 与五分钟 TTL。Nous 使用 request-scoped immutable observation，无需复刻 N-API handle。

VCP Sense 的 source field 是最终节点 FIR 能量的总量归一值，并非 hop-0 seed。边流在 next momentum 检查之前记录；进入河网的注入不必进入下一 hop state。Trace transition 只保留被接受的目的状态，有独立上限。Native V3 从 cached observation 设置的 `complete_observation` 只检查 source field 非空，未结合 state truncation、transition truncation 或双场 convergence。Reference 应重现此语义，Nous diagnostics 应分别保存可观测事实。

### DTSC

Exact contact 读取完整正能量场；vector interpolation 另受 mass-ratio/top-node 支持集合限制。几何 auxiliary 与 identity anchor 是有界 reward floor，不是无条件相加奖励。Low trust fallback 清空 bonus 并保留输入顺序；只恢复原 KNN 分数再排序不符合该合同。

### RiverMemo V3

候选使用文件 Tag 的稳定序位；relative topology 分别比较节点、边、方向、距离与独立来源比例。当前 motif score 复用 edge topology score，没有单独候选 fork/merge 模体匹配。

Morphology 与 Ω 只读取 query river。Morphology 的 active degrees 来自正 flow，sample confidence 使用数组 node/edge count。Ω 的 edge component 使用全部 edge count，flow entropy 只使用正 raw flow；单条正 flow 为 0.5，无正 flow 为 0，各 component 在 geometric mean 前取 epsilon 下限。

Direct Anchor 的 semantic similarity 用于选择 contact；contact contribution 使用固定 semantic discount。候选池 rarity 依赖完整 selected pool。Conditional innovation 同时需要高于条件期望和 uncertainty，不能以裸 graph score 替代。Role caps/部分 frontier 常量在当前 Rust 中固定，不全部从 rag_params maps 读取。

V3 pure score 直接混合 original/local/transfer cosine，加有界 path reward；enhanced vector 用于 superset，不是第四个 pure semantic 分量。BM25/time 参与候选来源，不直接加入 pure final 分。Nous adapter 只能生成 TopologyWave lane 内的排序和可解释 metadata，最终仍由 Runtime fixed fusion 与 Authority revalidation 决定。

[返回研究入口](README.md)
