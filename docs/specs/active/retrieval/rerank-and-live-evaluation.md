# Query Rerank and Local Mechanisms

## Principle

### 2026-09-30 用户批准的QueryExpr合同

atom为leaf candidate set；all为canonical identity交集；any为并集。父hard constraints被继承，子constraint只refine自身subtree，不能静默提升。整棵树固定一个QueryPlan与总work budget；subtree allocation在candidate generation前确定。

effort/limit/diagnostics/explore/materialize只允许root，nested直接reject。entity/tag/schema/resource/external-object是typed semantic cues；只有@ref是exact cognition identity read。unknown modifier语义明确reject，不忽略。

软偏好是bounded signed低权重ranking contribution，不生成candidate、不绕过hard constraints、不按观察到的candidate临时调权。初始系数/工作预算/贡献bound在正式run前冻结到当前typed QueryPlan并记录snapshot；正式测量固定参数。

删除裸+recent/-recent。语法为+recent(axis)/-recent(axis)，axis只允许occurred/observed/valid/formed/recorded。未知该轴时间贡献neutral，不替换成其他轴。

外部rerank后再沿原BoundQuery批量revalidate exact revision/head/epoch/lifecycle/source/hard constraints；不能rebind mutable target。变化候选drop，diagnostic为authority_changed_during_rerank，必要时partial；最后才user limit。公开baseline/preference/rerank/final score、rank和structured机制/validation诊断。

Candidate generation、baseline fusion、rerank是不同局部机制。

允许：

```text
lexical
dense
entity
temporal
runtime
topology / Wave
other bounded future mechanisms
```

它们贡献signal，再由稳定QueryPlan组织。模型reranker不替代基础检索，Wave也不替代整个retrieval engine。

## Rerank execution order

正式顺序：

```text
candidate lanes
→ RRF baseline fusion
→ bounded Authority validation/materialization
→ optional rerank of valid textual pool
→ final user limit
```

原因：

- stale/purged/invalid candidate先被Authority丢弃；
- 外部模型只花额度处理真实可返回候选；
- baseline始终存在，可用于fallback和实验；
- model rerank不能让invalid cognition复活。

## Internal validated pool vs public limit

当前Kernel在validation后直接truncate到public user limit，这会让Core没有足够candidate做rerank。

增加 Core→Kernel internal execution option，例如：

```text
validated_candidate_limit
```

语义：

- NousQL `$limit` / public Query limit仍是最终用户limit；
- model rerank启用时，Core请求更大的bounded validated pool；
- pool size来自role/query policy，例如32或64；
- Kernel自身validation budget仍有效；
- non-rerank path保持当前behavior；
- 不通过篡改用户NousQL `$limit`表达内部mechanism。

该字段如果只服务Core orchestration，可以留在kernel internal protocol，不必扩大public CognitionService surface。

## Rerank mechanism seam

Core内部定义小而稳定的interface：

```text
RerankMechanism
  id
  readiness
  rerank(queryIntent, candidates, signal)
```

当前实现：

```text
none
model
```

未来 deterministic reranker可以接同一seam，但本轮不做dynamic plugin manager。

## Model rerank request

输入：

```text
query text
candidate documents[]
top_n
```

每个document必须保留candidate canonical ref映射。

Response validation：

- index必须在range；
- index不得重复；
- relevance score finite；
- malformed response → provider degradation；
- provider遗漏的candidate按baseline relative order放在返回reranked candidates之后；
- exact/hard semantic requirement不被模型否定。

## Rerank query text

从compiled/bound QueryExpr确定性抽取positive textual intent：

- text cues；
- concept cues；
- AND / OR structure。

不要把：

- `$memory`；
- `$limit`；
- `$diagnostics`；
- UUID；
- internal ref；

作为自然语言检索词。

可以输出稳定形式：

```text
ALL OF: ...
ANY OF: ...
```

不要再调用一个LLM改写query。

以下默认跳过model rerank并写diagnostics：

- exact-only；
- entity-only；
- temporal-only；
- purely negative cues；
- 没有textual candidate；
- 无法形成稳定textual intent。

## Requirement behavior

RoleBinding requirement：

- `required`：reranker不可用时该query不能宣称完整完成，按current error/status model返回明确失败或Partial；
- `preferred`：回到baseline并加degradation；
- `optional`：baseline正常工作，diagnostics标unavailable；
- role未配置：完全不执行rerank。

本轮default建议 `optional`，真实实验之后再决定。

## Query score evidence

当前Runtime已有baseline/final score内部结构，但public Hit没有完整暴露。

本轮让CLI/research可可靠读取：

```text
baseline_score
rerank_score?
final_score
final_rank
```

可以作为Hit optional fields，也可以是typed structured diagnostics；选择对当前Proto更整洁的一种，不能只塞不可解析string日志。

## Rerank provenance

Query diagnostics至少记录：

- mechanism id；
- protocol；
- model identifier；
- model profile digest；
- candidate count；
- returned count；
- latency；
- usage（provider有时）；
- failure/degradation code。

token绝不记录。

## No durable learning from rank score

rerank score：

- query-ephemeral；
- 不写Memory；
- 不写Accessibility；
- 不直接创建AssociationEvidence；
- 不作为Wave edge weight；
- 不自动改变RRF weight。

真实使用反馈仍由UseEvent表达。

## Wave experiment semantics

真实研究variants至少尝试：

```text
baseline
baseline + model_rerank
baseline + Wave
baseline + Wave + model_rerank
```

如果真实corpus没有Tag/Schema/Entity/AssociationEvidence/meaningful-use等能产生topology signal的数据：

- Wave variant标 `NOT_RUN` 或 `NO_SIGNAL`（记录中仍用允许的result vocabulary时用NOT_RUN + reason）；
- 不建立答案泄漏edge；
- 不把与baseline相同的结果包装成Wave实验。

如果需要真实edge，依据source中真实关系或正常产品操作建立，并记录来源。


# Real Corpus and Evaluation

## Corpus facts must be real

Coding Agent使用网络搜索和下载公开资料。

禁止：

- LLM虚构文章/访谈/日志；
- LLM批量生成假聊天作为“real corpus”；
- fixture预填candidate order；
- fixture预填latency、precision、work units；
- 为Wave直接把query target连成答案edge；
- 只用原句复制query然后宣称semantic recall。

## First live corpus target

按内容质量和API额度调整，大致：

```text
5–12 independent public sources
100–500 semantic textual observation units
5–10 real images
2–5 public audio clips
2–5 public short videos
30–80 grounded queries
```

不为了数字机械切分。5篇信息密度高的长资料可能比500条假记录更有价值。

## Source selection

文本至少跨多个主题，制造真实distractor。

可选：

- 个人技术博客；
- 开源项目设计/发布文章；
- changelog；
- GitHub issue/discussion；
- 官方技术文章；
- 公开访谈/演讲文字稿。

多媒体优先有明确来源页和license的公开资料，例如 Wikimedia Commons / 项目官方示例。

不要把Nous Wave自己的docs当唯一corpus。

## Source manifest

tracked：

`research/corpus/sources.json`

每个source：

```text
source_key
url
title
source_type
retrieved_at
content_digest
license_or_use_note
local_cache_name
```

raw cache：

`data/research/live/corpus/`

默认gitignored。

没有明确再分发许可的整篇网页/媒体不commit。

## Source units

保留真实结构：

- heading；
- paragraph；
- list；
- section；
- timestamp；
- media source locator。

Observation unit按语义段落/section建立，不用固定500字符chunk。

每unit：

```text
source_key
unit_key
source_locator
text or media reference
```

import后ignored mapping保存：

```text
source unit
→ artifact/source-region
→ occurrence
→ derived representations
→ memories
```

## Query oracle

tracked：

`research/corpus/queries.json`

每条：

```text
id
category
nousql
expected_source_units[]
acceptable_source_units[]
notes
```

category至少覆盖：

- lexical；
- paraphrase；
- entity；
- temporal；
- distractor；
- cross-source；
- correction/current-state（来源真实支持时）；
- association；
- early/long-distance information。

Coding Agent可以让LLM提出候选问题，但每一条必须回读真实source确认。source没写的信息不能进入oracle。

## Provenance-scored relevance

Memory文字允许paraphrase。

Hit relevant，当它的provenance最终追到 expected/acceptable source unit。

主要错误类别：

- wrong source；
- wrong entity；
- stale revision；
- unsupported synthesis；
- formation miss；
- retrieval miss。

## Controlled retrieval track

目的：隔离retrieval能力。

对选中的真实source unit：

- 通过public API建立Observation；
- Memory text使用原文中真实、bounded内容或人工忠实摘要；
- Evidence/provenance真实；
- 使用真实embedding API；
- 使用真实NousQL。

formation LLM不决定这个track的ground truth。

## End-to-end cognition track

```text
real source
→ Observation
→ DerivedRepresentation if needed
→ live formation LLM
→ Memory
→ embedding
→ NousQL
→ recall
```

Relevant仍按source provenance。

报告分开：

```text
formation coverage
retrieval conditional recall
end-to-end recall
```

这样能区分“没形成正确Memory”和“Memory存在但找不到”。

## Multimedia evaluation

对小代表集比较：

- description_only；
- direct_structured；
- describe_then_structure。

每个sample记录：

```text
source
strategy
model/profile
committed representation ids
manual factual checklist
hallucination observations
latency
usage/cost when available
downstream Memory
downstream query outcome
```

Coding Agent必须实际查看/听取原素材，手写简短事实checklist，例如：

```text
visible objects
main action
speaker/topic
important visible text
important uncertainty
```

不能让同一模型自评自己。

## Run artifacts

ignored：

```text
data/research/live/runs/<timestamp>/
  environment.json
  source-map.json
  model-profiles.redacted.json
  query-results.jsonl
  metrics.json
  media-results.json
  logs/
```

tracked只保留不含secret、可再分发的摘要和方法。

## Cost bounds

runner提供硬上限：

```text
--max-sources
--max-units
--max-queries
--max-model-calls
```

或等价机制。

如果gateway提供usage，记录：

- generation input/output tokens；
- embedding usage；
- rerank calls/documents；
- request count。

用户控制真实额度，runner负责避免意外无限调用。


# Live Gateway Qualification

## Live qualification is manual

真实付费gateway不进入每次PR CI，也不要求把用户token放GitHub Secrets。

建议命令：

```text
pnpm qualification:live
```

可以内部按slice拆分，但外部命令保持少而清楚。

## Preflight

打印redacted readiness：

- gateway host/base path；
- configured roles；
- model identifiers；
- protocols；
- prompt digests；
- embedding space signature；
- FFmpeg availability/version。

不得打印token。

## Protocol smoke

对实际配置role最小真实调用。

### Memory formation

验证：

- endpoint；
- bearer；
- text generation；
- structured output/validation；
- timeout。

### Embedding

验证：

- vector finite；
- dimension严格符合配置；
- space/producer signature一致。

### Rerank

真实2–4 documents：

- valid index；
- finite relevance score；
- adapter正确解析。

### Speech transcription

配置时使用真实短音频。

### Multimodal description

配置时使用真实图片。

Protocol smoke只证明protocol/model call，不宣称Memory recall通过。

## Real text vertical path

fresh data root：

```text
Core boot
→ official Node Client / nous-cli
→ Subject
→ Session
→ real external text source
→ Observation
→ live Memory formation
→ live embedding preparation
→ paraphrased NousQL
→ correct provenance hit
→ report meaningful UseEvent
→ process restart
→ same NousQL
→ correct provenance hit
```

保存redacted run evidence。

## Multimedia verticals

### Image

```text
real image
→ upload
→ Observation
→ ImageDescription
→ optional StructuredInterpretation
→ Memory formation
→ embedding
→ NousQL
→ trace to original source
```

### Audio

当 speech role configured：

```text
real audio
→ upload
→ Transcript
→ Memory formation
→ recall
```

### Video

当 FFmpeg + required model configured：

```text
real video
→ upload
→ bounded frames/audio
→ SceneDescription
→ optional structure
→ Memory formation
→ recall
```

缺少实际prerequisite时对应slice `BLOCKED`，不能用text PASS代替多媒体claim。

## Prompt/producer check

随机检查至少一个DerivedRepresentation和一个MemoryRevision，确认能追：

```text
model identifier
protocol
prompt logical id/digest
role/config digest
exact source evidence
```

## Controlled failure

触发一个低成本可控失败，例如local config中引用不存在的model id或临时移除optional role。

验证：

- role readiness/degradation明确；
- token不泄漏；
- optional rerank不可用时baseline仍返回；
- required role不可用时不会伪装Complete。

不要对真实gateway制造大量失败请求。

## Evidence categories

Qualification分别记录：

```text
local deterministic
Linux CI
live gateway
real corpus
multimedia slices
```

不能把其中一类外推到全部。


# Real Retrieval Experiment

## Remove the pre-baked experiment

当前 `apps/nous-core/research_retrieval.ts` 直接从fixture读取：

- candidate lists；
- support precision；
- entity error；
- latency；
- work units。

这不执行Nous retrieval。

本轮替换它。

metrics aggregation若有独立价值可提取纯函数；pre-baked runner和预填candidate fixture删除。

## New runner

建议：

```text
research/
  retrieval/
    import.ts
    run.ts
    metrics.ts
```

runner只使用official Client/Node Client，不调用Kernel内部服务、rank函数、Wave函数生成最终结果。

## Actual variants

每个variant通过真实config/query执行改变mechanism：

```text
baseline
baseline_model_rerank
baseline_wave
baseline_wave_model_rerank
```

### Baseline

- exact/entity/lexical/dense/temporal/runtime；
- topology disabled；
- model rerank disabled。

### Model rerank

与baseline相同，只有rerank enabled；candidate pool budget相同。

### Wave

topology enabled；same corpus/query/embedding/hard constraints/final limit。

没有真实topology signal时标NOT_RUN并解释。

## Authority dataset identity

variants必须使用同一 imported Authority dataset。

优先：

1. import一次canonical corpus；
2. formation/controlled Memories固定；
3. prepare embeddings；
4. 对同一Subject/Authority state切换Serving/query settings运行variants。

如果配置应用机制要求restart/rebuild，按真实产品流程做。

不允许每个variant重新跑formation LLM得到不同Memory集合。

## Public query path

全部最终query必须通过：

```text
client.cognition.recall(subjectId, nousql)
```

或official Client等价public call。

内部instrumentation可以提供机制统计，但不得替代final public query结果。

## Metrics

至少：

```text
Recall@1
Recall@5
Recall@10
MRR
provenance precision
wrong-source rate
wrong-entity rate
stale-revision leakage
empty-result rate
p50 latency
p95 latency
```

model rerank：

```text
rerank call count
rerank documents
rerank latency
```

Wave：

```text
topology seed count
visited nodes
topology candidate contribution
```

provider usage可得时：

```text
generation/embedding usage
gateway request count
```

## Latency

从public Client query call开始计时到response结束。

- warm-up和measured run区分；
- external rerank variant把gateway latency计入总延迟；
- 同时记录rerank sub-latency。

禁止fixture latency。

## Formation and retrieval decomposition

若end-to-end track中formation根本没形成支持expected source的Memory：

```text
formation miss
```

不能当作纯retrieval miss。

聚合同时输出：

```text
formation coverage
retrieval recall conditioned on available relevant Memory
end-to-end recall
```

## Honest interpretation

第一轮样本规模主要发现问题和比较observed behavior。

不要写“统计显著”除非真的做了足够样本和统计检验。

保留raw per-query results，使人工可以查看每个错误。

## Wave continuation decision

- observed增益且成本合理：继续experimental；
- mixed：保持research lane；
- 无增益/成本明显过高：允许后续直接重构或删除；
- 不能因为Wave已经写了很多代码而给它保留特权。

本轮没有授权把Wave设成default。

## 2026-10-01 expression execution allocation

Current canonical internal CognitiveQuery owns one CognitiveQueryExpr; old root-level targets/cues/constraints route is removed. Each expression has operation/typed cues/domain targets/local constraints/children. One BoundQuery resolves exact identities and configuration before work. Logical leaf order fixes allocation: floor(total/leaves), with the first total%leaves leaves receiving one extra. Apply this independently to each lane, total validation, topology nodes and resource actions. Contradictory inherited constraints yield an empty set. No validation allocation yields Partial with expression_budget_exhausted. Query sources are limited to64 expression nodes/16 depth. Branch combination uses identity intersection/union and maximum observed baseline contribution, with stable reference ordering for equal scores.

Query embedding uses actual batch requests for candidate text only, excludes soft preference text from candidate material, and caches at most128 vectors in process by exact text/space/producer digest. No cross-space cache reuse. Invocation summaries count actual model requests; cache hits do not claim new model calls. Required embedding failure rejects the operation; lexical fallback for optional/preferred remains visibly degraded.
