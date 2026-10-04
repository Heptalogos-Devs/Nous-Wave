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

300 events、249 queries 的 authored graph 已保存，文件摘要、引用、时间顺序和 1/2/3-hop 最短路径已检查。独立 research PostgreSQL 已通过下面的 harness 导入 300 events、60 Sessions 和 207 条有来源支持的 association，并回读确认 formed/recorded 时间。**尚未运行完整 benchmark**。后续还需：

- 补全 importer 的外部 evidence/graph coverage 审计，并使用保存的 deterministic event → Authority revision 映射进行评测。
- 将 research clock 与查询 as-of 调度接线，验证五轴过滤和历史状态；当前 importer 通过正常 owner 写入时间，没有直接修改数据库时间字段。
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

本次实际导入位于 ignored `data/research/runs/cognitive-retrieval-2026-10-04/cc0-runtime`，已提交真实 provider embedding 材料（361 个唯一文本、831 个 reference，余下 needs=0）；尚未启用 Serving 或执行完整 query runner。初始 manifest 的 `status` 描述 authored release 时点，实际 import/run 状态以对应 run root 为准。

[返回语料入口](../README.md)

材料生成入口见 [Cognitive embedding material](../../../../scripts/README.md#cognitive-embedding-material)。使用 production `resolvedEmbedding` 与 `ModelInvocations.embeddingBatch`，经过同一个 research gateway。首次批量调用在 100 个文本后出现 HTTP 429；缓存保留，改为 6000ms 批次间隔后完成剩余文本，没有重复生成已缓存内容。这个限流恢复记录属于 provider operational 结果，不是 retrieval quality 结论。Query embedding 尚未批量生成。
