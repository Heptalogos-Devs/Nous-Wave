# Memory Authority & Provenance

## Owner

Memory (crates/memory) owns Memory/CognitiveSchema/Episode/Journal/Tag/AssociationEvidence Authority, provenance and lifecycle. Subject and Material provide the referenced identities and source records.

## 身份与来源

- `SubjectId` 是跨模型、Session 和进程稳定的主体身份；Model、Session、consumer 和 process 不是 Subject identity。
- `Artifact`、`ObservationOccurrence`、`DerivedRepresentation`、`EvidenceRef`、`Memory` 和 `MemoryRevision` 分别拥有独立身份。相同 Artifact 的多次观察不得被内容去重抹掉。
- Derived representation 保存 producer、输入和 source region；同一 source root 的多个摘要/模型重述不构成独立佐证。
- `EntityMention` 与 `CognitionAboutness` 分离；aboutness 不能反向伪造 source mention。
- 发生、观察、有效、形成、记录时间保持不同语义；未知时间保持未知。

## Memory object/revision

- Memory object 有稳定 identity；revision 内容 immutable；head、object epoch、lifecycle 和主体 authority sequence 分离。
- Cognitive role 与 formation mode 正交。当前 role 至少包括 `experiential`、`declarative`、`procedural_experience`；formation mode 至少包括 `grounded`、`synthesized`。
- 同一 referent、claim family、scope 和 world-valid interval 的纠正、澄清、收窄或重新解释创建新 revision；后续新 world state、独立事件或不同有效时间形成新 object/successor。
- Revision 提交必须检查 expected head/epoch、权限、source/provenance、lifecycle 和幂等 operation identity。head 变化不得静默覆盖。
- parent aboutness 非空时，revision 至少保留一个共同 aboutness identity；完全漂移必须创建新 object。
- contradiction、derivation、temporal successor、support、replacement basis 等是显式 relation；contradiction 不自动撤回对象。

## CognitiveSchema、Tag 与 AssociationEvidence

- CognitiveSchema 属于 Memory owner，具有独立 object/revision、applicability、support、counterexample/boundary evidence 和 lifecycle。create/revise/split/merge 使用 `CognitiveSchemaContent` 输入；formed/recorded time 由 owner 的 CognitiveClock 分配。canonical create/revise 可以携带经过规范化的 consolidation ProducerSignature；revision 接受新的 exact evidence links 与明确 copy links，保持 formation kind 和 aboutness continuity。Receipt replay 返回原 exact SchemaRevision，不因为后续 head 变化重绑。
- `explicit_import` 至少有一条有效 evidence；`synthesized` 需要至少两个 normalized inputs、至少两个 known independent provenance roots 和无 cycle；UnknownDependency 不增加独立 root。
- synthesized Memory 与 CognitiveSchema 共用同一 provenance root traversal；同一 Artifact 的派生表示、同源重述、部分共享根和未知依赖均不能凑成两个独立根。
- 增补/撤回 Schema evidence 改变 object epoch 和 projection invalidation，不伪造 content revision。
- Tag identity 与显示名称分离；不同 Tag identity 可以有相同显示字符串。create 在同一 Authority 事务建立 Directory binding；revise 使用 expected revision fence，追加不可变 TagRevision，保留旧合法 label 为 alias。
- merge 保留指定 survivor，将 retired Tag 标为 merged 并映射到 active canonical Tag；后续 merge 压平 survivor 映射。历史 attachment/AssociationEvidence endpoints 不改写。Directory name/lexical ref、Query cues/preferences/descriptors、Serving projection 和 Neighborhood 读取当前 canonical identity；同一 canonical Tag 的旧名称匹配合为一个候选。
- split 创建 2..8 个新概念及 lineage，保留 parent，历史引用不搬迁；新 scope 的 attachment/revoke 由显式操作或 maintenance proposal 表达。merge/split 均需要 1..16 个去重的 exact supports；一个明确来源即可支持 alias/equivalence 或语义分化，不要求两个独立 provenance roots。验证、lineage、Directory 与 receipt 同事务。模型形成的 create/revise/split 保留实际 ProducerSignature，receipt 回放返回原 exact TagRevision。lineage 保留 parent/child 及其 exact TagRevision、operation、支持、顺序和时间。
- 第一方 ConceptService 提供 revise/merge/split，Tag 输出 current revision、status 和 canonical survivor。Neighborhood 返回当前解释的 endpoints 和原 AssociationEvidence identity/support，原端点仍保存在 Authority evidence。
- Association cognition endpoint 只接受 exact `MemoryRevision` 或 `CognitiveSchemaRevision`；Entity、Tag、Resource 可以是 stable structural endpoint。UseEvent support 必须可由 durable event 或 purge receipt 核验。
- `cognitive_derivation` 与 `derived_structure` 必须携带 producer identity；`result_refuted` 不制造 positive meaningful-use association。
- Topology projection 展开 provenance roots；同 `(from,to,root)` 只保留最高 support quality。contradiction、counterexample、boundary 和 negative association 不进入普通 non-negative adjacency。

## Lifecycle、幂等和 purge

普通 cognition 读取只接受 accepted/valid/normal/not-purging。Suppression、restore、revalidation 和 purge 是独立 lifecycle 操作，不创建虚假 content revision。

来源对象的 revision、lifecycle、support set 或 purge 变化在同一 Authority 事务中沿当前 exact dependency 传播到 Memory、CognitiveSchema、Episode、Journal。当前 dependent 标记为 `revalidation_required`；同一失效传播中，多个路径到达同一对象只增加一次 epoch。Journal 排入 `journal_revalidate`，来源恢复或重建不会将 dependent 自动改回 valid。重新提交支持经过验证的新 revision 后，该对象恢复 valid。

传播记录保留精确 dependent/source revision 和当前失效原因；immutable 正文及支持引用保持原样。Memory 进入 purging 时就标记下游完整性；完成清除后，指向被清 Memory 的 Schema evidence link 撤回。Serving 的相关 family watermark 共用该事务已分配的 authority sequence，传播到 Memory/Schema 时覆盖其 projection family。

Memory、CognitiveSchema、Episode 和 Journal 的 mutation 使用 Persistence `MutationEnvelope`：owner Subject lock 先于 operation lock，receipt 检查 canonical digest，Replay 由 owner 解码。领域验证和 SQL 写入保持在 owner；envelope 合并 owner 选定的 projection families，receipt 与 Authority 写入同事务提交。未完成事务整体回滚。Memory purge 的 purging checkpoint 与最终清除分别提交，resume 校验同一 operation identity/digest。

同一 operation/event identity 携带相同 canonical digest 时返回相同语义结果；相同 identity 携带不同 digest 时返回 conflict。Purge 后保留不含认知正文的幂等 receipt，不能用 receipt 恢复被清内容。

Authority commit 只发布 projection invalidation/watermark；lexical、dense、topology 和 runtime serving 均可重建，不拥有 cognition truth。Memory owner 不持有 concrete Retrieval/Serving；topology candidate generation 属于 Retrieval shared contributor。

[返回文档目录](../../INDEX.md)

## Semantic Concept 与显式 Tag

Tag 是共享 embeddable semantic concept，稳定 identity 与 immutable semantic revisions 分离。Current reads 使用 current canonical Tag；as-of reads 使用截点状态，未来 revise/merge/split 不改写历史意义。normalized label/description/kind_hint 产生 versioned canonical text/digest，别名不进入 semantic representation。

显式 formation Tag 在 Memory commit transaction 中校验 Subject、canonicalize active/merged identity 并去重；不依赖 concept-maintenance model。客户端使用 Concept API (`client.concepts`)。Query inferred Tags/novel hypotheses 是 ephemeral activation，创建、修订、merge、split 与 attachment/Association mutations 仍由 canonical owner API 提交；维护模型活动必须由 Host 有界 grant。
