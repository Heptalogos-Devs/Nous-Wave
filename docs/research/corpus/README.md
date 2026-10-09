# 研究语料

- [VCP 与 Native 冻结数值结果](vcp/README.md)：原输入、expected、容差与来源身份，算法检查直接读取这份研究数据。

tracked tree 保存 source manifest、少量 oracle metadata 和手工功能场景。下载正文、拆分文本、vectors、大型 source-output matrices 与 run outputs 保存于 ignored `data/research/`。

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
