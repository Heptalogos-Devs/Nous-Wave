# 04 — Runtime Use Idempotency and Accessibility Contract

状态：IMPLEMENTATION-AUTHORIZING
目标路径：`docs/specs/active/r2-cognitive-retrieval/04-runtime-use-idempotency.md`

## 1. Scope

冻结：

- ReportUse batch dedup；
- durable UseEvent identity；
- duplicate/conflict；
- Session side effects；
- ResidentSet admission；
- RuntimeLane数据来源；
- accessibility reference parameters；
- purge interaction。

## 2. Event identity

唯一键：

```text
(subject_id, consumer_ref, event_id)
```

`event_id` caller-supplied UUIDv7。

canonical digest包含：

- subject
- consumer_ref
- event_id
- session_id
- exact cognition revision
- use_kind
- occurred_at
- canonical context

same key + same digest = duplicate。
same key + different digest = conflict。

## 3. Allowed refs

UseEvent target只允许：

- MemoryRevision
- CognitiveSchemaRevision

不接受：

- Memory
- CognitiveSchema

## 4. Batch preprocessing

请求1..256 events。

事务前/事务内准备步骤必须先对 input batch按 key分组。

### 4.1 same batch same key/same digest

coalesce：

-第一条成为 canonical item；
-其余 input entries计入 `duplicate_count`；
-不 conflict。

### 4.2 same batch same key/different digest

整批 `CONFLICT`。

### 4.3 stored duplicate

DB已有 event或purged receipt，digest相同：

- canonical item标 duplicate；
-不再验证 target ref存在；
-不产生任何 runtime side effect。

digest不同：

-整批 `CONFLICT`。

## 5. Transaction atomicity

一个 ReportUse batch：

1. lock `(subject, consumer_ref)`；
2.验证 Session；
3. preprocess batch duplicates；
4.读取 stored events/receipts；
5.检测全部 conflicts；
6.验证所有 new event exact refs；
7. INSERT全部 new events；
8.对 **new events only** 应用 Session/Resident side effect；
9. commit。

任何 conflict ->整批无新增 row、无 side effect。

## 6. accepted / duplicate counters

按原始 input entries计数。

例：

同 batch两个完全相同的新 event：

```text
accepted_count = 1
duplicate_count = 1
```

若该 event在DB已有：

```text
accepted_count = 0
duplicate_count = 2
```

## 7. Session side-effect rule

只有 `accepted_count > 0` 时，Session可以变化。

duplicate-only request：

- `last_activity_at` 不变；
- `last_meaningful_use_at` 不变；
- resident refs不变；
- `runtime_revision` 不变；
- response返回当前 runtime_revision。

## 8. new non-meaningful event

`presented`：

-写 durable event；
-更新 Session `last_activity_at = recorded_at`;
- `runtime_revision += 1`;
-不 admit resident；
-不更新 last_meaningful_use_at。

## 9. new meaningful event

meaningful：

```text
referenced
acted_on
result_supported
result_refuted
corrected
pinned
```

效果：

-写 event；
- exact revision进入/刷新 ResidentSet；
- last_meaningful_use_at=max(old,event.occurred_at)；
- Session last_activity_at=recorded_at；
- Session last_meaningful_use_at=max；
-整个 batch只 `runtime_revision += 1` 一次。

## 10. ResidentSet

key：

```text
(session_id, ref_kind, ref_value)
```

只保存 exact revisions。

meaningful new use：

-不存在 -> insert resident；
-存在 -> state=resident；
-更新 last_meaningful_use_at。

`presented`不 admit。

## 11. Closed Session

带 closed/foreign session：

`FAILED_PRECONDITION`

不允许为了接受 late use自动 reopen。

异步 outcome可用 `session_id=None`，通过 consumer_ref/context保留业务关联。

## 12. Accessibility reference implementation

R2保持现有 R1 reference algorithm，不在 correctness阶段改参。

参数：

```text
epsilon = 0.02
tau_days = 30.0
decay = 0.5
formation_weight = 1.0
```

contribution：

```text
contribution(w, delta_days)
= w * (1 + max(delta_days,0) / tau_days)^(-decay)
```

mass：

```text
mass
= epsilon
+ formation_weight * decay(age_since_memory_creation)
+ Σ typed_use_weight * decay(age_since_use)
```

activation：

```text
A = ln(mass)
```

levels：

```text
A >= -0.80             -> Normal
-1.60 <= A < -0.80     -> Deep
A < -1.60              -> Explicit
```

typed weights：

| use | weight |
|---|---:|
| presented | 0.0 |
| referenced | 1.0 |
| acted_on | 1.4 |
| result_supported | 1.1 |
| result_refuted | 1.2 |
| corrected | 1.5 |
| pinned | 2.0 |

这些是 R2 reference参数。benchmark以后可以通过新的 Decision/Spec revision调整；当前开发不等待 benchmark。

## 13. Accessibility override

`AccessibilityMode`：

- auto
- normal
- deep
- explicit

override优先于 auto activation。

Eligibility：

| level | Light | Normal | Deep | Maximum | Exact |
|---|---:|---:|---:|---:|---:|
| Normal | yes | yes | yes | yes | yes |
| Deep | no | no | yes | yes | yes |
| Explicit | no | no | no | no | yes |

Exact只能绕 accessibility，不能绕 purge/lifecycle。

## 14. RuntimeLane contract

Spec 01 RuntimeLane只消费：

- Situation current refs
- ResidentSet

ReportUse是 resident state的主要更新路径之一。

不得使用：

`reference_in_subject()` 代替 residency。

## 15. Purge

Memory/Schema purge：

-移除 ResidentSet refs；
- active use rows中 target identity按既定 purge contract删除/摘要化；
- `purged_use_receipts`保留 idempotency digest；
-同 event retry仍 duplicate success；
-不能因为 ref已purge变成 NotFound conflict。

## 16. context hygiene

UseEvent context：

-最大 16 KiB；
-只保存 metadata；
-不得保存 Memory正文、prompt全文、raw artifact。

建议 validation拒绝明显 payload key：

```text
raw
prompt
content
memory_text
artifact_bytes
```

如果已有合法业务字段同名，需要显式结构化 namespace，而不是放原文。

## 17. Proto/API response

ReportUse response必须包含：

```text
accepted_count
duplicate_count
session_runtime_revision?
```

duplicate-only session request返回未变化的当前 revision。

## 18. Required tests

1. stored duplicate + session -> runtime_revision不变。
2. same-batch two same new events -> accepted1 duplicate1。
3. same-batch same id/different digest -> whole batch conflict。
4. mixed new + stored duplicate ->只 new触发 side effect。
5. batch两个 meaningful new events -> runtime_revision只+1。
6. new presented -> revision+1但不 resident。
7. new referenced -> resident。
8. closed session -> failed precondition。
9. purge后 retry same event -> duplicate，不 NotFound。
10. MemoryId/SchemaId target -> invalid。
11. reference_in_subject但非resident对象不进入 RuntimeLane。
12. accessibility固定数值 fixture覆盖所有 weights/thresholds。

## 19. Rejected alternatives

- duplicate row幂等但允许 Session继续变化：拒绝。
-同 batch identical duplicate当 conflict：拒绝。
- retrieval hit自动写 use：拒绝。
- presented强化 accessibility：拒绝。
- current MemoryId作为 use target：拒绝。

## 20. Unresolved decisions

None.
