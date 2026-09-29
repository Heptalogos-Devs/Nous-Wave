# 2026-10-27 Nous Wave 中期检查计划

状态：ACTIVE MILESTONE PLAN
父计划：[Target Engineering Plan](../roadmap/2026-09-target-engineering-plan.md)

语义 Authority：[Architecture-Vault Nous Wave Target Design](https://github.com/Heptalogos-Devs/Architecture-Vault/blob/main/docs/Nous-Wave/TARGET_DESIGN.md) 及其已接受设计决定。

## 目标

中期检查交付 **Memory Reference Profile**：在真实 public path 中证明长期 Subject Memory 的来源、Authority、修订、时间、查询、使用、生命周期、Serving 与恢复语义可以形成闭环。高级联想算法独立评估，不改变 Authority。

## Memory Reference

主 demo spine：

`Memory-only Subject → Session A/B → Observation/Artifact → grounded Memory → typed public Query → meaningful UseEvent + retry → process restart → same identity/revision → suppress → restore → purge → no stale recall → provenance trace-back`

Integrity evidence is organized as high-information scenarios:

- provenance independence：同源派生不放大 independent root，独立 source 才满足 synthesized gate；
- revision/world-state：同一认知事项用 revision，后来世界状态用新 object 和 explicit successor；
- entity/temporal isolation：aboutness 与 source mention 分离，occurred/observed/valid/formed/recorded 五个 typed axes 按 hard intersection；
- use/lifecycle：retrieval hit 不等于 meaningful use，receipt retry 幂等，suppression/restore/purge 分离；
- serving/recovery：stale generation 经 final Authority validation 丢弃，watermark rebuild、vector-space isolation 和 restart 可追溯。

Memory acceptance 不依赖 WorkContext 或 Episode。

## Runtime、Episode 与研究

当前 continuation 工作由 [Cognitive Runtime and Episode plan](2026-09-29-cognitive-runtime-episode.md) 授权：

- WorkContext 是 Runtime-owned、Subject-level、可跨 Session 延续的 durable checkpoint；
- Active Cognition 是 ResidentSet、active WorkContext exact refs 与 request situation refs 的 Runtime view；
- Episode 是 Memory-owned representational foundation，支持 exact identity/revision、members、provenance、hierarchy、track、lineage 和 lifecycle；
- 不实现 automatic boundary detector、automatic split/merge、Journal、Dream、ranked Episode retrieval 或新的 product topology。

Bundled retrieval research 固定 corpus、QueryPlan、budget、embedding space、hard constraints 和 final validator，对比 baseline 与 experimental Wave lane。Wave 结果不自动改变默认路径。

## 外部 benchmark 与官方输入

LongMemEval、Memora 等只做 selected mapping；网络、授权或 dataset preparation 成本过高时记录 `NOT_RUN`，不阻塞内部 Integrity Suite。

截至当前记录仍缺：项目任务书及获批考核指标、获批 2,000 元预算分项、计算机学院 10 月 27 日具体检查时间与材料格式。不得从工程计划推导学院要求。

## 范围边界

本计划不实现 Journal、Dream/Offline Cognition workflow、Self、Social、Motivation、Desired Condition、Heptalogos live integration、UI、learned fusion、automatic Episode boundary detector 或新的 PPR product path。

完成实际授权行为和所需 evidence 后停止。
