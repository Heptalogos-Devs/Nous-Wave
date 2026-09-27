# Memory Reference Profile 实施核查

日期：2026-09-27
状态：CURRENT-STATE AUDIT
核查提交：`7b3e2251555c47fca51deaa8c5a25f2f30f8636f`
目标落库路径：`docs/current-state/audits/2026-09-27-reference-profile-review.md`

## 1. 职责

本文记录对已提交 Memory Reference Profile 实现的源码核查结果。

本文不是目标设计 Authority，也不是新的实施规范。长期语义仍由 Architecture-Vault 中 Nous Wave `TARGET_DESIGN.md`、`DECISIONS.md` 和相关 rationale 管理；本审查确认的缺口由 Cognitive Retrieval Active Plan 与 Executable Specs 授权修正。

## 2. 总体结论

Memory Reference Profile 已完成真实的纵向 semantic rebase，可以作为继续开发的代码基线。已经可靠落地的部分包括：

- Memory Object / immutable Revision / `object_epoch`；
- object-level `CognitiveRole` 与 revision-level `FormationMode`；
- acceptance / integrity / suppression / purge / accessibility 分离；
- Occurrence-bound Evidence；
- exact-revision CognitionDependency；
- provenance DAG / source-root dependency relation；
- caller-scoped UseEvent identity；
- power-law accessibility reference implementation；
- Serving generation / authority watermark；
- two-phase Memory purge；
- Wave propagation state identity 修正。

Memory Reference Profile 尚不能被视为 Cognitive Query / Serving correctness 已完全闭合。剩余问题集中于 Query candidate generation、final Authority validation、UseEvent Runtime side effect、CognitiveSchema formation gate、AssociationEvidence→Topology provenance feed，以及 Memory identity guard。

这些问题不单独建立修复阶段，统一作为 Cognitive Retrieval 的 carry-over invariants。

## 3. 确认的 carry-over

### C0 — stale Serving 可把历史 revision 当普通 current hit 返回

普通 Serving candidate 可以携带历史 `MemoryRevisionId`。当前 final validation 验证 lifecycle，却没有要求普通 recall 的 candidate revision 仍等于 `MemoryObject.current_revision_id`。

**Cognitive Retrieval 必须：**

- 普通 recall 只允许 current revision；
- explicit exact historical revision lookup 才允许历史 revision；
- final validator 批量核对 current head；
- 不允许 stale projection 通过“revision 仍存在”这一点绕过 current semantics。

### C1 — Entity / Runtime / Temporal lane 使用 arbitrary first-N Memory sample

当前 Memory Query 会先取一批 `ORDER BY memory_id LIMIT candidate_limit` 的对象，再做 entity/runtime/temporal 判断，因此真正匹配对象可能因不在 first-N 中而漏召回。

**Cognitive Retrieval 必须：** 删除通用 first-N candidate source；每个 lane 从正确的数据结构生成 bounded exact-revision candidates。

### C2 — Runtime lane 把 Subject membership 当 Runtime residency

带 `session_id` 时，当前 Memory Query 可能仅凭 reference 属于 Subject 就给予 Runtime rank，没有验证 ResidentSet / Situation current refs。

**Cognitive Retrieval 必须：** RuntimeLane 只能由 typed runtime refs 产生 candidate。

### C3 — hard constraints 语义错误或未执行

已确认：

- `cognitive_roles_include` / `formation_modes_include` 使用 `all(current == value)`，多值 include 错误；
- source class / authority / modality / evidence class 等并未完整执行。

**Cognitive Retrieval 必须：** 所有 hard constraint 要么得到明确语义并执行，要么 query binding 阶段明确拒绝，禁止 silent ignore。

### C4 — Lexical lane 二次 literal substring 会丢真 hit / 制造假 rank

Tantivy 已经返回 rank，当前代码又通过 `representation_text.contains(cue)` 决定 lane membership，并在没有 index rank 时可能 fallback rank 1。

**Cognitive Retrieval 必须：** Lexical family membership 只能来自 Lexical provider hit。

### C5 — qualification 对 Query correctness 覆盖不足

当前测试没有覆盖 multi-value include、runtime resident isolation、stale old revision、arbitrary first-N miss、lexical false rank、session duplicate side effect。

**Cognitive Retrieval 必须：** 全部转成永久 regression scenarios。

### C6 — UseEvent row 幂等，但 Runtime side effect 不完全幂等

同一已提交 UseEvent 重试不会重复 INSERT，但只要带 `session_id`，当前逻辑仍会更新 `last_activity_at` 并令 `runtime_revision += 1`。

同一 batch 内同 id + 同 digest 当前也返回 conflict，而非 duplicate/coalesce。

**Cognitive Retrieval 必须：**

- side effect 仅由 newly accepted events 触发；
- duplicate-only request 不改变 Session；
- same-batch same-id/same-digest coalesce；
- same-id/different-digest whole-batch conflict。

### C7 — Memory revision 缺少明显 cognitive-matter drift guard

Revision path 未比较 parent / new aboutness，因此 about Alice 的对象可以被 revision 成只 about Bob 并保留原 MemoryId。

**Cognitive Retrieval 必须：** parent aboutness 非空时，新 revision 至少共享一个 aboutness entity；完全不相交要求创建新对象。

### C8 — CognitiveSchema synthesized formation gate 未验证 independent provenance

当前只要求多条 evidence link，不要求独立 provenance roots；同一 Observation 的两个 link 可以满足门槛。

**Cognitive Retrieval 必须：** synthesized Schema 至少两个 normalized inputs 且至少两个 known independent provenance roots；UnknownDependency 不增加独立计数。

### C9 — AssociationEvidence cognition endpoint 未要求 exact revision

当前 mutable `MemoryId` / `CognitiveSchemaId` 可以成为长期 Association endpoint。

**Cognitive Retrieval 必须：** Cognition endpoint 只接受 `MemoryRevisionId` / `CognitiveSchemaRevisionId`；Entity / Tag / Resource 可保持稳定 object ref。

### C10 — Association producer/source contract 未完整实现

Domain/SQL 存在 producer 字段，但 create path 没有完整传入和验证。

**Cognitive Retrieval 必须：** derived structure / cognitive derivation 绑定 producer identity；meaningful-use association 绑定真实 UseEvent。

### C11 — Association provenance roots 没有进入 Topology builder

Graph builder具有 per-root dedup 能力，但 TopologyProjectionInput 没展开 Association supports；Serving build 最终以 `provenance_root=None` 构图。

**Cognitive Retrieval 必须：** topology projection 按 provenance root 输出 edge-evidence contributions。

### C12 — contradiction 被错误当作普通正向 Wave edge

Memory relation `contradicts` 当前被投影为 positive structural edge，且 Graph 允许该 relation。

**Cognitive Retrieval 必须：** contradiction / counterexample / negative evidence 不进入普通 non-negative Wave adjacency。

### C13 — Wave seed policy未落实 reference weights

当前多种 seed 实际统一 weight=1.0。

**Cognitive Retrieval 必须：** 实现已定义 typed seed weights，并在 diagnostics 中暴露 seed composition。

### C14 — 存在两个可能漂移的 fusion owner

`cognitive-retrieval/ranking.rs` 与 `memory-service/query.rs` 都维护融合逻辑。

**Cognitive Retrieval 必须：** 只保留一个 canonical fusion owner。

## 4. 验证状态说明

提交中已有本地验证记录，但该 commit 没有可用 GitHub workflow run 可供独立复核。本审查依据提交源码、测试与文档语义核对，不将“已有测试 PASS”视为对上述未覆盖行为的证明。

## 5. 结论

Memory Reference Profile 基线保留并继续向前演化。Cognitive Retrieval 不以“回修旧阶段”为目标，而是建立稳定的 Cognitive Retrieval / Serving / Evaluation substrate，并把 C0–C14 作为新的永久 regression contract。
