# Core Cognition Semantic Qualification

[返回 Research](README.md)

## 研究合同

用真实来源审查 formation、Memory identity、Tag/Association/Schema/Accretion、历史 Authority 和检索增量。实验通过 official Client 连接已有普通 Core，复用 research gateway，不启动数据库、Kernel 或另一套 Runtime。第三方原文和逐次 trace 保留在 ignored `data/research/`；所有检索禁止 rerank。

每包建立 fresh experimental 与 raw-control Subjects，按来源发展顺序摄入，记录 Subject CognitiveClock checkpoint。先执行有界 formation/Host-granted maintenance，再对照 exact sources 人工审查 cognition；达到 `ACCEPTABLE_FOR_RETRIEVAL` 后 freeze Authority 才比较检索。形成 unsupported Authority 或历史泄漏时停止受影响轨道，保存证据，交由独立 production 修复分支。Calibration 可以修研究方法和 runner，不能依据结果调整 production prompt、cognition semantics、ranking 或 topology。

## 来源、查询与 runner

[official Client runner](../../scripts/research/core-cognition-semantic.ts) 的 CLI 参数与命令由[脚本说明](../../scripts/README.md#core-cognition-semantic-qualification)维护。来源与查询的 owner 是 [Simon](corpus/core-cognition/simon.json)、[CPython](corpus/core-cognition/cpython.json)、[Rust](corpus/core-cognition/rust.json) 和 [Kafka](corpus/core-cognition/kafka.json) manifests；Simon 引用现有 personal source/query catalogs。

每个 source 保存 URL、version identity、publication time、rights、raw/text SHA256 与 ignored cache path；query 保存 expected/forbidden sources、分类、时间 cut 与 oracle notes。Oracle 只用于评价和显式实验 arm，不进入自动 formation 或 plain query。原始 bytes 与提取文本分别核验 hash。超过单次输入限额的正文按完整、无重叠、Unicode-safe 的 12000-byte regions 处理，不按 oracle 选片段；各 region 使用同一个 canonical web root，不把 chunk 当独立佐证。HTML 去模板与 release-version 展开有 extraction identity。

## 执行与门禁

1. `acquire-check` 核验来源；`ingest` 只建立 observations/raw-control；`formation` 执行真实 model formation 与有界 Host grants。可用 `--through-checkpoint` 在一个阶段停止，恢复后继续。
2. `review-export` 导出 exact source receipts、Memories、Tags、Association neighborhoods、maintenance receipts 与 checkpoints。人工逐条对照来源，写入 ignored `review.json`：decision、snapshot digest、notes、blockers，以及可选的实际 Tag binding。没有模型 judge。
3. `retrieval` 仅接受 `ACCEPTABLE_FOR_RETRIEVAL` 且无 blocker 的 review，重新核验 snapshot，再 freeze 两个 Subject 的 Authority。公开 embedding cache 准备可能因 Serving invalidation 推进 sequence：先证明 cognition snapshot 不变，再记录 post-cache retrieval watermark；查询前后核验该 watermark。
4. 保留原 query 的 plain/raw-control；按 manifest 执行 as-of/history、时间约束、Entity、WorkContext、direct Tag、bounded exploration、existing/model enrichment 与四 selected profiles。rerank 一律 `forbidden`。未形成唯一 Tag 时记 `NOT_RUN`；prepare closure guard 拒绝时记 `BLOCKED`，不改写原问题或偷偷启用 text-only compatibility。`COMPLETED_WITH_BLOCKED_ARMS` 只表示矩阵记录完成。
5. 主矩阵后单独执行 `feedback`，消费真实返回的 exact revision/query identity；事件接受/重复回执、Grant 前 snapshot、Grant 后 proposal 分开保存。use-review interval=1 的研究 override 单列，不并入 reference-default 分数。自然到期的 Episode need 必须按 focus/queryFeedback 判断归因，不能把 Grant 内所有变化归因于 UseEvent。
6. `revision` 在独立 fresh Subject 执行[真实来源 slice](corpus/core-cognition/revision-slice.json)：同一持续事实的 rephrase，使用 explicit source-backed research Tag；它验证 revision、as-of/history 与 maintenance continuity，不代替自动 formation 质量。
7. Calibration 通过后才允许 live sealed Rust/Kafka。`--sealed-lock` 必须声明 calibration `PASS`、Simon/CPython result digests、完整共同 identities，以及每 pack 的 manifest/oracle/source identities。runner 严格比较全部 keys；未通过 calibration 或缺少 lock 即拒绝。门禁前允许 `acquire-check` 验证 sealed 候选 sources。Kubernetes reserve 只在可能改变现有决策时启用。

`all` 只编排 formation → review export → retrieval；第一次在缺少已接受 review 时停止，人工审查后恢复。Feedback/revision 独立调用，避免检索冻结与后续 Authority 写入混同。既有产品合同以 Active Specs 与 Architecture-Vault 为准；不预设 Schema 数量/Tag 名称，不把直接 seed 命中计成多跳收益。

## Resume 与身份冻结

`state.json` 在远端 effect 前保存 operation intent/digest。public stable operation/event identity 的 mutation 可恢复，完成 receipt 不再调用 provider；未知结果的 query、embedding preparation 或 grant 不自动 replay，核查 Core/ledger 后在新 run 继续。runner lock 拒绝同一 output 的并行执行，可清理死 PID lock。失败尝试与当次身份保留，不用后来的成功覆盖历史。

Calibration resume 固定 manifest/oracle/source/model/schema/config；研究代码修正逐次记入 execution identities。Sealed resume 则锁定全部 keys：code HEAD、runner digest、Vault/product HEAD、model、prompt/schema、embedding space、active config、Kernel binary。变更须建立新 qualification version，不覆盖 sealed 结果。`identities.json` 必须提供这些必要身份，来自 production registry 导出与实际配置，不含 credential。

## 结果与证据合同

`run.json`/`state.json` 保存 Subjects、checkpoints、phases、每次 execution 与 identities；`formation-summary.json` 保存 exact receipts/snapshot；`query-results.json` 保存每 arm 的 query/prepared identity、authority cut、normalized refs、source matches、指标、degradation、Activation/route diagnostics；独立 slices 写入 `feedback-results.json`、`revision-results.json`；`cost.json` 保存共享 gateway 累计调用与 usage。原文与完整 trace 只留在 ignored data，tracked 报告只提升自写判断、短 locator、refs、hash 与指标。

source Recall@1/5/10、MRR 与 all-support 按不同 source roots 计算；absence oracle 返回 `null` 分数。这些衡量来源召回，不能自动证明 claim 语义正确或回答拒绝。多跳价值必须看到实际 activated route、Association support 与超出 direct attachments 的结果，并保留 truncation/degradation。

public API 无 exhaustive Schema/Accretion inventory：Schema 只从已检查 neighborhoods 发现，Accretion 经真实 maintenance input trace 审查。零 observed Schema 不等于已验证 formation/usefulness。时间 slice 的 returned membership 也不扩大为所有历史 descriptor/vector/topology 的全局证明。

## 当前结果

[2026-10-07 集中报告](core-cognition-2026-10-07.md)与[稳定结果 metadata](corpus/core-cognition/results-2026-10-07.json)记录实际 calibration、独立 slices 与 blocked sealed packs。CPython grounding 失败、Simon query closure guard 阻塞完整比较；本轮没有 sealed PASS，不晋升 enrichment/profile。
