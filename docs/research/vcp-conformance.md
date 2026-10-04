# VCP source conformance

## 来源与状态

本轮冻结 `lioensky/VCPToolBox@e03b891d42055cdf3cab5dc961cc71dd5facd65a`。精确文件 blob 与研究基线由 [source manifest](corpus/cognitive-retrieval-sources.json) 拥有。本页描述已读 production Rust 合同及当前差异；独立 reference kernel 已开始实现，完整 fixture matrix 与 Nous adapter 尚未完成，不能将源码研究写成 `reference-parity`。

已完整研究 `memo_pipeline.rs`、`memo_sensing.rs`、`memo_artifact_builder.rs`、`memo_dtsc.rs`、`rivermemo_topology_v3.rs`、`TagMemoEngine.js`、`rag_params.json`、要求的 Deep Dive/Topology V3 文档及 unified geometry probe。已进一步读取 lib.rs 的 EPA/Intrinsic Residual owner、TagMemoV10 配置快照、RiverMemo 的 native 参数映射和 RAGDiaryPlugin 的 TimeDecay 调用与实现。参数须以 frozen effective config 为准：Rust fallback defaults 与 rag_params 的 alpha、support count、contact thresholds、language penalty 有差异。

VCP 为 CC BY-NC-SA 4.0，Nous 为 MIT。本页只记录独立提取的数学合同；Nous 实现不包含第三方源码。Reference fixture 将保存数值输入、输出、浮点容差与来源身份。

## 分项矩阵

`Label` 描述当前 Nous 实现与该项 reference 的关系；未完成的新 reference/adapter 明确标记待实现。参数相同不代表语义一致，排序相同也不能替代中间数值 parity。

| Component | Frozen VCP owner | Exact contract | Nous current | Reference kernel | Nous adapter | Label |
| --- | --- | --- | --- | --- | --- | --- |
| EPA | memo_pipeline analyze_epa；lib.rs EpaBasisTask | f32 query centering，basis dot 的平方能量概率，归一化熵、dominant axes 与 bridge resonance；query analysis 不读取 cached training energies | weighted representative PCA 与当前 EPA policy | query analysis 首个 native fixture；density sampler / weighted f32 basis builder 8 个 native case 通过 | 待实现 | uncertain |
| Residual | memo_pipeline analyze_pyramid / orthogonal_projection；lib.rs IntrinsicResidualTask | 每层 residual ANN，按返回序正交化 Tag，dependent axis contribution 为零；f32 投影累积，解释量归一到原 query energy | bounded residual search 与 Nous own residual contract | Pyramid 四个 actual ANN snapshot case；Intrinsic Residual 16 个 native task case 通过 | 待实现 | nous-native |
| graph build | memo_artifact_builder build_fact_matrix / build_transport | 有序 file-tag 全 pair；位置/距离/semantic/reverse anchor 调制；log evidence、target inflow hub 校正与预算内 wormhole reserve | CognitiveRef/AssociationEvidence，support class quality 与独立 provenance root max dedup | neutral ordered graph builder 首个 native fact/CSR/provenance fixture 通过；完整 graph matrix 待补 | 待实现 | nous-native |
| bounded Sense | memo_sensing sense_typed | energy、momentum、hop、previous-node state；normal/wormhole decay、立即回流抑制、FIR、邻居/state 上限 | Wave 的固定 outbound、步数预算与 state 上限；无 VCP wormhole/decay 合同 | neutral Sense 数值与状态 matrix 通过；单一 lineage 边界单独记录 | 待实现 | nous-native |
| query river | memo_sensing SenseOutput | node potential 与实际注入 edge flow；source field 为最终 retained FIR 分布；edge 记录先于 momentum/state admission | QueryRiver 包含 source/potential/edge/provenance/mass；native source 为初始归一 seed | Sense node/edge/source field 与逐跳 transfer matrix 通过；VCP 单一 lineage 不作为稳定合同 | 待实现 | nous-native |
| dual fields | memo_pipeline solve_dual_fields | 同一 source/transport 的两个 scaled resolvent，独立 L1 convergence、mass-ratio domains 与 vector projection | 尚无该 VCP profile 的双场 readout | neutral 双 resolvent 首个 native fixture、field vector projection 11 个 native case 通过 | 待实现 | vcp-derived |
| query gating / fusion | memo_pipeline gate_tags / fuse_observation | EPA/pyramid 激活、core/language/layer 门控；seed max、emergent cap、传播后 core/ghost、20% dedup 权重转移与向量融合 | native 使用 own sensing/activation | gating 16-case、fusion 17-case native 中间值对照通过；完整 query composition 待接入 | 待实现 | vcp-derived |
| DTSC | memo_dtsc score_curve / run | ordered curve 的 exact/interpolated field contacts、coverage、continuity、action、closure、D/S/T、受限 reward floors；low trust 保留原输入顺序 | 当前 native 以 node potential 排序 | field 14-case、完整 curve/reward/batch fallback/order 22-case native 对照通过 | 待实现 | vcp-derived |
| relative topology | rivermemo_topology_v3 evaluate_topology | exact/semantic node 对应，hop/position 相对距离、方向、独立 file 来源比例；node-only reliability cap；motif 复用 edge score | 当前 native node-potential 没有此 candidate readout | neutral 13-case native node/edge/component/reliability 对照通过 | 待实现 | vcp-derived |
| morphology | rivermemo_topology_v3 compute_query_morphology | candidate/text 无关的 river 统计，三个 logits stable softmax，按样本/complete confidence 向 uniform prior 收缩 | 无三形态混合 | neutral 19-case native features/probabilities/discrete mode 对照通过 | 待实现 | vcp-derived |
| Ω | rivermemo_topology_v3 compute_omega | edge activation、emergence、正 raw flow entropy 的 epsilon-floored geometric mean，乘完整度因子 | native complete/discarded mass，尚无 Ω | neutral 19-case native Ω components/regime 对照通过 | 待实现 | vcp-derived |
| Direct Anchor | rivermemo_topology_v3 compute_anchors | hop-0 seed/core exact 或高阈值 semantic contact，mass/specificity/closure/pool rarity noisy-OR；缺少 lineage 的 fallback 限 reliability | exact lane 与 native seeds，不等于 VCP 独立 anchor reward | neutral 11-case native contact/pool rarity/noisy-OR/reliability 对照通过 | 待实现 | vcp-derived |
| conditional innovation | rivermemo_topology_v3 assign_v3_scores | 条件 peer Gaussian 期望、variance/ESS uncertainty，正 innovation 经 role cap 与 Ω gate；独立 batch anchor activation | native topology rank 经 Runtime fixed fusion，无此读出 | neutral 17-case native peer statistics/role caps/anchor activation 对照通过；已组装 V3 candidate readout，query preparation 待接入 | 待实现 | vcp-derived |
| candidate observables / pure score | rivermemo_topology_v3 evaluate_observables / run_native | source contact、semantic boundary、双场覆盖/potential、closure；original/local/transfer cosine 与 bounded path reward | native 使用 own node-potential | observables 14-case native 对照、其中 5 个实际 selected curve 的 pure score 对照通过；另有 11-case 组合 readout | 待实现 | vcp-derived |
| candidate pool / V3 readout composition | rivermemo_topology_v3 select_superset / run_native | 七路 channel quota/归一化/union score/source count；同一 observation 的 path/topology/anchor、形态混合、条件创新、最终排序 | native 使用 own TopologyWave lane | pool 12-case、cached observation 到完整 V3 readout 11-case native 对照通过；query preparation 与 production adapter 待接入 | 待实现 | vcp-derived |
| unified query preparation | memo_pipeline run_pipeline | 同一个 query 的 EPA/pyramid/gating/Sense/fusion/dual fields/projection，供 DTSC/V3 共享 | 已有 native shared signals；VCP 查询准备尚待 adapter | 首个 full native observation 组合 fixture 通过；ANN 返回次序作为条件输入 | 待实现 | vcp-derived |
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

独立 [candidate pool](../../crates/retrieval/src/reference/pool.rs) 的 [12-case fixture](../../crates/retrieval/tests/fixtures/vcp-candidate-pool.json) 比较 native `select_superset` 的来源、union score、quota、cap 和次序。独立 [V3 readout](../../crates/retrieval/src/reference/readout.rs) 再从同一 cached observation、完整 offered curves、持久 f32 向量和冻结 CSR/provenance 计算全部 candidate 读出；[11-case fixture](../../crates/retrieval/tests/fixtures/vcp-v3-readout.json) 使用实际原生 `run_native` 调用，覆盖默认、source quota、union cap、最终截断、BM25/time 补充、无 seed lineage、node-only river、不完整观测、空双场、visible direct 范围与联合参数覆盖。Morphology/Ω、selected pool、最终 membership/顺序、geometry/topology/observables/anchor、pure/final score 和三种 bonus 均通过 `1e-12` 对照；baseline 为六个 offered、五个 selected，最终次序 `4,1,6,2,3`。

这个组合组的 query observation 与图资产是冻结条件输入，没有重复执行 Sense 或求场；EPA training、intrinsic residual、query gating/fusion/projection、DTSC 和 Nous production adapter 仍未由这组完成。Native `allowedFileIds` 在此 readout 中仅控制 observable direct visibility，不能作为 Nous Authority 的权限证据；adapter 必须消费经过 Authority 限定的候选与图视图。

独立 [DTSC field preparation / sampling](../../crates/retrieval/src/reference/dtsc_field.rs) 使用 [14-case fixture](../../crates/retrieval/tests/fixtures/vcp-dtsc-field.json)。期望值由原生 `run` 的实际局部场数据及原 `sample_field` 生成，比较完整 exact field、保留 interpolation nodes/向量、总能量/最大值/熵、trust flag 和采样势能/exact/source type。案例包含质量与数量截断、插值子集外 exact contact、sourceField-only、空场、低熵及关闭熵守卫、缺向量、维度不符、energy fallback、重复 node、配置夹逼与负语义阈值，均通过 `1e-12`。

独立 [DTSC curve kernel](../../crates/retrieval/src/reference/dtsc_curve.rs)、[typed config](../../crates/retrieval/src/reference/dtsc_config.rs) 与 [batch readout](../../crates/retrieval/src/reference/dtsc.rs) 已接入完整场准备。[22-case fixture](../../crates/retrieval/tests/fixtures/vcp-dtsc.json) 由实际 `rerankMemoDtsc` 生成，核对全部数值 curve/reward/diagnostic 字段与完整 membership/order；Tag 文本标签由 adapter 映射，不作为中性 numerical kernel 输出。案例覆盖 native defaults、实际 rag 配置、低支持/低熵/缺向量/无接触回退、sourceField-only、emergent structural、两个 semantic direct contacts、thematic、非零 sparse pair relief、非零 geometry/identity floor、联合低覆盖/低区分度、缺失曲线、Top-K、空候选、维度错配、零维度拒绝和冲突参数夹逼。数值 tolerance 为 `1e-12`，membership/enum/count exact；零维度按输入错误拒绝。

DTSC 的正常最终分为原 KNN 分加 bounded reward，不沿用 V3 的 `[0,1]` final clamp。批级回退清零 base/aux/total bonus 并保留输入次序；`fallbackToKnnOnLowTrust=false` 关闭的是联合低信任/熵守卫，最小支持、向量可用性和无贡献回退仍存在。此组资产与查询观察为冻结输入，query preparation、完整矩阵要求与 production adapter/benchmark 仍需完成。

独立 [Intrinsic Residual builder](../../crates/retrieval/src/reference/intrinsic.rs) 使用 [16-case fixture](../../crates/retrieval/tests/fixtures/vcp-intrinsic.json)，由实际 native task 在独立 SQLite 文件计算并读取持久 value/status。包括 anchored GS/centroid/SVD、basis 限制、未归一化输入、缺少 pairwise、关闭语义门控、hard floor、min gain、抵消 centroid、零向量、空/超大 file、legacy position、固定 anchor mapping 与配置夹逼。ID、status、neighbor count exact；raw ratio/anchor gain 的 absolute/relative tolerance 为 `1e-10`，SVD 使用 f32、不同 nalgebra 版本的 `1e-6` tolerance。没有把全库 median 引入 anchor mapping。

Frozen task 的等权邻居顺序来自 HashMap；独立 builder 用稳定 ID 起序，不声称任意 tie 的唯一 basis/lineage parity。资产签名、数据库 cache/persistence 属于 adapter/lifecycle，数值 fixture 没有替代这些生产要求。

独立 [field vector projection](../../crates/retrieval/src/reference/fields.rs) 的 [11-case fixture](../../crates/retrieval/tests/fixtures/vcp-field-projection.json) 实际调用原 `project_field`，覆盖 local/transfer 场、单节点、缺失 index node、空/非正场、抵消、微小向量、未归一化向量和质量缩放。按 artifact node order 累加可用 f32 index vectors，只用可用节点质量取均值，再作 f64 单位化和 f32 输出；absolute/relative tolerance 为 `1e-7`。

独立 [Tag gating](../../crates/retrieval/src/reference/gating.rs) 的 [16-case fixture](../../crates/retrieval/tests/fixtures/vcp-gating.json) 直接调用原 `gate_tags`，核对输出 seed ID/name/weight/core、effective boost 与 dynamic core boost。覆盖技术/社会/Unknown world、语言开关、core casefold、零相似度回退、非正节点、首次重复占位、空 pyramid、range fallback 与参数夹逼，数值 tolerance 为 `1e-12`。

独立 [fusion](../../crates/retrieval/src/reference/fusion.rs) 的 [17-case fixture](../../crates/retrieval/tests/fixtures/vcp-fusion.json) 使用原 `fuse_observation`、实际 index vectors 与 SQLite core-name lookup。核对 seed max/emergent 排序与 cap、core 补全、hard/soft ghost 和无效 ghost、缺/零向量、20% dedup 转移与 core 属性、全部诊断计数/selected IDs/weights，以及最终向量，数值 tolerance 为 `1e-12`。空 selected 保持原 query 而非强制单位化；已在 emergent 中的 core ID 不被补全重复升级。以上仍为 component fixture，完整 query pipeline 与 Nous adapter 尚待接入。

独立 [EPA training](../../crates/retrieval/src/reference/epa_training.rs) 的 [8-case fixture](../../crates/retrieval/tests/fixtures/vcp-epa-training.json) 核对原 `select_epa_density_residual_samples` 和实际 native compute/publish cache。案例包含冻结十 Tag、六维密集分布、anchor 数量上下界、单 basis、未归一化输入、七 Tag 不足和十二维能量截断。密度 mean/key、centroid、weights、labels、representative count/bucket count 与 weighted mean 对照通过；sampling f32 tolerance 为 `1e-7`，SVD basis 以符号等价、energy 以 absolute/relative `1e-5` 比较不同 nalgebra f32 实现。十二维案例实际保留 11 axes。

Sampler 的 xorshift 12-bit density key、残差/密度评分、多样性衰减、candidate `swap_remove` 次序、f32 加权 SVD 和 95% 能量/minimum-eight-axis 规则均由独立实现承担。Representative samples 仅影响诊断数量，不被追加为 SVD 行。参数化 `samples_per_anchor` / `candidate_limit` 对应 native 环境参数，本组验证其默认 32/512；同分 bucket 的 native HashMap 起序未声明稳定 parity。Basis 资产发布与 generation 元数据仍属于 production adapter/lifecycle 的后续工作。

独立 [query pipeline](../../crates/retrieval/src/reference/pipeline.rs) 的 [首个组合 fixture](../../crates/retrieval/tests/fixtures/vcp-query-pipeline.json) 使用同一次实际 native `run_pipeline` 输出，从 query/cache basis/Tag vectors/图资产进入 EPA、Pyramid、门控、Sense、融合、双场及投影。EPA stats、Pyramid/gating、完整 source/node/edge 数值、Sense 诊断、fusion 计数/selected IDs、双场每节点质量/domain/convergence/residual 对照通过；f32 enhanced/local/transfer vector 使用 `1e-7` tolerance。Native 任意合流 lineage 不作为唯一稳定来源输出；ANN 实际返回的候选次序与相似度仍是冻结输入。此组只覆盖首个组合案例，剩余查询矩阵、生产索引 adapter/资产生命周期、Authority 映射和 benchmark 待完成。

## Nous adapter 映射进度

[VCP adapter identity/evidence boundary](../../crates/retrieval/src/vcp_adapter.rs) 已实现 generation-local `CognitiveRef` 双向映射、中性 candidate curve 与 reference ranking 到 LaneCandidate 的转换。输入引用稳定排序并去重，ID 从 1 起；输出引用必须属于同一映射，未知/非正 ID 拒绝。Curve member 使用同维、有限 embedding，保留真实 source sequence；无 Authority 序位时明确使用 `stable_identity` 控制顺序，不能据此宣称 narrative/causal order。

Positive AssociationEvidence projection 保留 support class、association kind 与独立 provenance root，按 directed edge/root/class/kind 的重复输入取最大 support mass，不把重复记录当作独立支持；negative evidence 不产生 reference 正向 flow。此变换不写回 Authority，不改变 acceptance/evidence 语义。最终 ranking 去重并映射原 CognitiveRef，保留 kernel metadata；这些 ID 映射检查不能代替 Runtime Authority revalidation。

映射聚焦检查覆盖输入次序不变的 identity、source/stable curve 顺序、重复 member、维度错误、独立/重复/negative evidence 和未知输出 ID。Production graph/embedding 资产组装、Typed config、atomic generation publication、immutable VCP observation、引擎与 public query 接入仍待完成；当前 VCP profile 尚未启用。

`AuthorityStore.cognitive_projection_input()` 在同一个 repeatable-read read-only transaction 中读取 subject Authority sequence、拓扑证据和符合 capability 的 material/memory/longitudinal 文本源。原 `topology_projection_input()` 复用同一个 topology snapshot owner；`CognitiveProjectionInput.authority_watermark` 表示全 Subject sequence，另有原 topology watermark，二者不能互换。

[Serving VCP material preparation](../../crates/retrieval/src/vcp_material.rs) 复用既有文本拼装与 host-supplied embedding provider，冻结 space/producer 并检查每个输出身份/维度。它返回 generation identity map、独立证据、candidate vectors 与 Tag membership，Authority 无序 membership 明确标记 stable-identity 曲线顺序。缺已存储 embedding 时返回 unavailable；本机配置使用 StoredEmbeddingProvider，该步骤没有独立远程调用路径。

真实 PostgreSQL 的 association integration 回归同时验证 memory 开关对文本与拓扑的同步排除、独立 association provenance、缺 material、提交存储 embedding 后的候选 vectors/space/producer 和全 Authority 水位。此准备入口现在由 VCP profile generation build 消费；查询 lane/readout 仍待接线。

[Nous graph assets](../../crates/retrieval/src/vcp_graph.rs) 将同一 projection 的 Tag membership 和正向 Authority evidence 汇入独立 reference transport。无序 membership 使用 position=0 的对称 cooccurrence 分支；有实际 source sequence 时保留位置。Evidence 先按端点/独立根/support class/association kind 取最大值，再累加独立贡献；保留原 semantic identity 与质量。这里未添加新的 support-class 权重。

Reference 图 owner 现在分开构建按文档的 facts 与全图 transport，批量共享 pairwise/anchor lookup。Adapter 的 provenance root ID 使用独立命名空间，每条边保留各根贡献，根表可回译到原始 identity。它还不是 V3 candidate visibility 的 file ID；查询接线时必须由 Authority scoped candidate view 映射可见贡献。Frozen ordered graph parity 和 adapter 的无序/去重/重排回归通过，真实 PostgreSQL material 也已构建 transport。该资产构建入口现在由同代 VCP generation 消费；VCP 查询 readout 仍不可用。

[VCP generation owner](../../crates/retrieval/src/vcp_generation.rs) 持有 identity map、embedding space/producer、candidate/tag vectors 与 labels、曲线及顺序来源、cooccurrence pairwise、intrinsic residual/anchor、EPA basis 和 graph assets。缺少 concept vector 时返回 unavailable；图中缺向量的节点显式保留在 diagnostics 资产中。EPA 标签不足时保留 reference 的 cache unavailable 状态。

`retrieval.vcp.assets` 是 Developer/SystemOnly/ServingRebuild 的 typed policy，默认 EPA anchors/max basis 为 64/64，samples/candidates 为 32/512；图与 intrinsic 参数来自明确的独立数值合同。VCP `vcp.json` 使用既有 staging readback/checksum/rename/publication 流程，回读检查 generation/identity map、vector/curve identity、EPA shape 与 provenance root。ServingSnapshot 的 native/VCP 视图在同一次 publication 中互斥切换；topology implementation revision=4。VCP 配置 digest 包含 profile、asset policy、synopsis budget、space/producer 和 capability，刷新水位使用完整 Subject authority_seq。

真实 PostgreSQL 回归验证 VCP publication/reopen、native/VCP 切换、回读拒绝乱序 identity map 和曲线向量不一致。Asset policy 的 outbound mass 从 0.95 改为 0.7 时 generation 重建，每个非空 transport 行总质量变为 0.7。此次验证覆盖资产生命周期；immutable VCP query observation、候选 readout、Authority scoped provenance visibility 和公共查询仍待完成。

[VCP indexed generation](../../crates/retrieval/src/vcp_index.rs) 将数值资产与 candidate/Tag 两个 USearch index 放在同一个不可变 owner 中。三个文件使用同一次 staging/checksum/publication；回读逐 ID 检查 index vector 与原资产一致。VCP 索引显式使用 cosine metric，Dense 原有 metric 选择保持原合同。Tag residual 搜索使用 frozen pipeline 的 f64 `1/(1+f32 distance)` similarity，并从同代 label/vector 表返回输入。实际 USearch 保存/回读后，两个正交 Tag 的 residual 搜索结果一致；真实 PostgreSQL generation 的 candidate 搜索包含 MemoryRevision，空 Tag index 返回空输入。这里只证明索引接线，未声明 ANN 排序与 frozen VCP index 完全一致。

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
