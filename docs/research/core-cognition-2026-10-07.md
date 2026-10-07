# Core Cognition Semantic Qualification — 2026-10-07

[方法与运行合同](core-cognition-semantic.md) · [稳定结果 metadata](corpus/core-cognition/results-2026-10-07.json) · [返回 Research](README.md)

## 当前决策

本轮实际运行了真实 source calibration 与两个独立生命周期 slices。CPython unsupported 日期 claim 已进入 Memory 并传播到 Tag；Simon 合法原始 bare-text 问题被 closure guard 拒绝。停止受影响的算法比较，sealed Rust/Kafka 未执行。本分支没有调整 production prompts、cognition semantics、ranking 或 topology。

保留 concept enrichment 默认 `off`，不晋升 selected profile，不进入 external qualification。Native 在一个明确 Tag seed 上为 `CONDITIONAL_VALUE`：存在 source-backed 两跳新增结果，但探索截断、weak-cue/sealed 矩阵未完成，不能宣布 winner。Kubernetes Sidecar 无法消除现有 blockers，不启用。

## 真实基线与实验边界

开始时重新 fetch/核验远端：Nous-Wave `master=e14eae96596417c225749a8b96db6fb9ea121d00`；Architecture-Vault `main=2de60296bc80d790e9dd508bc6b3abd19c7d3236`。附件 SHA 只是经核验的起点。研究分支 `research/core-cognition-semantic-qualification`，Draft [PR #18](https://github.com/Heptalogos-Devs/Nous-Wave/pull/18)。没有修改 Vault 或 production files。

普通 Core 使用 fresh `data/instances/core-cognition-qualification`，official Client 连接其 RunRoot。原 `cognitive-retrieval` 实例有 migration checksum 不兼容，保留数据，没有改旧 migration 或 reset。Core 使用当前 source 与编译的 Kernel。全局自动 maintenance 关闭，实验 Subjects 只进行显式 Host grants；formation 时 dense/topology 关闭，接受 review 后重启普通 Core 开启 serving。该 calibration 配置转换逐次记录，model/prompt 未调优。

完整 raw/text、source receipts、snapshot、traces、ledger、review 和逐次身份在 ignored `data/research/runs/core-cognition-2026-10-07/`，主要 roots 为 `simon-v1/`、`cpython-v1/`；source cache 位于 manifests 指定 paths。稳定 metadata 保存证据 SHA256，摘要不替代完整 trace。

## 状态

| 轨道 | 实际状态 | 证据与解释 |
| --- | --- | --- |
| Runner contracts / resume / sealed guard | PASS | 10 deterministic tests：source hash、付费操作 receipt、未知非幂等结果停止、identity drift 与 calibration 门禁 |
| Simon 自动 formation review | PASS | 7 Memories / 4 Tags / 6 Associations；`ACCEPTABLE_FOR_RETRIEVAL`，不等于概念最优 |
| CPython 自动 formation review | FAIL | 22 Memories / 12 Tags / 48 Associations；`NEEDS_MODEL_POLICY_RESEARCH`，错误日期传播到 Tag |
| Simon 四个已执行原始 direct/chronology 对照 | PASS | formed/raw 每题 source Recall@5=1，不是全矩阵 non-regression 结论 |
| Simon origin、weak-cue/context/enrichment/profile、source-time arms | BLOCKED | 10 个 arm 在 public prepare 被拒绝，没有 query |
| Simon early-cut / history membership | PASS | 没有 future source matches；限该 slice |
| Simon direct Tag / native route witness | PASS | 3 direct attachments；native 新增 Link Blog，经实际相关 Tag 两跳，lane truncated |
| QueryActivation existing/model 质量 | BLOCKED | 原问题 prepare 被拒绝；query-concept model calls=0 |
| Simon feedback lifecycle | PASS | exact query/revision、事件去重、Grant 前概念不变、Host Grant 后有来源支持；不声明所有 proposal 高质量 |
| CPython 主 retrieval / feedback | BLOCKED | formation gate 未通过，未执行 |
| 独立 source-backed revision/history | PASS | same-fact rephrase；direct recall 1→0→1（Grant 后），旧 cut 排除新版 |
| Schema 自动形成及后续 usefulness | NOT_RUN | bounded main formation 未自然形成 Schema；public inventory 有限，无质量证据 |
| Accretion 可见信号 | PASS | source units 不增加独立 roots、use/refutation 分离可见；不是 recurrence/priority/global quality qualification |
| Rust Async Fn in Traits sealed | BLOCKED / NOT_RUN | source/manifest/oracle 已核验；无 live formation/query |
| Kafka KRaft sealed | BLOCKED / NOT_RUN | 同上 |
| Kubernetes Sidecar reserve | NOT_RUN | 不会改变当前停止/不晋升决策 |

## Formation 与来源时间

Simon 全部 7 accepted Memories 对照 actual source units 审查，exact evidence 属于正确 Subject/receipts。四个 Tag 区分 2022 retrospective、2024 link practice、2026 beats、Datasette announcement；2022 center 较宽，长期写作实践跨文档复用仍弱。Association 支持合法，真实 `assoc.related` 连接 2022 center 与 2024 link practice。初始 review 的“未发现多跳 Association”表述过强，ignored `review-addendum.json` 修正判断，原 frozen snapshot/review 保留。

`what-to-blog` 是 2022 发布但包含作者 2024 update 的冻结正文。early checkpoint 表示实验 Subject 已知道该正文，不表示 2022 不含更新的历史网页。source publication、正文内事实时间、observation time、formation/recording time、CognitiveClock cut 分开解释。

CPython official snapshots 覆盖 PEP 703、PEP 779、3.13/3.14 how-to 和 extension docs；正文完整按 region 摄入。release-scoped 状态各自形成 Memory，没有强行把 3.13 experimental 修订成不同的 3.14 状态。关键失败：

- source `pep-703`，raw SHA256 `9ca62cc097c11a0ca33b7e5cddb074133bbbe2217a27ddaf9edad29b83b170c9`；header `Created: 09-Jan-2023` 与 `Resolution` URL。
- Memory `01a11601-3b1b-7370-8b55-693fb52e2096` / revision `01a11601-3b1b-7370-8b55-6942f27cd87a` 将该日期断言为 Steering Council acceptance。
- Tag `01a11602-4b72-71a1-bd53-d5aed2a13b66` 继续使用错误日期。exact refs 结构合法，不证明 claim 有语义支持。

独立读取并冻结[所链接的 Steering Council resolution announcement](https://discuss.python.org/t/pep-703-making-the-global-interpreter-lock-optional-in-cpython-acceptance/37075)：帖子日期 2023-10-24，并引用此前公告；不把它断言成唯一首次接受日期。原 review 把该日期写成 PEP header 的 `Resolution` 日期不准确，addendum 已更正。失败结论仍是无依据地把 creation 晋升为 acceptance。证据支持 model grounding/concept propagation 缺陷，没有证明跨 Subject、owner invariant 或 temporal query 腐坏。

## Retrieval 与 associative evidence

Simon 共记录 28 arms，18 真正执行、10 prepare blocked。18 包含早期 baseline exploration control 和后来的 native exploration，不把它们伪装成同一 profile。`personal-practice`、`personal-recurring-project`、`personal-chronology`、`personal-tools` 的 formed/raw expected sources 均在 Top-5；ranks/MRR 见稳定 metadata。

`personal-origin` 原问题已经命名 Simon Willison，但 `his` 仍被当成未闭合 pronoun，错误为 `UNRESOLVED_QUERY_REFERENCE`；raw-control 同一原问题取得 expected source。`writing-recurrence` 的 plain/context/existing/model/四 profile 和 source-time 问题遭到同类拒绝。没有改文本、注入 gold Entity 或给 formed 偷开兼容性。该 query closure owner 缺陷需要独立修复后重跑。

salary absence 返回相关资料但未生成薪资 claim；retrieval API 不是 answer/refusal 系统，absence 无 source recall 分数，不能宣称已证明拒答。early as-of/history 没有 future source matches；不扩大为全部历史 cache/topology 路径验收。

直接 Tag seed `01a115fd-3b37-7890-b885-f880ab5d5c57` 有 3 个 2022 source attachments。native `nous-node-potential-v1` 经 `assoc.related` 到 Tag `01a115ff-1397-78c2-8928-1d95b623ca00`，再经 `tag_attachment` 到 Link Blog revision `01a115fe-d1d9-71f3-b2d2-69a2008b9b6b`。route support 包含 2022 advice 与 2024 link-blog roots，新增项不属于直接 attachment。实际 max hop=2、visited nodes=8、activated edges=13；保留 `topologywave_lane_truncated` / `complete=false`。这是条件性关联价值，不能代表公平四 profile 比较已完成。

## Feedback 与 revision

Simon 消费实际 TagDirect query `01a11615-8b63-7303-95dd-4368059b997f` 返回的 revision `01a115fc-cb16-7fa2-a269-f1c93ac2383e`，通过 namespaced consumer 提交 presented、result_supported、result_refuted。每项首次 accepted=1，第二次 duplicate=1/accepted=0；Grant 前 Tags/Associations 不变。

presented-only Grant 同时遇到自然 hard-idle Episode settlement，随后对新 Episode 做 concept maintenance（trace 78）。focus 是 Episode、`queryFeedback=[]`，不能归因于 presented reinforcement。极薄 Episode 内容形成 generic Tag 也是质量不足的观测，不能因结构合法就计成概念成功。

后续 trace 79 的 concept input 可见 meaningfulUse=1、counterevidence=1；queryFeedback 保留 exact revision 与 explicit Tag/profile。支持和反证没有合并成两次正向使用。Host Grant 后移除旧 center 的一个 attachment，形成有来源支持的 Datasette release Tag；Memory 正文未改写。验证的是身份、幂等、归因、写入边界，不要求反馈必然新增 Tag。

独立 CPython slice 使用 3.13/3.14 同一 GIL re-enablement runtime rule 的自写概括与澄清，fresh Subject `3007dc7e-a644-5b0b-a159-988a92e40263`，没有修复失败 main Authority。初始 revision `01a1160f-f439-7b72-8783-7743e4dd5553` → rephrase revision `01a11612-4260-77a1-b91b-a37ca12ee024`；explicit Tag 不在 revise payload 中复制。direct recall 1→0→1（Host maintenance 后），结果 `continuity_lost_until_maintenance`。旧 cut 只返回旧版，public history 和 lexical `$history` 可见两版。合同不要求无条件 Tag 继承，因此记录可见性空窗，不自动判定 production defect。

## Schema、Accretion 与 sealed 候选

main formation 均无自然 Episode/Journal/Schema settlement；参考 idle/settle policy 为 300/1800/300 秒，bounded grants 不强行等待或捏造 recurrence。Schema 仅从 neighborhoods 发现，零计数不代表 exhaustive absence。Accretion trace 中 PEP 703 多 Memory members 仍为 distinctRoots=1，region 没有膨胀 independent corroboration。use/counterevidence 可区分，但未证明长期 recurrence、review priority 或 hub bias 改善。

Rust 候选覆盖 RFC 3185 旧快照、2023 async-trait announcement/1.75 release、tracking issue body 和 2026 Reference；issue comments 不在 snapshot scope。Kafka 候选覆盖 KIP 500 v29、KIP 866 v30、3.6/3.9 operations、4.0 upgrade official commit snapshots。3.6 ZooKeeper→KRaft migration 为 Early Access/不推荐 production，不能和 KRaft runtime readiness 混同。query/oracle 在 sealed outcome 前固定，raw/text hashes 已核验，live sealed 全部未执行。

归档 `qualification-lock.json` 的 calibration status 为 `BLOCKED`，保存 final runner/config/model/prompt/schema/embedding/source/query/oracle identities，不签发 passing sealed lock。生产缺陷修复后须建立新 calibration/version，不覆盖本轮失败证据。

## 实际 provider usage

| 类别 | Calls | Items / 说明 |
| --- | ---: | --- |
| Memory formation | 29 | Simon 7，CPython 22 |
| Concept maintenance | 32 | Simon 7，CPython 22，独立 revision 1，feedback 2 |
| Query concept enrichment | 0 | prepare blocked |
| Embedding | 18 | 46 items，含 Serving 与 query embedding |
| Rerank | 0 | 全部 forbidden |
| Provider failed attempts | 0 | runner/API validation failures另存，不是 provider calls |

共享 gateway 总计 79 attempts；prompt tokens=283818，completion tokens=84092，total tokens=367910，费用金额未知。Formation/concept 使用 `doubao-seed-2.0-mini` / openai-chat；embedding 使用 `doubao-embedding-vision` / openai-embeddings、2048 dimensions、配置 weights revision `doubao-embedding-vision-251215`。Provider 未给可钉住的 generation model revision，profile/prompt/schema/config hashes 已保存；不宣称日后相同名称可 bitwise 重放。

各 pack 的 `cost.json` 是同一 ledger 的不同累计截面，不能相加。所有 paid usage/失败/预算按真实 gateway telemetry 计入，raw/traces 不进 PR。

## 验证与保存

最终代码运行 repo-native `just check` 成功：Rust fmt/clippy（all targets/features、deny warnings）、TypeScript format/proto/typecheck/lint、24 test files/86 tests，以及 Rust workspace tests/doc-tests。研究合同的最窄检查为 2 files/10 tests，零 provider calls。文档导航 `documents=60 human=51 indexes=5 issues=0`；全部四包 raw/text SHA256 再核验，diff whitespace/credential-pattern/production-file 检查通过。CI 不下载 live corpus、不读取 credential、不调用付费模型。

研究结束后普通研究 Core 与 gateway 已优雅停止，9471/18001 端口已释放；fresh instance、旧实例、ignored corpus 与 traces 均保留。归档 identities 用代码提交 `bf67bd3` 和独立 runner digest，逐次 live invocation 自有历史 identities；最后的 guard/ingest/终态修正不冒充所有早期调用使用的代码。
