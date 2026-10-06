# 研究语料

tracked tree 保存 source manifest、少量 oracle metadata 和手工功能场景。下载正文、拆分文本、vectors、大型 source-output matrices 与 run outputs 保存于 ignored `data/research/`。

- [手工功能语料](functional/README.md)：Tide deployment rule、Aya journal preference、snapshot/build-cache analogy 三个场景与 15 个 prepared intents。
- [六项 text-only 选择](text-compatibility-selection.json)：2 LongMemEval、2 LoCoMo、2 Python official docs，原始 query 不注入 Entity/Tag/WorkContext oracle。
- [外部来源元数据](external-memory-sources.json)：外部数据的来源与许可。
- [Hard Text 来源元数据](hard-text-sources.json)：URL、license/rights、source checksum 与提取方式；完整 blocks 留在 ignored cache。
- [VCP source manifest](cognitive-retrieval-sources.json)：冻结 reference source identity。
- [媒体来源](media.json)、[音频 negative controls](audio-negative-controls.json)。
- [个人文本来源](personal.json)、[个人查询](personal-queries.json)。
- [原始文本 manifest](manifest.json)、[查询](queries.json)、[已有观测元数据](observed-results.json)。

先验证 formation 与 Tag/Association maintenance，再解释四 profile 的小型 readout。六项文本兼容性仅作 smoke，不宣称全量 benchmark accuracy。全量 LongMemEval/LoCoMo、RAGFlow 和付费 rerank/provider 比较本轮不执行。

[返回研究入口](../README.md)
