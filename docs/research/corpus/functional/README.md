# 手工认知功能语料

[返回语料入口](../README.md)

[scenarios.json](scenarios.json)保留三个手工场景：[Tide deployment rule](scenarios.json)、Aya journal preference、snapshot/build-cache lease analogy。[queries.json](queries.json)包含 15 个 prepared intents；[manifest](manifest.json)维护语料身份。

先通过真实产品 Client/CLI 提交 Subject、Session、Observation 和 WorkContext，显式授予有界 maintenance，再检查 Tag create/reuse、alias merge、distinct concepts、one-off joke、Association support、recurrence 和 ambiguity。形成失败时先修产品能力，再做算法 readout。

PrepareQuery 固定 semantic representation 与 exact context；baseline/native/DTSC/RiverMemo 使用相同 semantic input。自动验证使用 fake/deterministic provider，不调用真实 embedding、rerank 或模型；原始 text-only compatibility 独立报告。小型 runner 只连接已经运行的 public Core，结果保存到 ignored `data/research/`。

2026-10-06 的 deterministic public smoke 已通过三场景形成质量检查，运行 15 intents × 4 profiles，并验证同一 intent 的 prepared semantic text 不因 profile 改变。六个 selected raw-text cases 在四 profiles 下 的 first relevant source 均为 rank 1。Fake embedding 为线路验证向量；query metrics 原样保留，部分查询仍有 temporal unknown、truncation 或排名不足，不把线路执行解释为语义质量通过，也不宣布 profile winner。当前 source/case 级结果保存在 ignored `data/research/cognitive-functional/`。
