# Self Authority 开发计划

STATUS: SUPERSEDED FOR CURRENT EXECUTION
IMPLEMENTATION EXISTS; NO CURRENT EXPANSION AUTHORITY

日期：2026-09-27
状态：SUPERSEDED FOR CURRENT EXECUTION（Self Authority vertical slice 已落地；Seed parser/import 的历史 `SPEC_GAP` 记录保留）
目标路径：`docs/plans/superseded/2026-09-27-self-authority.md`

## 1. 目标

本计划的实现已保留在当前仓库。由于 Memory Reference Profile qualification 尚未闭合，本计划不再授权当前扩展；其原先关于“Memory spine 已稳定”的进入理由被后续 qualification 证明过早。待 Memory Reference Profile closure 后重新排期。

实现独立 Self Authority，使主体可以保存、修订和查询：

- 身份认知；
- 角色；
- 能力；
- 局限；
- 人格/行为倾向；
- 价值；
- 长期偏好；
- Narrative Identity。

同时把现有 Character Seed 收敛为目标设计中的 Cognitive Seed，并让 Seed 只承担初始化来源职责。

本次施工还会把 Cognitive Query 从 Memory-only contributor 结构改成能够容纳多个 cognition owner 的共享查询机制，为后续 Social Cognition 与 Motivation 避免重复实现。

## 2. 为什么现在进入 Self

Memory / provenance / revision / use / query / serving 已经形成可复用基础。

Target Engineering Plan 已明确 Self 是该基础之后的下一个 cognition owner。

继续只在 Memory 上增加更多 benchmark、测试矩阵或算法实验，当前收益低于建立第二个真实 cognition owner。Self 会直接验证：

- shared cognition primitives 是否真的可复用；
- query 是否真的不是 Memory 私有机制；
- Seed 是否正确地是 source，而不是人格真值；
- lifecycle / revision / provenance 是否能够跨领域保持一致。

## 3. 本次施工顺序

### A. 清理长期代码中的阶段标签

仅改名称，不改变行为。

### B. 工具链配置降噪

先让新增工具真正服务维护，而不是形成永久红灯。

### C. 抽取 shared cognition primitives

把已经被 Memory 使用、且 Self/Social/Motivation 都需要的生命周期与 support 类型移出 `memory-domain`。

不新建“万能框架 crate”；优先放入现有 `nous-core`。

### D. Cognitive Seed 正式化

删除 Character Seed 命名与“revise current character seed”的旧语义。

建立 immutable Cognitive Seed version + Subject adoption history。

### E. Self domain / service

新增：

- `self-domain`
- `self-service`

实现 Self facet 与 Narrative Identity 的 object/revision/lifecycle/provenance。

### F. Seed → Self 初始化

使用 deterministic TOML format。

不在 Subject 创建事务里调用模型。

### G. Cognitive Query 多 owner

把 query contributor 从单一 Memory 参数演化为固定的多个 cognition owner。

Self 支持 exact / direct / lexical（通过 shared Serving document）查询。

### H. 当前查询实现维护修正

顺手处理：

- Memory candidate N+1；
- Schema final revalidation；
- 3 个已有高价值未执行场景。

### I. 验收与文档

更新 current-state、API、索引和验收记录。

## 4. 本次明确不做

- Social Cognition；
- Motivation / Desired Condition；
- Episode / Journal；
-自动人格学习；
-自动 Self 冲突合并；
-模型自动重写 Narrative；
- Heptalogos live integration；
-新图算法；
-为了 benchmark 新建完整 evaluation framework；
-全仓 mutation testing；
-为了覆盖率写测试。

## 5. 完成后的结构

```text
Subject Foundation
├── Cognitive Seed versions / adoption
│
├── Memory Authority
│
├── Self Authority
│   ├── Self Facets
│   └── Narrative Identity
│
├── Cognitive Runtime
│
├── Cognitive Query
│   ├── Memory contributor
│   └── Self contributor
│
└── Serving
    └── rebuildable cognitive documents
```

Social / Motivation 后续接入相同 query contributor 与 shared cognition primitives，不再复制 Memory 机制。
