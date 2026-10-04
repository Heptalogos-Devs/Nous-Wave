# 研究语料

- [manifest.json](manifest.json)：6 个文本来源、105 个语义单元及来源 locator、digest、主题和使用说明。
- [queries.json](queries.json)：40 条 grounded query 与 acceptable/expected source units。
- [media.json](media.json)：14 个图像、音频和视频样本（含保留对照）。
- [personal.json](personal.json)：Simon Willison 的 6 篇相关博客、来源/出版时间、HTML 与正文 digest、抽取规则、作者/项目实体与独立事实 oracle。
- [personal-queries.json](personal-queries.json)：单篇事实、工作实践、跨文档项目、年代关系和工具工作流查询。
- [audio-negative-controls.json](audio-negative-controls.json)：两段 NASA 音频的人工听辨事实。
- [cognitive-retrieval-sources.json](cognitive-retrieval-sources.json)：PR #14 后 Cognitive Retrieval 实验的 Nous/Vault/VCP/New API 基线与 VCP blob 身份。
- [external-memory-sources.json](external-memory-sources.json)：LongMemEval-S / LoCoMo 的官方数据版本、SHA-256、许可、分类与 annotation 适配状态。第三方原文和 prepared fixture 均保存在 ignored `data/research/external/`。
- [observed-results.json](observed-results.json)：2026-10-01 初始文本检索测量。

2026-10-02 文本检索及 2026-10-04 真实模型验证见 [Research](../observations.md)。原始来源内容、媒体、逐条输出和运行日志位于 ignored 的 `data/research/`。

[返回 Research 入口](../README.md)

[原创 Cognitive Recall Corpus](cognitive/README.md) 保存三条事件图、249 个查询 oracle 和有依据的来源路径；已由独立 research harness 导入，已完成首轮四个 no-rerank profile 的前缀比较，完整 Spec tracks 仍待完成。

## 外部长期记忆 retrieval-only 数据

统一入口的 `prepare --suite` 命令见 [scripts](../../../scripts/README.md#external-memory-suites)。LongMemEval-S 使用官方 cleaned 文件的 `has_answer` turn 与 `answer_session_ids` 两层 oracle，500 个独立 Subject、23,867 sessions、246,750 turns、500 queries；7 个类别全部保留。Abstention 按 `_abs` 单列，没有把官方 answer 字段写进输入。

LoCoMo 按原 session 编号顺序保留 10 个独立 Subject、5,882 dialogue turns、1,986 queries。每条 turn 保存 speaker、文本、原始 dialogue locator，以及存在时的官方 image caption 文本；没有生成图片描述或 QA answer。Oracle 依据官方 dialogue evidence，category 按官方 1–5 映射。适配器规范化分隔符、`D:11:26` 的多余冒号和数字 padding；仍无法对应真实 dialogue 的三条 query 单列在 `adapter-audit.json`，没有补造 evidence。

Session 时间原数据没有 timezone，research fixture 显式按 UTC 解释；同一 session 内所有 turns 保留同一个 source 时间。Controlled history 的可用时刻设为 max(question date, last session)+1 秒。LongMemEval 76 条 question date 早于末次 session，因此输出同时保留原 question date，不能将这个完整-history track 当作前缀 future-exclusion 测试。Prepared oracle 不参与形成输入，tags/association graph 没有从 QA evidence 制造。

下载/适配完成仅证明输入已冻结。LoCoMo 已通过普通 owners 导入 5,882 turns / 272 Sessions，reopen 回读复核完成；LongMemEval 全量 owner 导入正在执行。实际 embeddings、各 profile 运行、session/turn recall、分层 end-to-end formation 与算法取舍都需要对应实测。此处结果不等同于原论文 answer-generation accuracy。

LongMemEval source 的 session occurrence 数为 23,867；其中 13 个 session ID 在同一个 item 重复出现，正文完全相同而日期不同，均不属于 answer session。Adapter 保留全部 246,750 turns 和各自日期，Cognitive Session 仍使用官方逻辑 ID，因此实际导入有 23,854 个 Session、500 个 Subject。`adapter-audit.json` 单列这些重复项，不将 occurrence 数与逻辑 Session 数混用。
