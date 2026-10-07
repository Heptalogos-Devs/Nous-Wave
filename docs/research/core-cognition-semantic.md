# Core Cognition Semantic Qualification

[返回 Research](README.md)

## 研究合同

用真实来源审查 formation、Memory identity、Tag/Association/Schema/Accretion、历史 Authority 和检索增量。实验通过 official Client 连接已有 Core，复用 research gateway。第三方原文和逐次 trace 保留在 ignored `data/research/`；本轮所有检索禁止 rerank。

每包建立 fresh experimental 与 raw-control Subject，按来源发展顺序摄入，记录 Subject CognitiveClock checkpoint。先执行有界 formation/Host-granted maintenance，再审查精确来源与实际 cognition；达到 `ACCEPTABLE_FOR_RETRIEVAL` 后冻结 Authority 才比较检索。形成 unsupported Authority 或历史泄漏时停止受影响轨道，保存证据，交由独立 production 修复分支。

## 执行顺序

1. 在 `scripts/research/` 建立 manifest/hash 校验、稳定 operation identity、resume、result aggregation 与 sealed lock；用本地 deterministic fixtures 检查研究机制。
2. 复用 Simon 的 `personal.json` / `personal-queries.json`，取得 CPython 官方版本快照；formation-first calibration 后执行 plain/raw-control、time/history、选定 context/enrichment/profile 比较。
3. 主检索完成后执行 Query → ReportUse → Host Grant Maintenance 与真实 source-backed revision/Tag continuity slice。
4. 冻结 code、runner、source/query/oracle、model/config、prompt/schema 与 embedding identities，执行 Rust Async Fn in Traits 和 Kafka KRaft sealed packs。
5. 仅在 reserve 能翻转当前决策时使用 Kubernetes Sidecar。按实际 `PASS / FAIL / NOT_RUN / BLOCKED` 写入稳定结果，运行 `just check` 后完成 PR 交付。

Calibration 可以修正研究方法和 runner，不能在本分支依据结果调整 production prompt、cognition semantics、ranking 或 topology。既有产品合同以 Active Specs 和 Architecture-Vault 为准。本研究不预先要求 Schema 数量或特定 Tag，也不把 direct/promoted seed 命中计为多跳收益。

## 结果状态

尚未开始 live qualification。方法、manifest/oracle 和稳定观测将随执行补充；本页当前不宣称任何 pack 通过。
