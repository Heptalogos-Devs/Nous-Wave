# 研究语料

- [manifest.json](manifest.json)：6 个文本来源、105 个语义单元及来源 locator、digest、主题和使用说明。
- [queries.json](queries.json)：40 条 grounded query 与 acceptable/expected source units。
- [media.json](media.json)：14 个图像、音频和视频样本（含保留对照）。
- [personal.json](personal.json)：Simon Willison 的 6 篇相关博客、来源/出版时间、HTML 与正文 digest、抽取规则、作者/项目实体与独立事实 oracle。
- [personal-queries.json](personal-queries.json)：单篇事实、工作实践、跨文档项目、年代关系和工具工作流查询。
- [audio-negative-controls.json](audio-negative-controls.json)：两段 NASA 音频的人工听辨事实。
- [cognitive-retrieval-sources.json](cognitive-retrieval-sources.json)：PR #14 后 Cognitive Retrieval 实验的 Nous/Vault/VCP/New API 基线与 VCP blob 身份。
- [observed-results.json](observed-results.json)：2026-10-01 初始文本检索测量。

2026-10-02 文本检索及 2026-10-04 真实模型验证见 [Research](../observations.md)。原始来源内容、媒体、逐条输出和运行日志位于 ignored 的 `data/research/`。

[返回 Research 入口](../README.md)
