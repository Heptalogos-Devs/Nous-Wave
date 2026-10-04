# VCP source conformance

## 来源与状态

本轮冻结 `lioensky/VCPToolBox@e03b891d42055cdf3cab5dc961cc71dd5facd65a`。精确文件 blob 与研究基线由 [source manifest](corpus/cognitive-retrieval-sources.json) 拥有。本页描述已读 production Rust 合同及当前差异；独立 reference kernel 已开始实现，完整 fixture matrix 与 Nous adapter 尚未完成，不能将源码研究写成 `reference-parity`。

已完整研究 `memo_pipeline.rs`、`memo_sensing.rs`、`memo_artifact_builder.rs`、`memo_dtsc.rs`、`rivermemo_topology_v3.rs`、`TagMemoEngine.js`、`rag_params.json`、要求的 Deep Dive/Topology V3 文档及 unified geometry probe。已进一步读取 lib.rs 的 EPA/Intrinsic Residual owner、TagMemoV10 配置快照、RiverMemo 的 native 参数映射和 RAGDiaryPlugin 的 TimeDecay 调用与实现。参数须以 frozen effective config 为准：Rust fallback defaults 与 rag_params 的 alpha、support count、contact thresholds、language penalty 有差异。

VCP 为 CC BY-NC-SA 4.0，Nous 为 MIT。本页只记录独立提取的数学合同；Nous 实现不包含第三方源码。Reference fixture 将保存数值输入、输出、浮点容差与来源身份。

## 分项矩阵

`Label` 描述当前 Nous 实现与该项 reference 的关系；未完成的新 reference/adapter 明确标记待实现。参数相同不代表语义一致，排序相同也不能替代中间数值 parity。

| Component | Frozen VCP owner | Exact contract | Nous current | Reference kernel | Nous adapter | Label |
| --- | --- | --- | --- | --- | --- | --- |
| EPA | memo_pipeline analyze_epa；lib.rs EpaBasisTask | f32 query centering，basis dot 的平方能量概率，归一化熵、dominant axes 与 bridge resonance；query analysis 不读取 cached training energies | weighted representative PCA 与当前 EPA policy | query analysis 首个 native 数值 fixture 通过；basis builder 待实现 | 待实现 | uncertain |
| Residual | memo_pipeline analyze_pyramid / orthogonal_projection；lib.rs IntrinsicResidualTask | 每层 residual ANN，按返回序正交化 Tag，dependent axis contribution 为零；f32 投影累积，解释量归一到原 query energy | bounded residual search 与 Nous own residual contract | neutral Residual Pyramid 四个 actual ANN snapshot case 通过；intrinsic residual kernel 待实现 | 待实现 | nous-native |
| graph build | memo_artifact_builder build_fact_matrix / build_transport | 有序 file-tag 全 pair；位置/距离/semantic/reverse anchor 调制；log evidence、target inflow hub 校正与预算内 wormhole reserve | CognitiveRef/AssociationEvidence，support class quality 与独立 provenance root max dedup | neutral ordered graph builder 首个 native fact/CSR/provenance fixture 通过；完整 graph matrix 待补 | 待实现 | nous-native |
| bounded Sense | memo_sensing sense_typed | energy、momentum、hop、previous-node state；normal/wormhole decay、立即回流抑制、FIR、邻居/state 上限 | Wave 的固定 outbound、步数预算与 state 上限；无 VCP wormhole/decay 合同 | neutral Sense 数值与状态 matrix 通过；单一 lineage 边界单独记录 | 待实现 | nous-native |
| query river | memo_sensing SenseOutput | node potential 与实际注入 edge flow；source field 为最终 retained FIR 分布；edge 记录先于 momentum/state admission | QueryRiver 包含 source/potential/edge/provenance/mass；native source 为初始归一 seed | Sense node/edge/source field 与逐跳 transfer matrix 通过；VCP 单一 lineage 不作为稳定合同 | 待实现 | nous-native |
| dual fields | memo_pipeline solve_dual_fields | 同一 source/transport 的两个 scaled resolvent，独立 L1 convergence、mass-ratio domains 与 vector projection | 尚无该 VCP profile 的双场 readout | neutral 双 resolvent 首个 native 数值 fixture 通过；vector projection 待实现 | 待实现 | vcp-derived |
| DTSC | memo_dtsc score_curve / run | ordered curve 的 exact/interpolated field contacts、coverage、continuity、action、closure、D/S/T、受限 reward floors；low trust 保留原输入顺序 | 当前 native 以 node potential 排序 | 待实现 | 待实现 | absent |
| relative topology | rivermemo_topology_v3 evaluate_topology | exact/semantic node 对应，hop/position 相对距离、方向、独立 file 来源比例；node-only reliability cap；motif 复用 edge score | 当前 native node-potential 没有此 candidate readout | neutral 13-case native node/edge/component/reliability 对照通过 | 待实现 | vcp-derived |
| morphology | rivermemo_topology_v3 compute_query_morphology | candidate/text 无关的 river 统计，三个 logits stable softmax，按样本/complete confidence 向 uniform prior 收缩 | 无三形态混合 | neutral 19-case native features/probabilities/discrete mode 对照通过 | 待实现 | vcp-derived |
| Ω | rivermemo_topology_v3 compute_omega | edge activation、emergence、正 raw flow entropy 的 epsilon-floored geometric mean，乘完整度因子 | native complete/discarded mass，尚无 Ω | neutral 19-case native Ω components/regime 对照通过 | 待实现 | vcp-derived |
| Direct Anchor | rivermemo_topology_v3 compute_anchors | hop-0 seed/core exact 或高阈值 semantic contact，mass/specificity/closure/pool rarity noisy-OR；缺少 lineage 的 fallback 限 reliability | exact lane 与 native seeds，不等于 VCP 独立 anchor reward | neutral 11-case native contact/pool rarity/noisy-OR/reliability 对照通过 | 待实现 | vcp-derived |
| conditional innovation | rivermemo_topology_v3 assign_v3_scores | 条件 peer Gaussian 期望、variance/ESS uncertainty，正 innovation 经 role cap 与 Ω gate；独立 batch anchor activation | native topology rank 经 Runtime fixed fusion，无此读出 | neutral 17-case native peer statistics/role caps/anchor activation 对照通过；完整 candidate pipeline 待接入 | 待实现 | vcp-derived |
| candidate observables / pure score | rivermemo_topology_v3 evaluate_observables / run_native | source contact、semantic boundary、双场覆盖/potential、closure；original/local/transfer cosine 与 bounded path reward | native 使用 own node-potential | observables 14-case native 对照、其中 5 个实际 selected curve 的 pure score 对照通过；graph mixture 与完整流水线待接入 | 待实现 | vcp-derived |
| TimeDecay | RAGDiaryPlugin modifiers / _applyTimeDecay | 显式修饰符启用，V3 后、外部 rerank/截断前，以文本/路径日期作半衰期乘法；time lane 跳过乘法；native timeScore 仅进入 superset | Runtime own temporal hard constraints/age preference，不声明 VCP parity | owner 已确认；独立 temporal 实验待完成 | 待实现 | nous-native |

## 当前数值 conformance

独立 checkout 的 frozen native 模块已编译。只读 research export 暴露同一 cached observation；条件创新 probe 另记录原函数局部统计值，原 score 表达式保持不变。原 blob、instrumentation 与 binary checksum 存在 ignored `data/research/vcp-reference/e03b891d42055cdf3cab5dc961cc71dd5facd65a/source-manifest.json`。

首个三维图 probe 实际执行 EPA publish、intrinsic residual、artifact build、统一 pipeline、DTSC 与 V3，导出 mean/basis/energy、residual/anchor、CSR、Sense 转移/node/edge、双场、morphology、Ω 与 candidate components/order。Tracked [数值 fixture](../../crates/retrieval/tests/fixtures/vcp-epa-dual-fields.json) 只包含生成的中间数值与来源身份，独立 [reference kernel](../../crates/retrieval/src/reference/fields.rs) 使用中性 DTO。

当前通过 query EPA 的 entropy/depth/resonance/axis energy，以及双场每个 node mass、支持域、迭代次数、convergence 与 L1 residual；absolute/relative tolerance 均为 `1e-10`。该 EPA/双场组仍只覆盖一个 component fixture。另有 [Sense graph matrix](../../crates/retrieval/tests/fixtures/vcp-sense.json)，由 frozen `sense_typed` 实际执行生成 chain、fork、merge、立即回流、hub、两独立 roots、wormhole bridge、disconnected distractor、weak/strong edges、state truncation、empty seeds 和 transition-record truncation 共 12 个 case。

独立 [Sense kernel](../../crates/retrieval/src/reference/sense.rs) 逐值比较 source field、node energy/normalization/hop、edge flow/max/normalization/conductance、逐跳 transfer、抑制质量与状态截断数量，absolute/relative tolerance 为 `1e-12`，离散 membership/flag exact match。数组以 node/edge key canonicalize，保留全部中间数值。Frozen 合流 state 的单一 seedId/originType 由首次 HashMap encounter 保留，不能宣称稳定唯一 lineage；reference 数值 DTO 不输出这份任意 lineage，candidate adapter 需要区分实际来源。

独立 [ordered graph builder](../../crates/retrieval/src/reference/graph.rs) 以 file/tag/position、pair similarity、anchor gain 和 typed config 为中性输入。[graph fixture](../../crates/retrieval/tests/fixtures/vcp-graph.json) 对照 frozen native 的 30 条归一化前 fact entries、CSR 节点/行/目标/conductance、wormhole 集合、inbound mass 与逐文件 provenance，容差 `1e-12`。Fact export 只调用原 private builder，算法函数没有修改。

图构建的 reverse anchor/semantic/distance 影响 fact mass；log evidence、hub correction 与 wormhole reserve 影响预算内 transport。Provenance 使用单独的 file/direction/distance mass 合同，独立计算，避免把 transport 权重当来源质量。首个 fixture 通过不替代完整配置边界与 graph matrix。

[Residual Pyramid fixture](../../crates/retrieval/tests/fixtures/vcp-pyramid.json) 固定 actual pipeline 的每层 ANN 返回候选，覆盖 duplicate directions、weak residual、dominant direction 与 empty query。独立 [Pyramid kernel](../../crates/retrieval/src/reference/pyramid.rs) 按该顺序正交化，检查每个 Tag contribution/handshake magnitude、projection/residual magnitude、energy ratio/explained、direction coherence/pattern/noise，以及最终 depth/coverage/novelty/activation。弱残差 case 实际经过三层，避免单层满秩例子掩盖层间合同。

[Query shape fixture](../../crates/retrieval/tests/fixtures/vcp-query-shape.json) 使用 frozen `compute_query_morphology` 与 `compute_omega` 的只读 export，包含 Sense matrix 的河网以及独立深链、同层关系、空/不完整观测、单边、零 raw flow/正 normalized flow 和 Ω scale override，共 19 个 case。独立 [query shape kernel](../../crates/retrieval/src/reference/query_shape.rs) 对照所有形态 features、confidence、三个概率、dominant mode，以及 Ω edge/emerge/flow/最终值与 regime；数值容差 `1e-12`，离散结果 exact match。

Morphology 的正 normalized flow 可以形成方向统计；Ω entropy 使用正 raw flow。这个边界已有单独 fixture。输入不包含 query text 或 candidate，保证这两个 kernel 的测量对象是 query observation。

[Direct Anchor fixture](../../crates/retrieval/tests/fixtures/vcp-anchors.json) 从 frozen `compute_anchors`/`anchor_contacts` 导出 11 个中性曲线 case，包括 exact/semantic、fallback、无 inbound、零 mass、缺 seed vector、semantic discount、全池共同 anchor、稀有 exact anchor、负正文 closure、空 seeds/pool。独立 [anchor kernel](../../crates/retrieval/src/reference/anchors.rs) 对照 score/reliability/strength、exact/semantic/contact 数与 mean closure，容差 `1e-12`。Rarity 先读取完整 selected pool，之后各候选独立读出；semantic cosine 用于选择 contact，奖励使用固定 discount。Hop-0 seed/core 的来源筛选仍由统一 pipeline/adapter 负责，不能将此 kernel 接受的任意 seed DTO 当作真实直接来源证明。

这些 fixture 证明给定候选序列时的数值 kernel；ANN 搜索排序和 ontology mapping 仍属于 adapter utility。[Path geometry fixture](../../crates/retrieval/tests/fixtures/vcp-path.json) 从 frozen `evaluate_path` 生成 10 个 case：正反有序曲线、transfer-only bridge、支持域外尾部、缺失 conductance、单 Tag、空曲线、负正文 closure、权重/closure override 和空 fields。独立 [path kernel](../../crates/retrieval/src/reference/geometry.rs) 复用中性 Curve DTO，按 max-normalized field 读取相邻段，逐值比较 local/transfer potential、direction、continuity、support/transfer count、正文 closure、path core/quality，容差 `1e-12`。

支持标记由有效域和边存在性计算，质量还取决于实际 field mass；因此支持 count 非零不等于 path quality 非零。单 Tag 使用受限节点读出，仍保留段 count 为零。此处验证数值 kernel，未将正反 case 的差值外推为实际 retrieval utility。

[Relative topology fixture](../../crates/retrieval/tests/fixtures/vcp-relative-topology.json) 通过 frozen `evaluate_topology` 导出 13 个中性 case，涵盖正反/semantic 曲线、多个 query nodes 对应同一 candidate tag、node-only/无对应、self/independent source、weak edge/cap、distance/direction override、负正文 closure 和空 river。独立 [topology kernel](../../crates/retrieval/src/reference/topology.rs) 对照 node/edge coverage、alignment、distance/direction/edge/motif 分量、两个 graph heads、最终 score/reliability/mode，容差 `1e-12`。来源贡献按 file ID 读取独立比例，保留 frozen `0.15` self-evidence floor。

当前 motif 分量与 edge topology 分量严格相同，没有额外图同构判定。缺少完整 edge 对应时采用 node-only readout，并限制 reliability；分数与 reliability 分开记录。

完整 vector/candidate matrix、EPA basis builder、intrinsic residual/gating/fusion、field vector projection、条件创新/DTSC/V3 final readout 与 adapter utility 继续实现。

独立 [V3 scoring head](../../crates/retrieval/src/reference/scoring.rs) 与 [17-case fixture](../../crates/retrieval/tests/fixtures/vcp-scoring.json) 比较实际函数记录的 peer expectation、variance、ESS、uncertainty、positive innovation、candidate/statistical confidence、requested bonus 和 peer count。Atomic/propositional/narrative、单候选/两候选/空池、Gaussian fallback、direct frontier、thematic cap、低/零 Ω、batch anchor promotion/平滑激活/饱和以及参数覆盖均通过 `1e-12` absolute/relative 对照，角色与数量 exact match。Anchor promotion 与 anchor reward 是两个独立决策：默认 z=2 的五候选案例中，最强 anchor 可提升角色而未超过激活阈值，加分仍为零。此组以 precomputed pure/graph/closure 等中性标量为输入；完整 candidate observables、pure score、候选池构建及 Nous adapter 尚未由此组证明。

独立 [observables / pure score](../../crates/retrieval/src/reference/observables.rs) 另使用 [14-case fixture](../../crates/retrieval/tests/fixtures/vcp-observables-pure.json)。五个同一次 native run selected curve 使用数据库实际 f32 Tag/chunk 向量和 cached original/local/transfer vector，逐项比较 observables 与 pure score；九个附加 observables case 覆盖 hidden direct、零/反向 query、空 curve、无 source、无场、tail-only、local-only 和重复 source ID，使用原 `evaluate_observables` 直接生成期望，均以 `1e-12` 对照通过。此组的 geometry/topology/morphology 为条件输入，尚未证明候选池选择、完整 graph mixture 或统一 candidate 流水线。

## 需要保留的实现边界

### 观测与完整度

VCP 同一 request 的 Sense、增强向量和双场保存在同代 runtime observation 中，DTSC/V3 复用它。不同 artifact signature 清除 cache；同 signature publication 幂等，handle 有 generation fence、容量 256 与五分钟 TTL。Nous 使用 request-scoped immutable observation，无需复刻 N-API handle。

VCP Sense 的 source field 是最终节点 FIR 能量的总量归一值，并非 hop-0 seed。边流在 next momentum 检查之前记录；进入河网的注入不必进入下一 hop state。Trace transition 只保留被接受的目的状态，有独立上限。Native V3 从 cached observation 设置的 `complete_observation` 只检查 source field 非空，未结合 state truncation、transition truncation 或双场 convergence。Reference 应重现此语义，Nous diagnostics 应分别保存可观测事实。

### DTSC

Exact contact 读取完整正能量场；vector interpolation 另受 mass-ratio/top-node 支持集合限制。几何 auxiliary 与 identity anchor 是有界 reward floor，不是无条件相加奖励。Low trust fallback 清空 bonus 并保留输入顺序；只恢复原 KNN 分数再排序不符合该合同。

### RiverMemo V3

JS 控制面只转交部分配置。`TagMemoV10Engine.getEffectiveConfig()` 合并 laboratory/riverMemo 并冻结 nested defaults；原生资产构建保留同代 `orderedCooccurrence`、`v9`、`spikeRouting`。`RiverMemoEngine._nativeConfig()` 没有转交 pure original/local/transfer weights、role cap/multiplier maps 和 frontier 常量，对应 production Rust 固定值须与实验覆盖项区分。

完整源码表明 `TagMemoEngine.applyTagBoost()` / `geodesicRerank()` 在 Rust-owned 资产下被显式停用。其旧 `observeQueryForV10()` 使用 enhanced vector delta 判断完整度；当前 native cached observation 使用 source field 非空，不能混合这两种合同。Deep Dive 中关于私有概念和普遍收益的描述属于上游观测，本轮检索质量仍由独立 benchmark 判定。

Unified geometry probe 是离线研究程序：采样/裁剪图、忽略 association reserve 的简化 kernel、叠加 decay 的 transport、无生产 momentum/state cap 的单种子有限场，以及简化 curve 泛函。它的同泛函两乘两设计、去源归一化和随机/拓扑消融可用于实验设计，不能用其输出代替 production DTSC/V3 source conformance。

候选使用文件 Tag 的稳定序位；relative topology 分别比较节点、边、方向、距离与独立来源比例。当前 motif score 复用 edge topology score，没有单独候选 fork/merge 模体匹配。

Morphology 与 Ω 只读取 query river。Morphology 的 active degrees 来自正 flow，sample confidence 使用数组 node/edge count。Ω 的 edge component 使用全部 edge count，flow entropy 只使用正 raw flow；单条正 flow 为 0.5，无正 flow 为 0，各 component 在 geometric mean 前取 epsilon 下限。

Direct Anchor 的 semantic similarity 用于选择 contact；contact contribution 使用固定 semantic discount。候选池 rarity 依赖完整 selected pool。Conditional innovation 同时需要高于条件期望和 uncertainty，不能以裸 graph score 替代。Role caps/部分 frontier 常量在当前 Rust 中固定，不全部从 rag_params maps 读取。

V3 pure score 直接混合 original/local/transfer cosine，加有界 path reward；enhanced vector 用于 superset，不是第四个 pure semantic 分量。BM25/time 参与候选来源，不直接加入 pure final 分。Nous adapter 只能生成 TopologyWave lane 内的排序和可解释 metadata，最终仍由 Runtime fixed fusion 与 Authority revalidation 决定。

`::TimeDecay` 是 RAGDiaryPlugin 的后处理，位于 RiverMemo 返回后、外部 rerank/最终截断前。半衰期、最低分优先取修饰符，其次全局配置，fallback 分别为 30 天和 0.5；日期依次取 Tag 行、括号、首行、路径。对有效日期使用非负整数日龄，原分优先 `rerank_score`，乘 `0.5^(days/halfLife)` 后排序并按最低分过滤。`source=time` 跳过衰减乘法，仍参加全批过滤；未匹配 target Tag、无日期或日期无效的候选保留原分。此合同不等于 Nous 的多轴时间约束，后续比较须分别标识。

[返回研究入口](README.md)
