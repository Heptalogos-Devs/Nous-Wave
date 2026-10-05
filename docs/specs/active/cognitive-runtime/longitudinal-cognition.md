# 纵向认知

## Owner 与路径

Session-bound Observation → Runtime ExperienceItem → automatic Episode → Memory-owned Journal → host-granted Maintenance → Memory/CognitiveSchema consolidation。

Runtime 持有 CognitiveClock、Experience feed、segmentation cursor/draft 与 MaintenanceNeed。Material 提交 Observation；Memory 持有 Episode、Journal、Memory 与 CognitiveSchema Authority。Core 执行模型和有界维护机会；Kernel 组合 owner；Retrieval 提供可重建 Serving。

## 时间与经验捕获

正常运行使用 SystemCognitiveClock；内部测试和研究可注入按 Subject 单调推进的 ManualCognitiveClock。HTTP/DB timeout、重试和 worker lease 使用基础设施时间。公开 RPC 不提供 clock mutation。Core 通过 private `KernelQueryService.GetCognitiveTime` 读取 Subject clock；NousQL 的相对时间窗口也使用该认知时间。

Observation `observed_at` 可省略，缺省使用 CognitiveClock。发生时间保留调用方提供的已知或 unknown 语义。Subject、Cognitive Seed、Material 来源、Memory、CognitiveSchema、Episode、Journal 及语义 lifecycle/UseEvent 的 timestamps 由 owner 提交分配；工作流重放保留原 formation time。Memory、CognitiveSchema、Episode、Journal 的创建和修订输入不接受 caller `formed_at`；Schema 使用 `CognitiveSchemaContent` 输入。

Session-bound Observation 在 Material 提交事务内写入一个 ExperienceItem，复用现有 invalidation 返回的 Subject `authority_seq` 作为 `recorded_seq`。记录包含 Session、foreground WorkContext ID/revision 和边界上下文，并 upsert 到期的 `episode_segment` need。无 Session 的 Observation 保持普通 Material/Evidence，不进入自动分段。摄入过程中不调用模型。

## 初始 Episode 与语义 refinement

初始自动 track 为 `interaction`。Runtime 按 recorded sequence 推进 durable cursor/draft；Session 变化本身保持连续。hard idle、结束的 WorkContext、多个上下文切换，或 soft idle 伴随上下文/Session 切换可关闭 draft。无新观察时，到期维护负责 idle closure。ready draft 使用稳定 operation identity 经 Memory 提交；ack 验证对应 commit receipt 后删除 draft 和成员，cursor 不保留已删除 draft 的引用。Memory commit 后、ack 前崩溃时，ready draft 和稳定 operation identity 重放得到原 Episode，再完成 ack。迟到观察排入有界局部 resegmentation。

`recorded_seq` 决定 exactly-once processing、durable cursor 和摄入顺序；experience chronology 使用 `(observed_at, recorded_seq)`。draft 最终成员、局部 repair、模型 partition input 与提交的 Episode 都按 experience chronology 排列，同 observed time 用 recorded sequence 确定顺序。

历史 repair 以迟到 Observation 的 observed time 定位同 parent/track 的 overlapping current Episodes 和两侧最近邻，构建局部 partition 后加入迟到 occurrence。`episode.max_neighbor_span_seconds` 限制受影响 neighborhood 的 experience span；经历距今天的年龄不限制 repair。neighborhood 的 Episode 数、experience span 或 materialization safety budget 超界时进入 `historical_repair_scope_exceeded` blocked。

`episode_segmentation` 接收精确、有序 member keys 和当前 Episode 组织，返回 `no_change` 或完整 partition。Memory 原子应用 N-to-M partition：1-to-1 修订同一对象；split/merge/general partition 创建替代对象并 withdraw sources，保留 exact lineage。缺失、重复、重排或虚构成员使整次提交失败。

## Maintenance 与工作流

`RuntimeService` 承载 canonical Session、WorkContext、Observation 和 Use 操作；Core 的 `CognitionService` 提供公开维护 grant 与查询/投影编排。Core 通过 private `KernelMaintenanceService` 调用 durable need 和 longitudinal owner commit，通过 `KernelModelWorkflowService` 管理模型执行重放状态。

MaintenanceNeed 在 active scope 内合并 trigger，due time 使用 CognitiveClock。并发 claim 使用数据库 row locking；基础设施 lease 与 token fence 控制 worker，过期 lease 可重新 claim。finish 保留执行期间新增的 trigger。

公开 `GrantMaintenance` 接收 `max_operations`、`max_model_calls`、`max_elapsed_ms`。Core standalone loop 完整使用 active Subject pagination，保存跨 poll tick 的 continuation；每个 Subject 每次获得至多一个操作机会，耗尽 tick budget 后从停止处继续，列表尾部 wrap。删除、停用或失效 page token 会跳过或重新建立 cursor。Core restart 可以从稳定列表起点开始。全局 operation/model/elapsed budgets 限制每次 tick；Kernel 没有自主认知 cron，只持有 durable needs、claim、lease 与 owner semantics。

维护种类为 `episode_segment`、`episode_resegment`、`journal_review`、`journal_revalidate`、`memory_consolidate`。Core 根据当前可执行角色构造 claim 的 allowed kinds：resegment 需要 `episode_segmentation`，Journal review/revalidate 需要 `journal_synthesis`，consolidation 需要 `memory_consolidation`；初始 segmentation 无需模型。未就绪角色的 needs 保持 durable、无 worker lease、attempt count 不增长。角色配置按当前 Core model runtime 的 restart 生效合同处理。

MaintenanceNeed 状态为 `pending / leased / blocked / satisfied / obsolete`。deferred 表示时间推进或已知未来事件能够改变条件，保留带 next due 的 pending work。blocked 表示依赖、配置或输入需要实际改变，不参与普通 due-time lease。来源 revision/lifecycle、显式 maintenance refresh 或有效配置快照变化可重新激活 blocked need；retry exhaustion 还可由当前 executable model configuration digest 的变化唤醒。真实 provider/network 故障使用配置的指数退避，等待基础设施时间；连续失败达到 retry attempt 上限后 blocked。新 trigger 重置 retry 状态，每个 need 的 trigger revision 随真实 trigger 或配置唤醒推进；执行期间的新 trigger revision 保留为 pending work。

unexpected scheduler/runtime failures 输出 Subject、stage、已知 need kind、稳定 problem class 和 retry/blocked decision；相同 scheduler failures 按重复次数聚合。凭据、配置值和原始 model payload 不进入诊断。正常 blocked 或未配置角色无需 exception 日志。

Episode/Journal/Memory/Schema revisions 是 durable cognition history；Observation/ExperienceItem 是 durable experience；EpisodeDraft、MaintenanceNeed 和 model workflow 是 temporary runtime work state。terminal need 按 `maintenance.terminal_retention_seconds` 保留短期 finish replay，再在 claim/ack 路径删除；pending、leased、blocked 不被 terminal housekeeping 删除。

Core 的 `model_workflow_operations` 保存固定 model/plan snapshot、proposal 与 outcome。operation identity 由 need、trigger authority sequence 和 trigger revision 生成，proposal 在 owner commit 前持久化；重试复用已存 proposal/outcome。maintenance workflow 显式绑定 need 和 trigger；ack 成功后删除已完成的关联 workflow。新 trigger 已替代的旧 workflow 在执行 lease 释放/过期后回收。显式 Memory formation、Material workflow 和仍需 retry 的当前 workflow 保持自身合同。owner 验证 Subject、head、epoch、lifecycle、来源 catalog 和 authority sequence；变化返回 stale 并刷新维护计划。

Workflow owner 是 1..64 字节的 semantic identifier，格式为 `[a-z][a-z0-9_]*`；Persistence 不枚举认知领域。private Kernel API 仅接受已实现的 `memory / material` 调用路径；Longitudinal 的 Memory-domain operations 使用 `memory`。

## Journal

JournalId 是稳定对象，JournalRevisionId 是不可变 exact narrative。每个 point 有 role、text 和至少一个精确 support；bounded narrative 与 points 对应。`journal_synthesis` 使用 settled EpisodeRevision sources 和 supplied support keys，模型不能创造来源引用。

创建、修订、get/exact revision、history/list、suppress/restore、withdraw/reaccept 与 purge 均归 Memory。source revision/lifecycle/purge 变化使当前 Journal `revalidation_required`，排入 `journal_revalidate`；重建提交新的 revision，scope 消失可 withdraw。Journal provenance 展开原来源 lineage，Journal 不增加独立 evidence root。

Journal 重验证等待全部当前来源稳定后再规划完整集合。来源数量或时间跨度超界时，Journal 保持 `revalidation_required`，need 分别进入 `journal_scope_bound_exceeded` 或 `journal_scope_span_exceeded` blocked；时间推进不重新运行。相关来源/Journal 变化、配置 bound 变化或显式 refresh 可以唤醒。重验证覆盖声明的完整 source scope；新 Journal review 可按边界选择已稳定的前缀。

## Consolidation

整合与 Journal synthesis 可读取手工 Episode 的 Occurrence 或 cognition members；普通 Occurrence 的目录条目保留正文和观察时间，Session 与 experience recorded sequence 仅在实际存在时提供。自动 resegmentation 使用 Session ExperienceItem 的完整顺序。

`memory_consolidation` 使用 `MemoryConsolidationText`，scope 为 current eligible EpisodeRevision 或 JournalRevision。计划包含有界 source/member/support/entity catalogs、independent roots 和通过 Query/Serving 选出的最多 16 个当前 Memory/Schema context candidates。候选包括 exact revision/epoch、正文、Schema applicability/boundary/tags、独立时间轴和按 use kind 汇总的 meaningful use；presented 不计入摘要。

proposal action 为 skip、create/revise Memory、create/revise CognitiveSchema、link relation。引用只能选择 supplied catalog 或较早 action result。Memory 将整组 proposal 作为一个原子事务验证与提交；沿用 identity、formation、provenance、independent roots 与 relation owner 合同。skip 不修改认知 Authority。Journal 是可选来源，Episode 可直接整合。

Query/Serving、WorkContext、UseEvent 和下游失效合同分别见 [Query](../memory-reference-profile/03-query-serving.md)、[WorkContext](work-context.md)、[Use](../memory-reference-profile/02-runtime-use.md) 和 [Authority](../memory-reference-profile/01-memory-authority-provenance.md)。

[返回当前产品合同](../../INDEX.md)

## Independent topology maintenance

`topology_maintenance` 是独立 kind，复用 durable need、host grant、lease、retry、固定 ModelWorkflow snapshot 与全局模型调用/elapsed 预算。accepted/revised Memory、Episode、Journal、CognitiveSchema 的 owner mutation 在同一事务用实际 Authority sequence 排入 exact focus；meaningful use 跨过 typed threshold 时排入 review，exposure 不触发。

Memory owner 的 planner 使用有界当前 cognition、Tag、association、Entity、exact/source/use support 与 provenance catalog。Semantic similarity 只读取兼容 space/producer 下的既有 embedding material，不调用 provider；没有材料时保留显式 partial。模型仅看到 invocation-local keys 与语义描述，owner snapshot 保存 exact identities/epochs；proposal 只引用 catalog keys 或顺序创建的 `new_` keys。

Core role `topology_maintenance` 使用独立 Structured Contract 与 Prompt，支持 reuse/create/revise/attach/detach Tag、create/revoke association、merge/split Tag、no_change。Merge/split 沿用 Tag owner 的 exact support 与 independent root 条件。owner 在一个 MutationEnvelope 中复核 lease、focus、当前 catalog/policy、局部 key、self-loop、duplicate/action/support budget、exact cognition endpoint 支持和 provenance，再写入所有动作与一个 receipt；非法动作整体回滚。没有变化不重建 Serving，已提交 proposal 在丢失 response 后幂等重放。

post-hoc `tag_attachment` 保存在 AssociationEvidence，不改变旧 MemoryRevision。共享 relation registry 将 tag_attachment、assoc.related、assoc.co_occurs、assoc.shared_outcome 投影为对称 adjacency，sequence/procedural 保留方向。Authority evidence 保留原端点、方向和支持；逆向 Serving edge 使用同一 provenance root。Episode/Journal exact revisions 可以作为显式 association endpoint。

Owner 校验关系特有的结构证据：共现需要共享 occurrence 或选中的 Episode 成员；顺序需要 Episode 成员顺序或不重叠的 occurred time，observed time 不证明先后；程序/共同结果需要覆盖两端的已分类 cognition witness。Planner 提供认知类型/角色、Entity aboutness、时间、Episode 局部成员顺序和 source context；Tag 候选优先已关联概念与文本相关候选。

Accretion 是可重建的派生信号，聚合独立根、当前成员、跨 Episode 重现、观察跨度、关系多样性、语义 coherence、meaningful use 与反证。缓存缺失时 coherence 保持 unknown；不调用 embedding。有效使用提高有限 usefulness，不更改 epistemic class、不增加独立根。宽泛中心被降权并进入 split review；成员高度重叠且至少两独立根的中心提供 merge hint；最终动作仍由独立 role 提案、owner 验证。配置可关闭 Accretion 作 ablation。

数据库场景验证新 Tag、后续复用、关联、非法 key 原子回滚、Tag-only Prepared Serving、无凭据关系拒绝与 Episode 顺序/共现/程序 witness。有效使用提高 usefulness，原 observed 类别与独立根不变；Episode review 不受 Accretion 中心类型限制。Core 测试验证固定输入、预算 reservation 与 proposal 重试。全类别 Functional Corpus、真实 provider/Agent Loop 和独立文本兼容性仍待本 PR 验收。

维护 catalog 的 sourceContext 包含有界原始文本或派生 descriptor，并链接 cognition.sourceSupportKeys；Episode 的 idle/boundary 机制不作为领域概念文本。每项 cognition 显式提供 exactSupportKey。create_tag 使用必填 cognitionKeys 指定 accepted cognition 锚点，由 owner 生成精确 revision supports，再与所选 source supports 去重；attachment/association 从显式局部 endpoint keys 记录 exact revision anchors；模型选择 source 或额外 cognition witness，owner 仍执行关系证据和 provenance 校验。

Consolidation 的候选查找使用独立 text-only lookup：输入是引用的 source 原文，不作为待闭合的用户意图。实体目录提供 source 文本中出现的 display name/alias 的有界候选，加上已有候选的 aboutness；目录匹配本身不写入 aboutness，仍由模型选择、owner 校验。形成阶段需要 lexical Serving 为 continuing claim 提供当前候选，dense/topology 与付费 embedding 在检索验证前保持关闭。

Directory 的当前有效 Entity binding 是 Subject 内可引用身份，即使尚未被 Memory aboutness 或 observation actor 使用。通用 reference 校验承认该 binding；不把它自动转换成 aboutness。跨 Subject 和 tombstoned binding 仍不能仅凭目录获得可引用资格。
