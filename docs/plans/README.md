# 施工计划

计划层连接长期目标与代码施工，不能替代 Architecture-Vault 的目标语义 Authority，也不能把研究候选直接提升为实现默认值。

本页是计划集合的入口；具体计划负责范围、执行顺序和验收授权，Qualification 文档负责实际运行证据。已完成或 superseded 的计划不应继续出现在“当前”列表中。

## 文档层次

Target Design → Target Engineering Plan → Active Milestone Plan → Executable Specs → Code → Qualification

## 长期工程计划

- [Target Engineering Plan](roadmap/2026-09-target-engineering-plan.md)：长期目标到真实 code owners、迁移顺序和验证闭环的 Pre-Spec 计划。

## 当前里程碑

- [2026-10-27 Memory Reference Profile](active/2026-10-27-memory-reference-profile.md)：Target Engineering Plan 的阶段投影，定义中期范围、Must-have 语义和 Qualification 方向。
- [Memory Reference Profile Closure R2](active/2026-09-28-memory-reference-profile-closure.md)：当前唯一新增代码施工授权，负责 Memory-only、多 Session、恢复和 Qualification closure。

## 当前 Executable Specs

- [Executable Specs 索引](../specs/README.md)：实施合同的职责、Authority 顺序与生命周期。
- [Cognitive Retrieval & Evaluation Substrate](active/2026-09-27-cognitive-retrieval-evaluation.md)：当前 Cognitive Retrieval 施工计划。
- [Cognitive Retrieval active spec set](../specs/active/cognitive-retrieval/README.md)：本施工计划的 BoundQuery、fusion、Schema/Association/Topology、Runtime 与 Qualification Specs。

Self Authority 与 Configuration Foundation/Social Cognition 的实现计划和 Specs 已移至 `superseded/`；实现保留，但当前不授权扩展。
