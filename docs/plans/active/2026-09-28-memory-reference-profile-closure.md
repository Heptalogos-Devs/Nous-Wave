# Memory Reference Profile Closure R2

状态：ACTIVE IMPLEMENTATION AUTHORIZATION
日期：2026-09-28

本计划是当前唯一新增代码施工授权。它执行外部 `Nous-Wave-Memory-Reference-Closure-Spec-R2` 包，并服从：

1. scoped `AGENTS.md`；
2. Architecture-Vault Nous Wave `TARGET_DESIGN.md`、`DECISIONS.md` 和相关跨系统合同；
3. [Target Engineering Plan](roadmap/2026-09-target-engineering-plan.md)；
4. [2026-10-27 Memory Reference Profile milestone](2026-10-27-memory-reference-profile.md)；
5. [Cognitive Retrieval executable Specs](../../specs/active/cognitive-retrieval/README.md)。

## 当前施工主线

本轮只闭合 Memory Reference Profile 的 Memory-only、multi-session、可恢复、可追溯、可查询、可使用和可清除纵向链：

```text
Artifact → Observation → Evidence/Provenance → Memory Authority
→ Revision/Temporal semantics → Serving/Query → Runtime Use
→ lifecycle → restart/rebuild → trace-back
```

执行顺序固定为：

1. 文档授权修正与 Architecture-Vault 长期设计恢复；
2. Runtime ResidentSet/Observation、Configuration receipt、Social provenance、EPA 的 P0 correctness；
3. Memory Reference Profile deterministic semantic scenario；
4. 多 Session / consumer Runtime closure；
5. retrieval/Serving targeted regression、corpus/oracle、restart/rebuild、scale 与 baseline admission；
6. Qualification 与 current-state 更新。

## 范围边界

Episode、Journal、Dream/Offline Cognition、Self/Social 新语义、Motivation、Heptalogos live integration、UI 和新图算法产品化均不在本轮范围。已有 Self/Social 实现保留为 frozen early slices；只有 R2 明确点名的 correctness repair 可以施工。

Wave 继续作为已存在的 reference topology lane；benchmark 只有在 correctness admission gate 全部 PASS 后才能运行。EPA 修复只证明 weighted PCA 数学转写，不把 EPA 或 Wave 晋升为生产默认。

## 直接执行合同

- 外部 R2 Spec 包的 Markdown 是本轮直接实施规范；其 manifest/解压记录与 Qualification 一并保存。
- 不建立无真实兼容义务的旧 ontology、双读、双写、fallback 或永久 adapter。
- 每个 semantic slice 按 `domain → Authority persistence → protocol → service → client → query/serving → tests` 维度闭合；没有实际运行的验证只记 `NOT_RUN` 或 `BLOCKED`。
- 发现改变 Authority、identity/revision、lifecycle、transaction、concurrency、idempotency、protocol、recovery、purge、query ranking、Serving consistency、security 或 algorithm default 的未决语义时，停止该局部并记录 `SPEC_GAP`；与更高 Authority 冲突时记录 `SPEC_CONFLICT`。

## 验收入口

- [Cognitive Retrieval Qualification](../../qualification/2026-10-cognitive-retrieval.md)
- 本轮新增 [Memory Reference Profile Closure Qualification](../../qualification/2026-10-memory-reference-profile.md)
- [Current State](../../current-state/CURRENT_STATE.md)
