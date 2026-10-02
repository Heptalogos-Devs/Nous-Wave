# Memory Authority & Provenance

## Owner

Memory (crates/memory) owns Memory/CognitiveSchema/Tag/AssociationEvidence Authority, provenance and lifecycle. Subject and Material provide the referenced identities and source records.

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

- CognitiveSchema 属于 Memory owner，具有独立 object/revision、applicability、support、counterexample/boundary evidence 和 lifecycle。
- `explicit_import` 至少有一条有效 evidence；`synthesized` 需要至少两个 normalized inputs、至少两个 known independent provenance roots 和无 cycle；UnknownDependency 不增加独立 root。
- synthesized Memory 与 CognitiveSchema 共用同一 provenance root traversal；同一 Artifact 的派生表示、同源重述、部分共享根和未知依赖均不能凑成两个独立根。
- 增补/撤回 Schema evidence 改变 object epoch 和 projection invalidation，不伪造 content revision。
- Tag identity 与显示名称分离；不同 Tag identity 可以有相同显示字符串。
- Association cognition endpoint 只接受 exact `MemoryRevision` 或 `CognitiveSchemaRevision`；Entity、Tag、Resource 可以是 stable structural endpoint。UseEvent support 必须可由 durable event 或 purge receipt 核验。
- `cognitive_derivation` 与 `derived_structure` 必须携带 producer identity；`result_refuted` 不制造 positive meaningful-use association。
- Topology projection 展开 provenance roots；同 `(from,to,root)` 只保留最高 support quality。contradiction、counterexample、boundary 和 negative association 不进入普通 non-negative adjacency。

## Lifecycle、幂等和 purge

普通 cognition 读取只接受 accepted/valid/normal/not-purging。Suppression、restore、revalidation 和 purge 是独立 lifecycle 操作，不创建虚假 content revision。

同一 operation/event identity 携带相同 canonical digest 时返回相同语义结果；相同 identity 携带不同 digest 时返回 conflict。Purge 后保留不含认知正文的幂等 receipt，不能用 receipt 恢复被清内容。

Authority commit 只发布 projection invalidation/watermark；lexical、dense、topology 和 runtime serving 均可重建，不拥有 cognition truth。Memory owner 不持有 concrete Retrieval/Serving；topology candidate generation 属于 Retrieval shared contributor。

[返回文档目录](../../INDEX.md)
