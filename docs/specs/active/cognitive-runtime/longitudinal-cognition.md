# 纵向认知

## Owner 与路径

Session-bound Observation → Runtime ExperienceItem → automatic Episode → Memory-owned Journal → host-granted Maintenance → Memory/CognitiveSchema consolidation。

Runtime 持有 CognitiveClock、Experience feed、segmentation cursor/draft 与 MaintenanceNeed。Material 提交 Observation；Memory 持有 Episode、Journal、Memory 与 CognitiveSchema Authority。Core 执行模型和有界维护机会；Kernel 组合 owner；Retrieval 提供可重建 Serving。

## 时间与经验捕获

正常运行使用 SystemCognitiveClock；内部测试和研究可注入按 Subject 单调推进的 ManualCognitiveClock。HTTP/DB timeout、重试和 worker lease 使用基础设施时间。公开 RPC 不提供 clock mutation。

Observation `observed_at` 可省略，缺省使用 CognitiveClock。发生时间保留调用方提供的已知或 unknown 语义。Memory、Episode、Journal 的 formation/recorded timestamps 由 owner 提交分配；工作流重放保留原 formation time。创建和修订协议不接受这三类对象的 caller `formed_at`。

Session-bound Observation 在 Material 提交事务内写入一个 ExperienceItem，复用现有 invalidation 返回的 Subject `authority_seq` 作为 `recorded_seq`。记录包含 Session、foreground WorkContext ID/revision 和边界上下文，并 upsert 到期的 `episode_segment` need。无 Session 的 Observation 保持普通 Material/Evidence，不进入自动分段。摄入过程中不调用模型。

## 初始 Episode 与语义 refinement

初始自动 track 为 `interaction`。Runtime 按 recorded sequence 推进 durable cursor/draft；Session 变化本身保持连续。hard idle、结束的 WorkContext、多个上下文切换，或 soft idle 伴随上下文/Session 切换可关闭 draft。无新观察时，到期维护负责 idle closure。ready draft 使用稳定 operation identity 经 Memory 提交；ack 与 cursor/draft 保存使重启和重试保持连续。迟到观察排入有界局部 resegmentation。

`episode_segmentation` 接收精确、有序 member keys 和当前 Episode 组织，返回 `no_change` 或完整 partition。Memory 原子应用 N-to-M partition：1-to-1 修订同一对象；split/merge/general partition 创建替代对象并 withdraw sources，保留 exact lineage。缺失、重复、重排或虚构成员使整次提交失败。

## Maintenance 与工作流

MaintenanceNeed 在 active scope 内合并 trigger，due time 使用 CognitiveClock。并发 claim 使用数据库 row locking；基础设施 lease 与 token fence 控制 worker，过期 lease 可重新 claim。finish 保留执行期间新增的 trigger。

公开 `GrantMaintenance` 接收 `max_operations`、`max_model_calls`、`max_elapsed_ms`。Core standalone loop 定期提供机会；Kernel 没有自主认知 cron。维护种类为 `episode_segment`、`episode_resegment`、`journal_review`、`journal_revalidate`、`memory_consolidate`。结果区分 committed、no_change、obsolete、rejected_invalid 和 blocked_dependency；settling 或缺少模型可留下后续 due work。

Core 复用 `model_workflow_operations` 保存固定 model/plan snapshot、proposal 与 outcome。operation identity 由 need/trigger 生成，proposal 在 owner commit 前持久化；重试复用已存 proposal/outcome。owner 验证 Subject、head、epoch、lifecycle、来源 catalog 和 authority sequence；变化返回 stale 并刷新维护计划。

## Journal

JournalId 是稳定对象，JournalRevisionId 是不可变 exact narrative。每个 point 有 role、text 和至少一个精确 support；bounded narrative 与 points 对应。`journal_synthesis` 使用 settled EpisodeRevision sources 和 supplied support keys，模型不能创造来源引用。

创建、修订、get/exact revision、history/list、suppress/restore、withdraw/reaccept 与 purge 均归 Memory。source revision/lifecycle/purge 变化使当前 Journal `revalidation_required`，排入 `journal_revalidate`；重建提交新的 revision，scope 消失可 withdraw。Journal provenance 展开原来源 lineage，Journal 不增加独立 evidence root。

## Consolidation

`memory_consolidation` 使用 `MemoryConsolidationText`，scope 为 current eligible EpisodeRevision 或 JournalRevision。计划包含有界 source/member/support/entity catalogs、independent roots 和通过 Query/Serving 选出的最多 16 个当前 Memory/Schema context candidates。候选包括 exact revision/epoch、正文、Schema applicability/boundary/tags、独立时间轴和按 use kind 汇总的 meaningful use；presented 不计入摘要。

proposal action 为 skip、create/revise Memory、create/revise CognitiveSchema、link relation。引用只能选择 supplied catalog 或较早 action result。Memory 将整组 proposal 作为一个原子事务验证与提交；沿用 identity、formation、provenance、independent roots 与 relation owner 合同。skip 不修改认知 Authority。Journal 是可选来源，Episode 可直接整合。

Query/Serving、WorkContext、UseEvent 和下游失效合同分别见 [Query](../memory-reference-profile/03-query-serving.md)、[WorkContext](work-context.md)、[Use](../memory-reference-profile/02-runtime-use.md) 和 [Authority](../memory-reference-profile/01-memory-authority-provenance.md)。

[返回当前产品合同](../../INDEX.md)
