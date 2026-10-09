# 研究语料

- [VCP 与 Native 冻结数值结果](vcp/README.md)：原输入、expected、容差与来源身份，算法检查直接读取这份研究数据。
- [全仓重整 consumer 续接](deep-rebase/consumer-continuation-2026-10-09.json)与[执行重整后续接](deep-rebase/execution-continuation-2026-10-09.json)：真实 Codex/MCP 的来源、模型、trace、检索和使用结果原文，包括暴露的失败或身份遗漏；当前解释见[重整观察](../deep-rebase-2026-10-09.md)。
- [Query 生命周期重整后续接](deep-rebase/query-continuation-2026-10-09.json)：producer 重整后的首次 dense degradation、显式 embedding preparation 和恢复查询原文。
- [当前数据库与 Windows Portable 续接](deep-rebase/portable-continuation-2026-10-09.json)：四份初始 schema 新库的全表恢复摘要、真实 Codex/MCP／Doubao formation、trace、embedding、Query 和 use 原文；旧认知仍保留。

- [Linux 干净实例](deep-rebase/linux-continuation-2026-10-09.json)、[重启续接](deep-rebase/linux-restart-continuation-2026-10-09.json)、[有界维护](deep-rebase/linux-maintenance-continuation-2026-10-09.json)与[显式材料恢复](deep-rebase/linux-maintenance-recovery-2026-10-09.json)：保存实际 Doubao Lite formation、原文、Unicode/exact/history、真实预算耗尽与后续 complete Query。

tracked tree 保存 source manifest、少量 oracle metadata 和手工功能场景。持久模型正文、query/result 与数值矩阵保存在 ignored `data/research/results/` 的紧凑成果中；完成提取后已清理旧数据库、下载正文、媒体、vectors 与原始运行目录。

- [手工功能语料](functional/README.md)：Tide deployment rule、Aya journal preference、snapshot/build-cache analogy 三个场景与 15 个 prepared intents。
- [六项 text-only 选择](text-compatibility-selection.json)：2 LongMemEval、2 LoCoMo、2 Python official docs，原始 query 不注入 Entity/Tag/WorkContext oracle。
- [外部来源元数据](external-memory-sources.json)：外部数据的来源与许可。
- [Hard Text 来源元数据](hard-text-sources.json)：URL、license/rights、source checksum 与提取方式；完整 blocks 留在 ignored cache。
- [VCP source manifest](cognitive-retrieval-sources.json)：冻结 reference source identity。
- [媒体来源](media.json)、[音频 negative controls](audio-negative-controls.json)。
- [个人文本来源](personal.json)、[个人查询](personal-queries.json)。
- Core Cognition：[Simon](core-cognition/simon.json)、[CPython](core-cognition/cpython.json)、[Rust sealed 候选](core-cognition/rust.json)、[Kafka sealed 候选](core-cognition/kafka.json)、[独立 revision oracle](core-cognition/revision-slice.json)、[2026-10-07 稳定结果](core-cognition/results-2026-10-07.json)。[2026-10-08 v2 稳定结果](core-cognition/results-2026-10-08.json)保存本轮实际 source identities、费用与已执行 sealed。[方法](../core-cognition-semantic.md)区分结构完整性与语义质量。
- [原始文本 manifest](manifest.json)、[查询](queries.json)、[已有观测元数据](observed-results.json)。

先验证 formation 与 Tag/Association maintenance，再解释四 profile 的小型 readout。六项文本兼容性仅作 smoke，不宣称全量 benchmark accuracy。全量 LongMemEval/LoCoMo、RAGFlow 和付费 rerank/provider 比较本轮不执行。

[返回研究入口](../README.md)
