# R2 Cognitive Retrieval & Evaluation — Executable Specs

状态：IMPLEMENTATION-AUTHORIZING
目标目录：`docs/specs/active/r2-cognitive-retrieval/`

## 1. 必须完整阅读

实施前阅读：

- repository root `AGENTS.md`
- `docs/AGENTS.md`
- Architecture-Vault Nous Wave `TARGET_DESIGN.md`
- Architecture-Vault Nous Wave `DECISIONS.md`
- `docs/plans/roadmap/2026-09-target-engineering-plan.md`
- `docs/plans/active/2026-10-27-memory-reference-profile.md`
- `docs/plans/active/2026-09-27-cognitive-retrieval-evaluation-r2.md`
- `docs/current-state/audits/2026-09-27-r1-reference-profile-review.md`
- 本目录全部 Spec

## 2. Spec 顺序

1. `01-bound-query-and-lanes.md`
2. `02-fusion-and-final-authority.md`
3. `03-schema-association-topology.md`
4. `04-runtime-use-idempotency.md`
5. `05-evaluation-and-qualification.md`
6. `06-implementation-map.md`

## 3. 执行原则

- 按 semantic vertical slice 施工，不按 crate 分阶段“做完”。
- 不保留 R1 错误行为作为 fallback。
- 不创建双读/双写兼容层。
- 不在 correctness 未闭合前引入更多算法候选。
- Wave 继续作为已选 reference topology lane。
- 每个 slice 完成后运行最窄有意义验证。
-最终以 Spec 05 的 Qualification 为 acceptance authority。

## 4. Error labels

若更高 Authority 与本 Spec 冲突：`SPEC_CONFLICT`。
若 Spec 真的缺失一个会改变公开语义的决定：`SPEC_GAP`。

普通实现细节不允许滥用 `SPEC_GAP` 推回决策责任。

## 5. Unresolved decisions

None.
