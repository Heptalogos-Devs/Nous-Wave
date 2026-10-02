# Research

真实语料与 oracle 在 [`research/corpus/`](../../research/corpus/README.md)，runner 为 `scripts/research/retrieval.ts`、`media.ts`。二者只使用 official Client。`research:gateway` 在 wire boundary 保存 run-owned request ledger，默认硬上限 10000，含失败和重试；普通 runtime 不承担研究计数。

同一 ledger 旁的 `.telemetry.jsonl` 保存 request ordinal、endpoint、HTTP status、latency 与可解析的 numeric token usage；cost 保持 unknown。provider 正文、headers 和 credentials 不进入记录。

[实验方法](retrieval-media-methods.md) 描述固定 Authority、对比策略和指标；[已观测结果](observations.md) 保存当前可复用结论。原始响应、媒体、运行日志留 ignored data。
