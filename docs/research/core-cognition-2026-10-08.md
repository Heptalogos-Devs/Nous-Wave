# Cognition Qualification v2 — 2026-10-08

[方法](core-cognition-semantic.md) · [稳定结果与 identities](corpus/core-cognition/results-2026-10-08.json) · [v1 原始结果](core-cognition-2026-10-07.md) · [返回 Research](README.md)

## 实际决定

本轮完成 fresh Simon/CPython calibration，以及锁定身份后的 Rust/Kafka sealed formation 和 retrieval。认知错误继续进入研究，来源或 owner 完整性错误才阻塞受影响实验。四套共 143 个 retrieval arms；存在 partial Native readout，全部原始 degradation 保留。

保留 enrichment 默认 `off` 与 baseline retrieval，不晋升 Native、DTSC 或 RiverMemo。可靠识别的指称应显式化：原始问题中的具体名称、任务 WorkContext、已确认 Entity 和 Tag 可以共同参与检索。无法消解的自然语言仍可执行；context 的价值取决于相关性，不能承诺加 context 总会提高排名。

同一 mini 模型的高推理 formation 在 PEP 703 首次形成中避免了 creation→acceptance 混淆，但后续默认 concept maintenance 又引入错误月份；高推理 concept review 未纠正它。因此高推理 formation 只有该角色、该来源上的条件性价值，不能替代概念维护的来源约束，也不据此修改生产 Prompt 或部署默认值。

## 执行身份与基础设施

重新 fetch 后的 Nous `master` 为 `db64f660de0741f60e277ced71ee9eef98763594`；Vault `main` 为 `5b96c63da34b0a4c697b6961ae10ba6aa4de3ee1`。完整读取当时 AGENTS、Vault Authority、Active Specs、v1 报告/结果与下载包。全程使用 `codex/cognition-qualification-v2` 和 [PR #20](https://github.com/Heptalogos-Devs/Nous-Wave/pull/20)。真实研究代码为 `7ac48d2fffc49494bcbf91260df6edf050c930d4`；后续报告提交不改变已经执行的代码身份。

使用普通 Core、official Client 和现有 research gateway。fresh instance 为 `data/instances/core-cognition-v2`，复用 canonical PostgreSQL 私有运行时。原有 New API Docker stack 已启动，入口 `http://127.0.0.1:3000`；研究 gateway 为 loopback 18001，Core 为 9471。凭据只读取原 SecretRoot，未进入 tracked 文件。全局 maintenance 关闭，各研究 Subject 显式接受 Host grant。formation 阶段关闭 dense/topology，retrieval 阶段开启；实际阶段 config digest 和 producer metadata 分别保存。

实际模型为 New API 所提供的 `doubao-seed-2.0-mini`，默认与 `reasoning_effort=high` 两个 ExecutionProfile；embedding 为 `doubao-embedding-vision`、2048 维。别名不证明上游真实 weights revision。所有研究查询禁止 rerank。没有另建 Runtime、Benchmark 框架或 qualification matrix。

ignored `data/research/runs/core-cognition-2026-10-08/` 保存实际 raw/text、manifests、review、operation intents/receipts、完整 query output、request/response trace、ledger、阶段 config、封存锁和日志。稳定 metadata 保存来源 URL/version/raw/text/extraction、Subject/ref、指标、逐次 wire usage/hash、角色成本以及关键文件 SHA256。Memory producer 可经 public API 读取；仅被 cognition Association 引用的 concept producer 读取返回 `not_found`，原错误与 signature ID 保留，实际角色仍由完整 wire trace 归因。

CPython 六份固定 commit 原文与 v1 一致，Rust 五份 raw 一致。Simon 网页 wrapper 和 Kafka Confluence wrapper 的本次 bytes 与 v1 不同，按实际 bytes 冻结；Kafka 页面 meta 对应既定 page versions 29/30。Simon 提取文本开头仍含 v1 identity 提示，实际 v2 raw hash 由本轮 manifest 拥有，不能把旧提示当作 bytes 相同的证据。Kafka extraction 在模型调用前已冻结为完整 KRaft/migration 小节及完整 4.0 upgrade 小节。既有 tracked v1 manifests/oracles 未改写。

## 形成、持续、传播与纠正

| Fresh calibration | 冻结 retrieval review 时的认知 | 实际语义观测 |
| --- | --- | --- |
| Simon | 7 Memory / 2 Tag / 7 Association | Datasette Tag 吸收一般 blogging advice；TIL Tag 的 attachment 范围包含更广泛 link-blog。保留原文中的后来更新，没有把页面年份当作所有 claim 的年份。 |
| CPython baseline | 22 Memory / 6 Tag / 22 Association | 初次 Memory 将 PEP 703 Created 日期误写为接受日期，后续来源到来后仍保留同一 revision。baseline PEP Tag 聚合接受状态，但没有复制错误的精确日期。 |
| CPython high formation | proposal checkpoint 的 8 Memory / 2 Tag / 15 Association | 高推理初次 Memory 区分创建日期和 Accepted 状态；默认 concept role 随后在 Tag 中新增“January 2023 accepted”。此 arm 只执行到 proposal，没有冒充完整四 checkpoint 对照。 |

[固定 PEP 703](https://raw.githubusercontent.com/python/peps/1d09abe70138f530d7dd3307e5cbc053a929b923/peps/pep-0703.rst) 的 Created header 不提供接受日期。baseline 首次 Memory `01a118f2-0beb-7c82-a2ac-6222e27c5328` / revision `01a118f2-0beb-7c82-a2ac-6233c8e49ca7` 的错误直到明确干预前仍持续。另一份标题为 py314-howto 的冻结来源本身保留 3.13 overhead 和 prospective 3.14 语言，生成 3.13 标题属于 source-faithful；3.14 Phase II 事实由 PEP 779 提供。

高 formation 正确 revision 为 `01a118fa-78e1-70c2-bc12-0b2cdf0b2c4a`；错误 concept Tag 为 `01a118fc-1fab-7ea3-bc05-e7a9607667e5`。随后切换的只是 concept maintenance 的高推理 route，真实 referenced use 与 Host grant 触发两个高推理调用，共约 40.9 秒。错误 Tag 的描述与 revision 均未改变。这检验的是使用后概念复审，没有伪造不受 API 支持的 Tag refutation，也没有把 oracle 写入该 arm。

原 baseline Subject 上进行了 source-backed Host `correct`，保留同一个 Memory 对象与实际 occurrence basis，新 revision `01a11909-d327-7b52-8100-5cadf014398d`。修订内容明确 Created 与 Accepted 是不同事实、header 未给接受日期。真实 query 返回的旧 exact revision 接受 `result_refuted`；重复事件 accepted=0/duplicate=1。maintenance 提交新 revision 的概念工作。current 排除旧版，修订前 as-of 排除新版，history 保留两版。Session ResidentSet 仍可携带原 exact revision，未被偷偷替换成 mutable current head。该纠正是明确 Host 干预，不是后续来源自动纠错。

Simon 的 presented、supported、refuted 使用同一实际 query/revision，首次 accepted=1、重复 duplicate=1，grant 前未改变概念。Host grant 后一次 concept call 返回维护结果，仍为两个 Tag；反馈可驱动复审，不要求每次都新增概念。

真实 hard idle 后，两 Subject 各由普通 Host grant 结算一个 Episode，成员分别为 7/22 个真实 Occurrences。首次 concept review 都为 `no_change`。按原配置等待实际 300 秒 settling 后，Simon 的 Episode review 无变化，Journal 提交，consolidation 从三份真实来源创建 Schema `01a1191c-f2e2-7553-bf7c-43453851feb2` / revision `01a1191c-f2e2-7553-bf7c-435aceabf67b`。显式 public Schema query/read 确认提交；原 neighborhood snapshot 没有穷尽 Schema，不能据其零项宣称不存在。Schema 概括 Simon 的博客/项目记录实践，范围限定该作者，属于宽泛描述，尚无可迁移规则的证据。Journal 将“2020 年开始 TIL、2022 年已有 346 条”压缩为“2020 年已 346 条”，新增时间归因错误。CPython 的 Journal 提案用 occurrence UUID 字符串代替 invocation-local support keys，被 owner `rejected_invalid`；没有提交损坏 Journal。其 consolidation 为 `skip/no_change`。两个角色调用均通过普通 Core，不修改时钟或人工注入期待 Schema。高推理 consolidation 没有成对实测，状态 NOT_RUN；本轮不据默认两次调用选择该角色的更强 profile。

## Context 与检索结果

平均值仅覆盖每套 `plain`、有 expected sources 的原始问题；absence 分数为 null，Tag/time/profile arms 不混入均值。短跟问是新增的小型诊断问题，使用先于输出冻结的任务文本，未注入答案或期望来源。raw_control 通过 public Client 保存同一原始输入的对照 Memory。

| Pack / track | 有分数问题数 | Recall@1 | Recall@5 | Recall@10 | MRR |
| --- | --- | --- | --- | --- | --- |
| Simon formed | 8 | .562500 | .958333 | 1 | .937500 |
| Simon raw | 8 | .500000 | .958333 | 1 | .854167 |
| CPython formed | 11 | .590909 | .909091 | .977273 | .954545 |
| CPython raw | 11 | .454545 | .863636 | .931818 | .831169 |
| Rust formed | 8 | .479167 | 1 | 1 | .791667 |
| Rust raw | 8 | .354167 | 1 | 1 | .729167 |
| Kafka formed | 10 | .475000 | .900000 | 1 | .833333 |
| Kafka raw | 10 | .475000 | .900000 | 1 | .833333 |

Simon 的 `How did it change later?` plain Recall@5=2/3、MRR=.5；foreground task text、真实 Tag、Entity+Tag 与显式 rewrite 为 Recall@5=1/MRR=1；Entity 单独为 2/3/MRR=1。CPython 同一短问 plain Recall@10=.75，task text/Entity/Tag/combined/explicit 到 1，但 Recall@5 都停在 .75。保存的 prepare input 确认 Session foreground 的真实 WorkContext、名称及 anchors 被消费。runtime_context/work_context 中相同 ref 的重复诊断来源不算独立佐证。

另一方面，CPython `native-boundary` 加 text context 后 MRR 从 .5 降到 .2；Kafka bridge Entity-only 从 MRR=1 降到 .5。Context 应保存当前任务，避免无关历史与大量 resident 挤占当前 intent；提供能可靠识别的具体名词和 anchor，再按实测结果决定是否保留。

QueryRepresentation 的修正将预算分配与输出顺序分开：intent → explicit selector/time/concept → pinned Entity/Tag → 当前任务 purpose/questions/text → pinned cognition → residents/background。实际预算回归使用 16 个 resident descriptors，当前任务与问题仍保留，低优先项有来源明确的 truncation flag。优先级不是给所有 pinned 长文本无限预算。

## Enrichment、Native/VCP 与 sealed

Simon writing-recurrence 模型 enrichment 将 Recall@5 从 1 降为 2/3，延迟约 .25→8.42 秒；CPython native-boundary 从 .5 降为 0，约 6.57 秒。Rust/Kafka 两个诊断问题的 model arm 均没有 Recall@5 增量，部分 MRR 降低；请求延迟约 7–9 秒。existing enrichment 在这些题上没有稳定增益。保留默认 `off`。

四 profile 使用每题相同 frozen prepared activation。CPython Native 与 RiverMemo 在 native-boundary 丢失 py314-howto 的 Top-10 support，baseline/DTSC 保留。Rust weak-cue 的 RiverMemo MRR 从 1 降为 .5；Kafka weak-cue 四者都保留 Recall@5=1，未提供晋升依据。Native 探索 arm 均记录 lane work budget 截断，不隐藏 `partial`/`complete=false`。

Simon true TagDirect family 只直接附着 LinkBlog；Native 经 `assoc.co_occurs` 一跳加入 beats，超出该 attachment。但 beats 已在 lexical/dense pool 中，并未成为新增 Top-10 source。实际两跳 witness 为 LinkBlog revision → Simon Entity → DatasetteBlog revision，两边均为 `aboutness/derived_structure`，且结果已由普通检索返回。这是实际关联路径，不能提升为独立来源、因果链或新的 recall 收益。五个 calibration 路径的完整 support/provenance refs 留在 metadata。

calibration 形成上述决定后，01:08:31 UTC 的 `qualification-lock.json` 锁定 code/runner/model/Prompt/schema/config/embedding/Kernel/Vault/source/oracle，随后才调用 sealed 来源。Rust 31 arms、Kafka 35 arms 完成；原始 queries/oracles 未按结果改写。所有 historical/time arms 的 forbidden source matches 为 0，四套 empty results 为 0。Kafka 首次保存 state 的 Windows `EPERM` 在 RPC 前发生；保持原 BLOCKED execution，使用同一 stable operations 恢复完成，未重放未知付费副作用。

Rust 六 Memory/三 Tag/九 Association 保留了 RFC 静态 dispatch、1.75 稳定、Send-bound 与 dyn 限制的发展。但 later Reference Memory 的宽泛概括遗漏明确 `async fn` 禁止条款，正确来源命中仍不足以回答该限制。Kafka 十一 Memory/三 Tag/十一 Association 保留 design、dual-write、3.9 finalization 不可逆与 4.0 KRaft-only；KIP-500 首条标题误升格 JIRA 编号，3.6 Memory 虽保留 Early Access，却遗漏明确 production/downgrade 限制。4.0 Memory 的 KRaft-only 正确，migration-before-upgrade prerequisite 没有完整显式保留。sealed 支持当前部署判断，也继续暴露忠实压缩不足。

## 实际调用、费用与验证

以下 latency 是实际 wire 请求，跨来源聚合不构成随机化的 profile 速度因果比较。高 formation 只有相同 proposal 八单元对照；默认 formation 总量还包含其余来源。

| Role / reasoning | 调用数 | tokens | wire p50 ms |
| --- | --- | --- | --- |
| memory_formation / default | 46 | 193,592 | 7,343 |
| memory_formation / high | 8 | 41,283 | 12,651 |
| concept_maintenance / default | 57 | 317,662 | 8,026 |
| concept_maintenance / high | 2 | 13,361 | 20,211 |
| query_concept_enrichment / default | 7 | 15,000 | 8,788 |
| embedding | 91 | 288,203 | 194 |
| episode_partition / default | 2 | 35,973 | 20,117 |
| journal_synthesis / default | 2 | 37,170 | 20,587 |
| memory_consolidation / default | 2 | 105,212 | 16,634 |

包含自然 settling，共 217 次实际 wire 调用、1,047,456 tokens、HTTP failures=0；没有 provider fallback。New API consume 日志与调用数、prompt/completion token 汇总一致，实际扣费 48,148 quota units。按本机 `quota_per_unit=500000`、CNY display/USD exchange=6.9，平台 credit 等值约 USD .096296 / CNY .6644424；上游供应商账单仍 unknown。各 pack 的 `cost.json` 是同一 ledger 的累积快照，不能相加。拒绝的 Journal 提案也包含在实际调用与扣费中。

生产代码 `7ac48d2` 已通过完整 `just check`、完整 `just smoke`，GitHub Linux check 与 windows-fast 均成功；最终文档提交再检查索引、链接、格式及同一 PR 的 CI。CLI 实际 Evidence query 返回 Occurrence/SourceRegion，`show result:2` 读取相同 SourceRegion；resource/mixed result 使用真实返回 refs 的 continuation 回归与 public smoke 通过。资源无本地外部 dataset，因此没有伪造外部查询成功。

官方 Agent 手册与 CLI help 使用先选 Subject、开 Session、foreground/reuse WorkContext 的 text-first 示例；强调强烈建议显式化可可靠识别的指称，并保留 unresolved 自然语言。CLI 不再用 cognition immutable-ref 条件拒绝 Evidence/Resource/mixed results，按实际 owner 提供 show/pin/use 后续动作。研究 runner 修正了过时 NousQL、Tag-only 丢失 TextCue 和把语义错误当 formation blocker 的逻辑。
