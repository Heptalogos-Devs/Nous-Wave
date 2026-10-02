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

`data/research/runs/corpus/`

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
data/research/runs/runs/<timestamp>/
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


# Real Retrieval Experiment

## Runner

`scripts/research/retrieval.ts` 通过 import/run/audit-formation commands 管理固定语料、查询和 formation 重放，`scripts/research/media.ts` 对比媒体策略。

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

没有真实topology signal时标未测量并解释。

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

Topology 保持显式实验 lane。

[返回文档目录](../INDEX.md)
