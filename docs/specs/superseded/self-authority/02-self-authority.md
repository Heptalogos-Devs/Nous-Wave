# 02 — Self Authority

## 1. Self owner

新增：

```text
crates/self-domain
crates/self-service
```

`self-domain`：

- value objects；
- validation；
- state transition rules。

`self-service`：

- Authority persistence；
- idempotent mutation；
- provenance validation；
- query contribution。

Self 不放入 `subject-core`，因为 Subject identity 与 Self cognition 是不同 Authority。

## 2. Self Facet

Facet kind固定：

```text
identity
role
capability
limitation
tendency
value
preference
```

不要增加 `other`.

如果出现新稳定语义类别，后续明确设计再增加。

### object

```text
SelfFacet
- self_facet_id
- subject_id
- kind
- key
- current_revision_id
- object_epoch
- acceptance_state
- integrity_state
- suppression_state
- purge_state
- created_at
```

`kind`、`key`创建后不可修改。

### revision

```text
SelfFacetRevision
- self_facet_revision_id
- self_facet_id
- subject_id
- revision_no
- parent_revision_id?
- revision_intent?
- statement
- scope
- epistemic_class
- valid_time
- formed_at
- recorded_at
- producer_signature_id?
```

statement：

- UTF-8；
- trim后非空；
-最大 64 KiB。

## 3. Self revision intent

```text
correct
refine
reinterpret
evolve
```

### correct

同一 valid interval 的旧认知错误。

### refine

同一 self matter的表达更具体，但没有改为另一件事。

### reinterpret

证据/经历基本不变，但主体对自身解释改变。

### evolve

主体自身在时间上发生真实变化。

`evolve`允许新 revision 使用不同 valid interval。

不把不同 `(kind,key)` 的内容 revision到同一 object。

## 4. supports

每个 Self revision至少 1 个 support。

允许：

- Seed support；
- EvidenceRef；
- exact cognition dependency。

Cognition dependency允许：

- MemoryRevision；
- CognitiveSchemaRevision；
- SelfFacetRevision；
- NarrativeIdentityRevision。

禁止 mutable cognition object ref。

同一个 support canonical key不能重复。

## 5. EpistemicClass

继续使用 shared `EpistemicClass`。

典型：

- Seed direct：Reported；
-经历/观察直接形成：Observed / Reported；
-反思：Inferred；
-Narrative组织：Narrative。

不建立统一 confidence float。

## 6. 创建 Self Facet

输入：

```text
CreateSelfFacet
- operation_id
- subject
- kind
- key
- statement
- scope
- epistemic_class
- valid_time
- formed_at
- supports[]
- producer_signature_id?
```

规则：

- `(subject,kind,key)` 唯一；
- operation id + digest幂等；
- supports全部在 commit前重新验证；
- model/derived proposal若不是 Seed/Host direct，producer signature必填；
-提交成功后 Subject authority_seq +1。

## 7. revision

输入：

```text
ReviseSelfFacet
- operation_id
- subject
- self_facet_id
- expected_object_epoch
- parent_revision_id
- revision_intent
- statement
- scope
- epistemic_class
- valid_time
- formed_at
- supports[]
- producer_signature_id?
```

事务：

1. lock object；
2. object epoch必须等于 expected；
3. parent必须等于 current head；
4. lifecycle必须 accepted + valid + normal + not purging；
5.验证 supports / producer / temporal；
6.插入 immutable revision；
7. current head切换；
8. object_epoch +1；
9. authority_seq +1；
10. commit。

## 8. lifecycle

支持：

```text
withdraw / reaccept
suppress / restore
mark_revalidation_required / restore_valid
purge
```

状态语义与 Memory一致，但 Self没有 Accessibility。

### withdraw

主体不再持有该 self cognition。

### suppress

当前普通认知查询不可返回，但对象仍存在。

### revalidation_required

依赖来源改变，需要重新核验。

### purge

破坏性删除 cognition内容。

## 9. Purge

Self当前没有独立 dense/topology projection时，也必须保留最小 operation receipt。

流程：

1. object lock；
2. set purging；
3.删除 revision statement/support等 cognition-scope内容；
4.删除 runtime resident refs；
5.删除 Serving document；
6.写 purge receipt；
7.删除/墓碑 object。

不要删除原始 Seed artifact、Observation或Memory source。

## 10. Narrative Identity

Narrative是 Self domain 中独立 object family，不塞进 facet kind。

### object

```text
NarrativeIdentity
- narrative_identity_id
- subject_id
- key
- current_revision_id
- object_epoch
- acceptance_state
- integrity_state
- suppression_state
- purge_state
- created_at
```

`key`规则与 facet key相同。

### revision

```text
NarrativeIdentityRevision
- narrative_identity_revision_id
- narrative_identity_id
- revision_no
- parent_revision_id?
- revision_intent?
- text
- valid_time
- formed_at
- recorded_at
- producer_signature_id?
```

另存：

```text
NarrativeReference
- narrative_revision_id
- position
- target_exact_ref
- role
```

v1 target允许：

- MemoryRevision；
- SelfFacetRevision。

Episode以后实现时再增加 EpisodeRevision。

不要为了未来 Episode先加 placeholder type。

## 11. Narrative reference 与 support分离

Narrative reference表示“叙事组织引用”。

它不自动构成该 narrative claim 的 evidence。

如同一个 MemoryRevision也是 evidence，必须同时出现在 supports中。

## 12. Narrative invalidation

被引用 exact revision：

- purge；
-被管理操作撤回且语义不再可用；
-其来源 dependency失效；

Narrative current object进入 `revalidation_required`。

不自动生成新 narrative revision。

## 13. Seed 初始化映射

每个 seed facet：

-不存在 `(kind,key)` → create；
-已经存在且来源就是同一个 seed version / 同内容 → idempotent；
-已经存在但来自旧 Seed /已有主体演化 →形成 proposal，不静默覆盖。

本次同步 API 的默认决定：

**已有 Self object 不自动被新 Seed import覆盖。**

返回：

```text
SeedImportResult
- created[]
- unchanged[]
- conflicts[]
```

conflict需要 Host/后续认知流程显式决定。

这避免“更新人格文件”直接抹掉主体历史。

## 14. producer

Host explicit / Seed direct可无模型 producer。

以下必须 producer：

- model reflection；
- cross-domain synthesized proposal；
-自动 narrative rewrite（未来）。

## 15. 不实现的自动行为

本次没有：

-人格自动学习循环；
-自动 conflict resolution；
-自动 facet merge/split；
-自动 narrative rewrite。

Self owner已经具备这些未来机制需要的正确 Authority结构即可。
