# Cognitive Retrieval & Evaluation Substrate

日期：2026-09-27
状态：ACTIVE IMPLEMENTATION PLAN
目标落库路径：`docs/plans/active/2026-09-27-cognitive-retrieval-evaluation.md`
父计划：`docs/plans/roadmap/2026-09-target-engineering-plan.md`
中期里程碑：`docs/plans/active/2026-10-27-memory-reference-profile.md`
审查输入：`docs/current-state/audits/2026-09-27-reference-profile-review.md`

## 1. 定位

本计划是 Memory Reference Profile 从“Authority / Provenance 纵向闭环”向“稳定 Cognitive Query / Serving / Evaluation substrate”深入的施工计划。

本计划不是：

- 单独的 bugfix sprint；
- 新的目标架构；
- 为 10 月 27 日另造的 MVP；
- 在开发前比较多种算法的 benchmark 阶段。

R1 审查中 C0–C14 是本阶段强制 carry-over invariants，与新能力同时完成。

## 2. Authority 链

```text
Architecture-Vault Target Design / Decisions
    ↓
Target Engineering Plan
    ↓
2026-10-27 Memory Reference Profile milestone
    ↓
R1 Implementation Review
    ↓
当前 Cognitive Retrieval Plan
    ↓
Cognitive Retrieval Executable Specs
    ↓
Code
    ↓
Qualification / Evaluation
```

如果 Cognitive Retrieval Spec 与 Architecture-Vault 更高层语义冲突，停止相关施工并报告 `SPEC_CONFLICT`；不得自行折中。

## 3. 核心产物

### 3.1 Immutable BoundQuery

Query 在 candidate generation 前完成 binding，冻结：

- Subject；
- exact object → exact revision；
- current / explicit-historical semantics；
- enabled lanes；
- lane budgets；
- hard constraints；
- accessibility policy；
- embedding space；
- topology requirement；
- fusion version；
- rerank policy。

### 3.2 Typed lane providers

建立：

- ExactLane
- RuntimeLane
- EntityLane
- LexicalLane
- DenseLane
- TemporalLane
- SchemaDirectLane
- TopologyWaveLane

删除“任意 first-N Memory 再过滤”的通用 candidate path。

### 3.3 Canonical fusion owner

一个纯函数 owner 完成：

- lane 内 revision-level multiview aggregation；
- fixed-plan RRF；
- deterministic tie break；
- optional second-stage rerank。

### 3.4 Batch Final Authority Validator

批量核验：

- exact revision；
- current-head requirement；
- lifecycle；
- accessibility；
- role/mode；
- aboutness/entity；
- temporal；
- source/provenance facet；
- authority/modality/epistemic constraint。

不允许 N+1 per-candidate SQL。

### 3.5 Schema / Association / Topology closure

完成：

- explicit-import vs synthesized Schema formation；
- independent provenance root gate；
- exact Association cognition endpoint；
- producer/use-event source contract；
- Association provenance → Topology；
- contradiction/counterexample/negative non-propagating；
- typed Wave seed weights。

### 3.6 Runtime idempotency closure

完成：

- same-batch duplicate coalescing；
- duplicate-only request zero runtime side effect；
- newly accepted events 才能改变 ResidentSet / Session revision；
- RuntimeLane真实消费 ResidentSet/Situation refs。

### 3.7 Evaluation substrate

correctness 全部 PASS 后才进行 baseline/Wave benchmark。

## 4. 明确不进入

本阶段不实现：

-完整 Self Authority；
- Social Cognition；
- Motivation / Desired Condition；
-完整 WorkContext scheduler；
- Offline Cognition；
- Heptalogos live connector；
- PPR 等替代图算法的正式实现（除非 correctness 完成后作为独立 research experiment）。

## 5. 阶段顺序

### Phase A — Query substrate

- BoundQuery；
- typed lanes；
- hard constraints；
- current/historical semantics；
- canonical lane outputs。

### Phase B — Fusion + Final Authority

-唯一 RRF owner；
- final batch validator；
- stale projection semantics；
- diagnostics。

### Phase C — Schema / Association / Topology

- provenance gate；
- exact endpoints；
- producer/use-event contract；
- topology provenance feed；
- Wave seed / edge semantics。

### Phase D — Runtime

- UseEvent side-effect idempotency；
- RuntimeLane integration；
- accessibility regression。

### Phase E — Qualification / Evaluation

- deterministic corpus；
- 10k Memory scale fixture；
- regression suite；
- restart/rebuild；
- baseline quality report；
- baseline vs Wave。

## 6. 10 月 27 日前的时间窗口

### 9/27–10/3
完成 Phase A 与关键 C0–C7 regression tests。

### 10/4–10/10
完成 Phase B/C，统一 fusion owner并闭合 topology provenance。

### 10/11–10/16
完成 Phase D 与 deterministic/scale qualification。

### 10/17–10/22
只有 correctness PASS 后运行 baseline/Wave benchmark。

### 10/23–10/27
Qualification Report、Research Report、demo flow、阶段材料。

## 7. 完成门槛

以下全部满足：

- stale generation不能把历史 revision当普通 current hit；
- Entity/Runtime/Temporal 使用真实 lane provider；
- hard constraints无 silent ignore；
- Lexical rank只来自 lexical provider；
- UseEvent duplicate retry无 Runtime side effect；
- Schema synthesized gate基于 independent provenance；
- Memory revision有保守 identity drift guard；
- Association cognition endpoint exact-revision only；
- Association provenance进入 topology；
- contradiction/counterexample/negative不进入普通 Wave；
- canonical fusion owner只有一个；
- scale fixture无 arbitrary first-N recall；
-完整 qualification + restart + rebuild PASS；
-仓库完整验证 PASS。

## 8. Benchmark 政策

先实现已确定的 reference/default 路径，再 benchmark。

第一轮只比较：

1. baseline：Exact + Entity + Lexical + Dense + Runtime/Temporal；
2. baseline + Wave R1。

后续 simple expansion / PPR 等只有在系统稳定后才作为 research experiment加入。

Benchmark 不能成为 correctness code path 的前置条件。
