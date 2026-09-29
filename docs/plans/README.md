# 施工计划

计划层连接长期目标与代码施工，不能替代 Architecture-Vault 的目标语义 Authority，也不能把研究候选直接提升为实现默认值。

本页是计划集合的入口；具体计划负责范围、执行顺序和验收授权，Qualification 文档负责实际运行证据。已完成或 superseded 的计划不应继续出现在“当前”列表中。

## 文档层次

Target Design → Target Engineering Plan → Active Milestone Plan → Executable Specs → Code → Qualification

## 长期工程计划

- [Target Engineering Plan](roadmap/2026-09-target-engineering-plan.md)：长期目标到真实 code owners、迁移顺序和验证闭环的 Pre-Spec 计划。

## 当前里程碑

- [2026-10-27 Memory Reference Profile](active/2026-10-27-memory-reference-profile.md)：Target Engineering Plan 的阶段投影，定义中期范围、Must-have 语义和 Qualification 方向。
- [Structural Rebase — Memory Reference Profile R1](active/2026-09-29-structural-rebase-memory-reference-profile.md)：当前唯一新增代码施工授权，负责 current truth、crate/schema/test/governance rebase 和 public acceptance closure。

## 当前 Executable Specs

- [Executable Specs 索引](../specs/README.md)：实施合同的职责、Authority 顺序与生命周期。
- [Memory Reference Profile active spec set](../specs/active/memory-reference-profile/README.md)：Memory Authority、Runtime/Use、Query/Serving 与 Qualification 合同。

Self Authority 与 Configuration Foundation/Social Cognition 的实现计划和 Specs 已移至 `superseded/`；实现保留，但当前不授权扩展。
