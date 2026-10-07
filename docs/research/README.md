# Research

[Research Corpus](corpus/README.md) 记录当前来源、语料、oracle 与结果文件。[实验方法](retrieval-media-methods.md) 描述查询 track、provenance relevance 和指标；[观测结果](observations.md) 保存实际检索与媒体结论。

[Core Cognition Semantic Qualification](core-cognition-semantic.md) 定义真实来源、formation-first 校准与 sealed qualification 的执行合同。
[2026-10-07 实际结果](core-cognition-2026-10-07.md)记录 calibration grounding/query closure blockers、已执行 slices 与未执行 sealed packs。

[VCP source conformance](vcp-conformance.md) 记录 frozen production 数学合同、Nous 差异与 reference/adapter 的实际实现状态。

研究 runner 位于 `scripts/research/`，通过 official Client 操作系统。原始来源内容、媒体、逐条输出和运行日志保存在 ignored 的 `data/research/`。

纵向认知模型研究使用 [`research:longitudinal`](../../scripts/README.md#longitudinal-model-research)，针对精确来源快照生成 Episode partition、Journal 或 consolidation proposal，供真实 trace 与来源事实的人工评估。[Functional cognition corpus](corpus/functional/README.md) 提供本轮人工编写的三场景功能集；自然形成、完整查询和质量验收仍在执行，尚无完整质量测量结果。

[独立文本兼容性选择](corpus/text-compatibility-selection.json)保留六个原始问题与 source locator/checksum；第三方原文及历史结果在 ignored data。此次重构用 public Client 的小型功能路径复验，不运行旧全量 importer、第二套 research Runtime 或新的 provider benchmark。

[返回文档目录](../INDEX.md)
