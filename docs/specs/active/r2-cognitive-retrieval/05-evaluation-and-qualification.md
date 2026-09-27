# 05 — R2 Evaluation and Qualification

状态：ACCEPTANCE AUTHORITY
目标路径：`docs/specs/active/r2-cognitive-retrieval/05-evaluation-and-qualification.md`

## 1. Purpose

R2只有在 correctness qualification PASS 后才允许进入 benchmark。

Benchmark不是 implementation selection前置步骤。

## 2. Result vocabulary

每项只能：

- PASS
- FAIL
- NOT_RUN
- BLOCKED

禁止“理论上”“应该”“基本”。

最终报告记录：

- git commit SHA
- toolchain
- PostgreSQL version
- provider/embedding space
-命令
-失败测试名称
- known research limitations

## 3. Static/build gates

至少执行仓库正式命令：

- `corepack pnpm check`
- `just fmt`
- `just check`
- `just lint`
- `just test`
- `just verify`

若当前 AGENTS/manifest规定更新后的正式命令，以仓库当前规定为准。

## 4. Regression suite C0–C14

### Q-01 stale old revision

-建立 Memory revision R1；
- build lexical generation；
- revise到R2；
-不重建 lexical，使用stale generation query；
-普通 recall不得返回R1；
- exact R1 target允许返回历史R1。

### Q-02 arbitrary first-N elimination

创建至少10,000 Memory。

唯一 entity匹配和唯一 temporal匹配放在不会被 `ORDER BY memory_id LIMIT small_budget`采到的位置。

必须仍被对应 lane召回。

### Q-03 Runtime resident isolation

- Session A resident=M1
- Session B resident=M2

Query A：

- M1可以Runtime rank
- M2不能因属于Subject获得Runtime rank。

### Q-04 multi-value include

include roles：

```text
[declarative, experiential]
```

两类对象都可匹配。

formation mode同理。

### Q-05 Lexical provider authority

建立 Tantivy tokenizer可以命中但 literal substring不成立的fixture。

- index hit必须保留。

建立 substring成立但 index不返回的fixture：

-不得产生 lexical rank。

### Q-06 Use duplicate side effect

有 Session的同 UseEvent：

第一次：

- accepted=1
- revision V→V+1

retry：

- accepted=0
- duplicate=1
- revision仍V+1
- last_activity不变化

### Q-07 same-batch coalesce

同 payload event重复2次：

- accepted1
- duplicate1
- runtime side effect一次。

### Q-08 Schema provenance gate

同一 external root的 Support+Boundary：

- synthesized reject。

两个 independent roots：

- synthesized accept。

explicit_import单 support：

- accept。

### Q-09 Memory identity drift

parent aboutness=[Alice]。

revision aboutness=[Bob]：

- reject。

revision aboutness=[Alice,Bob]：

-允许通过 identity guard（其他验证仍需满足）。

### Q-10 exact Association endpoint

MemoryId / SchemaId endpoint：

- invalid。

exact revision：

- valid。

### Q-11 association producer

cognitive_derivation无 producer：

- invalid。

derived_structure无 producer：

- invalid。

### Q-12 association provenance → topology

同 root多 supports：

- graph root mass只计一次最高 quality。

两个 independent roots：

- edge raw support包含两个 root贡献。

### Q-13 contradiction non-propagating

MemoryRevisionRelation::Contradicts：

- Authority relation存在；
- topology adjacency无普通 positive edge。

counterexample/negative association同理。

### Q-14 single fusion owner

构建检查：

- canonical RRF constants只存在一个生产 owner；
- query orchestration不复制 FAMILY_WEIGHTS/RRF公式。

## 5. Existing R1 integrity must continue PASS

不得因R2破坏：

- operation idempotency
- revision fencing
- source-root independence
- provenance cycle
- suppression/restore
- two-phase purge
- dependent revalidation
- serving watermark publication fence
- artifact checksum
- embedding space isolation
- Wave state merge key

## 6. Deterministic evaluation corpus

建立版本化 corpus，不依赖外部网络。

至少场景：

1. correction vs later world-state successor
2. stale projection
3. same-root duplicate summary
4. independent corroboration
5. entity mention vs aboutness
6. runtime resident/nonresident
7. old memory + meaningful-use accessibility
8. Schema support/counterexample
9. contradiction topology
10. purge/rebuild

## 7. Query oracle

每 scenario声明：

```text
MUST_RETURN
MUST_NOT_RETURN
MAY_RETURN
EXPECTED_DIAGNOSTICS
```

Correctness先按oracle判定，不用 ranking metric掩盖非法结果。

`MUST_NOT_RETURN` violation直接 FAIL，即使Recall高。

## 8. Scale fixture

至少：

```text
10,000 current Memory revisions
>= 3,000 historical Memory revisions
30,000 AssociationEvidence
2,000 Entity refs
1,000 Tags
>= 200 CognitiveSchemas
```

必须证明：

- Entity/Temporal/Runtime不使用 application-side全量 scan；
-没有 arbitrary first-N sampling；
-final validator无 N+1 SQL；
-Wave obey max_states/max_neighbors。

## 9. Performance baseline

暂不设生产 SLA。

记录：

- p50/p95 query wall time
- candidate counts/lane
- final validation count
- DB query count（测试/trace可观测）
- peak process memory
- Serving build/rebuild time
- topology nodes/edges/states

任何 query若 candidate generation随着总 Memory数线性 application-side materialization，FAIL架构门槛。

## 10. Benchmark admission gate

只有以下全部 PASS 后才运行：

- regression C0–C14
- R1 integrity
- scale fixture structural gates
- restart/rebuild

## 11. First benchmark

比较：

### Baseline

```text
Exact
+ Entity
+ Lexical
+ Dense
+ Runtime / Temporal when applicable
+ SchemaDirect
→ fixed RRF
→ Final Authority
```

### Wave

```text
Baseline
+ TopologyWave R1
```

固定：

- corpus
- query set
- QueryPlan
- lane budgets
- embedding space
- final validator
- rerank policy

不在比较期间调参让某一路“赢”。

## 12. Metrics

至少：

- Recall@K
- MRR
- nDCG（只有 graded relevance时）
- MUST_NOT_RETURN violation count
- stale/lifecycle violation count
- provenance correctness
- p50/p95
- candidates processed
- Wave discarded state mass
- peak memory

## 13. External benchmark usage

LongMemEval / LongMemEval-V2、Memora等只选与当前目标对应的场景映射：

- dynamic state
- premise awareness
- obsolete knowledge
- workflow/procedural experience

不声称这些 benchmark完整代表 Nous Wave。

外部 benchmark失败不能通过放宽 Authority/lifecycle规则“修分”。

## 14. Restart / rebuild

### restart

- Memory Authority恢复；
- UseEvent idempotency恢复；
- ResidentSet恢复；
- BoundQuery不跨进程持久化。

### rebuild

删除 serving artifact：

-从 Authority重建新 generation；
- semantic result identity一致；
-generation id允许不同。

### watermark race

build snapshot W；
期间 Authority变成D>W：

-W generation不能publish current；
-后续W2>=D才能publish。

## 15. Qualification report

落库建议：

`docs/qualification/2026-10-r2-cognitive-retrieval.md`

内容：

- Spec commit SHA
- implementation commit SHA
-全部 gates
- benchmark admission
- benchmark results
- known limitations
-未实现 stretch

## 16. Completion gate

全部YES：

- [ ] BoundQuery
- [ ] typed lane providers
- [ ] no arbitrary first-N
- [ ] exact current/historical policy
- [ ] hard constraints
- [ ] single fusion owner
- [ ] batch Final Authority Validator
- [ ] stale old revision blocked
- [ ] Schema independent-root gate
- [ ] exact Association endpoints
- [ ] producer/use support contract
- [ ] provenance roots feed topology
- [ ] contradiction non-propagating
- [ ] typed Wave seeds
- [ ] UseEvent zero-side-effect retry
- [ ] deterministic corpus
- [ ] scale fixture
- [ ] full verification PASS

## 17. Unresolved decisions

None.
