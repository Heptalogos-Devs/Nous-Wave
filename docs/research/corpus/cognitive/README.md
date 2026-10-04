# Cognitive Recall Corpus

项目原创的 deterministic event graph，许可为 **CC0-1.0**。事件图和 oracle 是输入来源；检索输出不参与事实或 grade 定义。

## 数据

[manifest.json](manifest.json) 固定文件摘要、规模和当前状态。[queries.json](queries.json) 包含查询、as-of、正/负 relevance 和 justified paths；[associations.json](associations.json) 包含有依据的 directed recall edges。

三条时间线各有 100 events，覆盖 2026-05 至 2026-08，包含 10 个不同项目和跨 Session 的复发模式：

- [灯塔观测站](scenarios/observatory.json)：光学观测、校准、设备维护和复测。
- [溪湾种子园](scenarios/garden.json)：育苗、灌溉、环境控制和留种管理。
- [铜湾口述档案馆](scenarios/archive.json)：记录采集、时间同步、修复和展陈。

每个项目包含异常出现、模式调查、另一班次的近似干扰、诊断、暂定值、明确更新、维护、复核和值班提醒。每个 event 保存五轴时间、valid interval、entities、tags、relations、独立 fact 和输入 render。另一班次使用单独 Session。所有事件共享 generic 工作记录 Tag，供 hub/noise 研究；私有术语是本 corpus 自造定义。

模板复用刻意制造相似事件干扰，项目、人物、部件、设置值和日期保持不同。结果必须报告这种结构的限制；该 corpus 不代表真实用户分布。

## Oracle

249 queries，包含知识更新、1/2/3-hop、叙事因果、private concepts、spontaneous situation、五轴时间、future exclusion、邻接、复发、weak cue、hub、diffusion、interference 和历史保留。

每个显式 candidate grade 有 reason。**同一 Subject 的未列出事件**使用 `default_same_subject_oracle` 的 grade=0；其他 Subject 的候选属于隔离错误，不套用默认 grade。Importer 必须先将同 Subject 默认项展开成完整 oracle，再交给通用 metrics owner；不能把省略项当作未审定。Grade -1 单独测 harmful exposure，不放入正 nDCG。

`required_paths` 固定 directed `recalls_precursor` 图中的最短路径。As-of 查询必须在真实 owner 时间语义下执行；当前图的日期不等同于已完成 occurred/observed/valid/formed/recorded 注入。暂定设置的有效期到更新时刻结束，历史查询仍可指定先前状态。

## 当前状态

300 events、249 queries 的 authored graph 已保存，文件摘要、引用、时间顺序和 1/2/3-hop 最短路径已检查。独立 research PostgreSQL 已通过下面的 harness 导入 300 events、60 Sessions 和 207 条有来源支持的 association，并回读确认 formed/recorded 时间。**首轮 249-query × 4-profile Kernel no-rerank 前缀比较已完成；完整 Spec 评测仍未完成**。后续还需：

- 补全 importer 的外部 evidence/graph coverage 审计，并使用保存的 deterministic event → Authority revision 映射进行评测。
- 补全实际 Runtime situation 对照与历史状态检索合同；前缀调度已接线，五轴约束使用正常 Query 输入，未修改数据库时间字段。
- 在所有 profile 上使用同一 Authority snapshot，按类别比较有/无 rerank 和指定消融。
- 分别报告模板干扰、时间轴实现能力、grade assumption、future leakage、无关扩散与有用链条召回；不将规模或单一总分作为结论。

现有 ignored 的三条观测站 live Memory 是独立 wiring probe，未算作这 300 events 的导入结果。Corpus 中的 as-of、setting grade 尚未经过完整真实检索验证。

## 研究 importer

```text
cargo run -p nous-kernel --example cognitive-import -- docs/research/corpus/cognitive data/research/runs/cognitive-recall
```

使用已安装的 PostgreSQL 18.6 runtime，在独立 ignored run root 中保存数据库、对象和 operation/revision/session 映射。正常结束会停止该 harness 的 PostgreSQL，保留数据供后续 reopen；不操作现有 Core 实例。Corpus manifest/scenario digest 固定数据，重跑复用已导入记录并回读 Memory 时间。

Harness 通过 `NousRuntime::open_with_clock` 注入串行研究时钟。Memory mutation 第一次 semantic `now()` 返回 formed_at，之后返回 recorded_at；读 current head 在 arm 之前进行。它是显式研究 fixture，不能当作真实用户运行期时钟。Occurrence 的 occurred/observed 与 Memory valid interval 通过正常输入合同提交，formed/recorded 通过原 owner 时钟写入并逐条检查。Importer 目前采用固定 scenario 文件名和已授权的 corpus schema。

`supersedes` 事件通过实际 `revise_memory(Correct)` 更新同一 Memory，保留先前 exact revision；recalls_precursor 映射 `assoc.sequence`，recurrence 映射 `assoc.related`，支持为原 observation evidence。这些映射进入报告；不要声称 relation ontology 完全等同。其他 render kind 当前作为文本 observation 输入，未据 journal/reminder 标签创建相应 cognition object。

本次实际导入位于 ignored `data/research/runs/cognitive-retrieval-2026-10-04/cc0-runtime`，已提交真实 provider embedding 材料（361 个唯一文本、831 个 reference，余下 needs=0）；后续前缀运行已启用 Serving，完整 Spec tracks 仍待执行。初始 manifest 的 `status` 描述 authored release 时点，实际 import/run 状态以对应 run root 为准。

[返回语料入口](../README.md)

材料生成入口见 [Cognitive embedding material](../../../../scripts/README.md#cognitive-embedding-material)。使用 production `resolvedEmbedding` 与 `ModelInvocations.embeddingBatch`，经过同一个 research gateway。首次批量调用在 100 个文本后出现 HTTP 429；缓存保留，改为 6000ms 批次间隔后完成剩余文本，没有重复生成已缓存内容。这个限流恢复记录属于 provider operational 结果，不是 retrieval quality 结论。另已生成 247 个唯一 query 文本向量，合并缓存共 608 项。

## 前缀查询 runner

```text
cargo run -p nous-kernel --example cognitive-import -- docs/research/corpus/cognitive data/research/<fresh-prefix-run> data/research/<material-run>/embedding-config.json data/research/<material-run>/embedding-vectors.json benchmark
corepack pnpm exec tsx scripts/research/cognitive-score.ts --input data/research/<fresh-prefix-run>/benchmark.jsonl --output data/research/<fresh-prefix-run>/metrics.json
```

`benchmark` 模式按 Subject/as-of 排序，只有 recorded_at 不晚于 query as-of 的事件进入 Authority。每条 query 的前缀在 baseline/native/DTSC/V3 间保持不变。这个 Kernel track 没有 model rerank，查询向量来自实际 provider 预生成缓存；相同文本只生成一次。Source 与 query 材料缓存必须完整，缺项会报错。

五轴 case 从查询文本明确写出的 RFC3339 时刻构造正常 QueryConstraints；oracle 不用于候选选择。其他查询使用原 text cue，historical retention 也未通过正例 ID 构造 Exact target。当前 spontaneous case 是自然语言当前 situation 描述，`situation` 结构体仍为默认值，需要与实际 Runtime situation context 对照分开解释。

每条 JSONL 保存 profile、Authority 水位、generation IDs、配置摘要、结果/diagnostics、oracle、latency 与 precomputed provider usage 标记。初次运行暴露 native/VCP 水位不一致导致 profile 切换失效，缺陷前结果单独保留；修复后的重跑使用新的 prefix root。运行前缀必须是新实例；目前不自动恢复已有 benchmark.jsonl，数据库和部分输出会保留，恢复流程仍需实现。

Scorer 展开同 Subject 的 grade=0 默认 oracle，按 category/profile 输出 Recall、MRR、AP、正 nDCG、source-set recall、harmful exposure 和 latency 分位数。退化行保留，未映射项单独计数。多轴 source 时间、旧修订 retrieval policy、relationship ontology 和模板结构是解释结果时的具体合同边界；不自动把全部低分视作产品 defect。

## 首轮按类别观察

实际结果位于 ignored `data/research/runs/cognitive-retrieval-2026-10-04/cc0-prefix-corrected/{benchmark.jsonl,metrics.json}`：249 queries、996 rows、80 category/profile groups。每个 query 的四个 profile 水位相同；无执行错误和 topology unavailable。289 rows 出现 `validation_budget_exhausted`（正常预算验证 40 个候选），没有过滤这些结果。

| Category | baseline/native Recall@10 | DTSC Recall@10 | V3 Recall@10 | baseline/native nDCG@10 | DTSC nDCG@10 | V3 nDCG@10 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| knowledge_update | 1.000 | 1.000 | 0.967 | 0.972 | 0.973 | 0.910 |
| association_1_hop | 1.000 | 1.000 | 1.000 | 0.445 | 0.469 | 0.450 |
| association_2_hop | 0.133 | 0.233 | 0.133 | 0.041 | 0.073 | 0.040 |
| association_3_hop | 1.000 | 0.567 | 1.000 | 0.433 | 0.189 | 0.391 |
| narrative_causal_chain | 0.656 | 0.533 | 0.678 | 0.636 | 0.620 | 0.623 |
| private_zero_shot_concept | 0.967 | 0.989 | 0.978 | 0.885 | 0.925 | 0.905 |
| spontaneous_helpful_recall（text 输入） | 0.520 | 0.500 | 0.540 | 0.392 | 0.375 | 0.404 |
| historical_state_retention | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| recurrence | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |

这轮 native 249 行 seed/edge count 都为 0：冻结 native 的 source seeds 来自显式实体/Tag/关系、exact binding 或 Runtime refs，当前输入只有 text cue，未触发 cue sensing。因此表中的 baseline/native 相同只描述这个输入合同；后续需比较 grounded structured cue 与 Runtime 情境输入。历史查询目前使用普通文本，只允许 current revision；已有 exact historical 能力不能代替从自然语言检索历史状态。

三条 future-exclusion case 无 future exposure，四个 profile Recall@10=1；这是前缀导入隔离条件下的结果。五轴 occurred/observed/formed/recorded case 正例均 Recall@10/nDCG@10=1，valid case Recall@10=1，但 V3 nDCG@10=0.544（其他三个 0.754）。小类各只有三条，不能外推到一般时间推理能力。Diffusion/interference 类四个 profile 的平均 harmful@10 均为 1，低分与 intrusion 都保留在 category 指标中。

当前 JSONL 的 `config_digest` 是 profile/assets/query/readout 四键的配置摘要；后续 runner 已改为完整 effective digest，并另存 `cognitive_config_subset_digest`。该轮 latency 混合冷/热 Serving preparation，未拆分 embedding/model rerank/final validation；cached query vectors 不算每条实时 embedding 调用。尚无 rerank、外部 suite、消融或 hybrid 取舍结论。

## Query input revision 2

`queries.json` revision 2 在每个 query 的 `input_context` 声明独立输入：Tag label 必须逐字出现在 query text 且已进入 as-of prefix；spontaneous case 的 `current_event_ids` 指向文字中“接手下一轮值班”所引用的实际提醒事件。这些字段从 authored story/input 取得，不读取 grade、oracle 或 required_paths。Runner 将 Tag cue 与 Runtime current refs 通过普通 bind/query 合同提交，逐条记录输入。30 条 spontaneous case 使用非空 Runtime context。

第二轮在新的 `cc0-context-v2` prefix root 对四个 profile 全量重跑；相同 query/source 文本复用既有真实 provider cache，不重新调用 embedding。首轮 revision 1 的 text-only raw 输出保留，和 revision 2 分开分析，不能把输入变化当作纯算法消融。新输入已实际激活 native 种子/扩散；完整结果仍需待运行结束评分。Recurrence 与 weak-cue 未出现可绑定的 Tag surface，因此仍保留 text-only 输入。
