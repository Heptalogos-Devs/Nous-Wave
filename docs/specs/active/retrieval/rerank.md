# QueryExpr 与 Rerank

atom 是 leaf candidate set，all 为 canonical identity 交集，any 为并集。父 hard constraints 被继承，子约束只收窄子树；一棵树固定单一 BoundQuery/QueryPlan。最多64 nodes、depth16。各 lane/validation/topology/resource 总预算按 leaf logical order 预分配：floor(total/leaves)，前 total%leaves 个多一份。约束矛盾为空集；无 validation allocation 返回 Partial/expression_budget_exhausted。

effort/limit/diagnostics/explore/materialize 仅 root 可用。typed entity/tag/schema/resource/external-object cues 与 exact @ref 分离。未知 modifier 拒绝。软偏好只排序通过 Authority 和 hard constraints 的 candidates，不生成候选。

## 执行

candidate lanes → 固定 RRF → bounded Authority validation/materialization → optional model rerank → final Authority revalidation → user limit。

Core 拥有 query embedding、rerank 与 Resource host action；Kernel 拥有 BoundQuery、QueryPlan、candidate fusion 与 final validation。内部 validated_candidate_limit 最大64，不改写用户 limit。正文本意图由 text/concept cues 和 ALL/ANY 结构确定；identity-only 查询跳过模型。Resource ticket 仍 finalize/release。

Rerank 请求使用标准 rerank-v1：query/documents/top_n。候选 canonical refs 固定；index 范围和唯一性、finite score 必须校验，遗漏候选保持 baseline tail。required 调用失败使 operation 失败；preferred/optional 失败返回 baseline 和 query_rerank_unavailable degradation。配置缺失的 optional role 不执行。

模型调用后按原 BoundQuery 批量核验 exact revision/head/epoch/lifecycle/source/hard constraints；mutable target 不重绑。变化候选丢弃，返回 authority_changed_during_rerank。ticket one-use、最多16 pending snapshots、保留6分钟。finally release。

## 分数与输入

public HitScore 保存 baseline/preference/optional rerank/final 与 ranks。rerank 分数 query-ephemeral；长期使用由 UseEvent 表达。QueryDiagnostics 为显式 opt-in，解释 lane availability、budget 和显式 topology work。真实 request/usage/latency/cost 由研究 runner/proxy 记录。

Normalized RRF baseline 在[0,1]。每个满足的 soft cue 贡献 signed 0.02，总偏好 clamp [-0.06,0.06]。recent(axis) 只接受 occurred/observed/valid/formed/recorded，未知时间为0；recency=1/(1+age/(30 days))。rerank final ordinal affinity=(k+1)/(k+rank)。pool 最大64 candidates/2MiB text。

Lexical owner 使用索引 analyzer，union body/title literal terms；文字不作为 Tantivy 查询语法。Query embedding 只取 candidate text，排除 soft preference text，实际 batch 发送并按 exact text/space/producer digest 缓存最多128 vectors。required embedding 失败拒绝 operation；optional/preferred lexical fallback 显式 degradation。

Topology 默认关闭，显式实验 lane 标识 experimental-node-potential-v1。weighted PCA/residual/bounded propagation/node-potential 属于当前实现；完整 VCP 尚待后续任务。
