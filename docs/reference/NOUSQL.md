# NousQL 当前实现参考

本页描述 `apps/nous-core` 当前 parser/compiler 支持的 NousQL 子集。语义查询最终编译为 Protobuf `QueryExpr`；查询算法与检索通道由 Kernel/Serving 实现。

## 当前语法

- 原子：引号包围的文本、`#concept`、`*`，以及 `@e`、`@tag`、`@schema`、`@r`、`@object`、`@ref` selector。
- 组合：显式 `&&`、`||` 和括号；`&&` 优先于 `||`。
- 软偏好：`+atom`、`-atom` 和 `+recent(axis)`、`-recent(axis)`。
- 指令：`$memory`、`$schema`、`$episode`、`$journal`、`$evidence`、`$resource`、`$effort`、`$limit`、`$source`、`$modality`、`$cognitiveRole`、`$formationMode`、`$evidenceClass`、`$authority`、`$current`、`$diagnostics`、`$explore`、`$materialize`、`$exclude` 和 `$time`。

`$memory`、`$schema`、`$episode`、`$journal` 分别选择 Memory、CognitiveSchema、Episode、Journal；多个域指令取并集。未指定域时查询所有可用域，包括四类认知对象以及 Evidence/Resource 引用。父级域限制对子表达式和 exact target 生效。

单个软偏好操作数目前也接受冗余括号；canonical form 会省略这层括号。

实体 selector 可列多个参与者；绑定后按 LexicalRef 排序，重复规范身份会报错。名称必须由 Kernel identity resolver 唯一绑定；解析歧义不会退化为向量猜测。`@object` 接收不透明 Host 引用，`@ref` 接收 LexicalRef。

`$time` 支持 `occurred`、`observed`、`valid` 以及 `formed`、`recorded` 五条时间轴，时间点必须带 ISO-8601 offset；`within` 不能与 `from/to/at` 混用。查询指令在同一表达式节点内去重，跨 scope 的 modifier 不自动搬移。

## 当前限制

- 单个查询上限为 32 KiB、2048 个 token、16 层嵌套；`$limit` 范围是 1–2048。
- `persona`、`relation` 和 `rel` 当前返回 `FAILED_PRECONDITION`，对应认知领域没有当前服务实现。
- 此接口不提供物理算法 selector；它只表达 typed query intent 和约束。

精确 parser/compiler、生成协议及当前行为测试见 [`apps/nous-core/src/nousql`](../../apps/nous-core/src/nousql)、[`proto/nous/wave/v1alpha1`](../../proto/nous/wave/v1alpha1) 和 [`apps/nous-core/tests/nousql.test.ts`](../../apps/nous-core/tests/nousql.test.ts)。

Kernel 保留整棵 expression：AND 对 canonical candidate identity 取交集，OR 取并集；父约束继承，子约束细化。effort/limit/diagnostics/explore/materialize 只允许 root。整树共享预算，执行前按 leaf 数分配；不以 branch 数增加工作量。只有 @ref 是 exact read，其余 selectors 是 typed semantic cues。recent 轴只允许 occurred/observed/valid/formed/recorded，裸 recent 拒绝。

`$current(none|prefer|required)` 可用于 scope；父级 required 不被子级放宽，最终约束进入该 leaf 的 Resource action。exact 子目标仍受父级 domain 限制，例如 `$memory` 下的 Artifact exact read 不产生返回候选。

[返回文档目录](../INDEX.md)

相对时间窗口 `within` 以该 Subject 的 CognitiveClock 当前时间为基准，由 Core 在编译时固定；执行 timeout 与 retry 继续使用基础设施时间。

`client.cognition.prepareQuery` 与 Query 共用 NousQL compiler/Identity resolver。正式 TextCue 必须可独立解释；`我和她这个项目` 等未闭合表达返回 `UNRESOLVED_QUERY_REFERENCE` 的 span/kind，Agent 应先 resolve 再提交明确人物、项目和时间。`boundQuery` 现在为 resolved query/representation/source refs/profile/config digest 的 JSON inspection。

Query/prepare 可以显式提供 `sessionId`、`workContextId`、`situation.currentRefs/currentObjects/objectDescriptions/consumer`。Text-only research 使用 typed standalone TextCue 加 `textOnlyCompatibility: true`，与 cognitive input 分开。

QueryRequest 的 typed `capabilities` 传递 text embedding、multimodal interpretation、residual sensing 与 rerank requirement。`rerank: "forbidden"` 明确关闭 model rerank，适用于 deterministic algorithm/Agent wiring run；`text_embedding: "forbidden"`（Client 为 `textEmbedding`）禁止 embedding provider。Required 需求的失败不静默回退。
