# Research

[Research Corpus](corpus/README.md) 记录当前来源、语料、oracle 与结果文件。[实验方法](retrieval-media-methods.md) 描述查询 track、provenance relevance 和指标；[观测结果](observations.md) 保存实际检索与媒体结论。

[VCP source conformance](vcp-conformance.md) 记录 frozen production 数学合同、Nous 差异与 reference/adapter 的实际实现状态。

研究 runner 位于 `scripts/research/`，通过 official Client 操作系统。原始来源内容、媒体、逐条输出和运行日志保存在 ignored 的 `data/research/`。

纵向认知模型研究使用 [`research:longitudinal`](../../scripts/README.md#longitudinal-model-research)，针对精确来源快照生成 Episode partition、Journal 或 consolidation proposal，供真实 trace 与来源事实的人工评估。当前尚无经过人工标注的纵向质量 corpus 或质量测量结果。
