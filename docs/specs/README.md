# Executable Specs

状态：IMPLEMENTATION-FACING DOCUMENT CLASS
目标落库路径：`docs/specs/README.md`

## 1. 职责

Executable Spec 将已接受的目标语义和 active plan 降为可直接约束代码的实施合同。

它不拥有长期 Target Design，也不复制 Architecture-Vault。

Authority 顺序：

1. scoped `AGENTS.md`
2. Architecture-Vault Target Design / Decisions
3. Target Engineering Plan / Active Milestone Plan
4. Active Executable Specs
5. code / tests / current-state docs

如果当前代码与 Active Spec 不一致，代码是待迁移实现，不是反向 Authority。

## 2. Spec 生命周期

- `active/`：当前施工 Authority。
- Spec 覆盖范围内不得保留 `TBD` / “自行选择” / 多个未决定方案。
- Spec 中允许存在实验候选，但必须先指定一个 reference/default implementation。
- benchmark 只能在稳定 reference implementation 上进行。
- 阶段结束后可归档，但 Qualification 必须指向实际使用过的 Spec revision/commit。

## 3. Implementation freedom

Agent 可以自行决定：

- 私有 helper 名称；
  -不会改变合同的局部文件拆分；
  -纯机械重构；
  -等价 SQL 写法。

Agent 不得自行决定：

- Authority / owner；
- identity / revision；
- lifecycle；
- transaction boundary；
- concurrency / idempotency；
- public protocol；
- retry/recovery；
- purge；
- ranking semantics；
- Serving consistency；
- 算法默认路径。

## 4. 当前 Active Specs

- `active/cognitive-retrieval/`
- `active/self-authority/`
- `active/configuration-foundation-and-social-cognition/`
