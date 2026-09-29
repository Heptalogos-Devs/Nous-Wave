# Nous Wave Structural Rebase — Memory Reference Profile R1

状态：ACTIVE IMPLEMENTATION AUTHORIZATION
日期：2026-09-29

本计划是当前唯一新增代码施工授权。它保持 [2026-10-27 Memory Reference Profile](2026-10-27-memory-reference-profile.md) 的阶段目标，并执行外部 `Nous-Wave-Structural-Rebase-Spec-R1` 交接包。

## Authority

语义 Authority 仍是 Architecture-Vault 的 [TARGET_DESIGN.md](https://github.com/Heptalogos-Devs/Architecture-Vault/blob/main/docs/Nous-Wave/TARGET_DESIGN.md) 与 [DECISIONS.md](https://github.com/Heptalogos-Devs/Architecture-Vault/blob/main/docs/Nous-Wave/DECISIONS.md)。本仓库维护当前实现、实现合同和实际 Qualification 证据。

交接身份：

- package：`Nous-Wave-Structural-Rebase-Spec-R1`
- manifest SHA256：`f24e145501a56fd753aef18d5c7180189bf7b66ff984d8c8d967fa84f32542d2`
- Nous-Wave baseline：`ba3b4b09512d8567681e0009b9d105f7560c8b53`
- Architecture-Vault baseline：`c1c1bb9a0f3a3595d6bcf0468163bf43754ef610`

## Authorized rebase

本仓库处于 PRE_PRODUCTION。没有当前外部兼容义务时，直接替换 current internal shape：

- 删除当前 Self/Social executable slices；长期 Self/Social 语义继续由 Vault 维护；
- 将有持续独立价值的 domain/service pairs 合并为当前 owner crate；
- 将 Cognitive Seed foundation 迁入 Subject；
- 将 fresh schema 重写为当前 canonical baseline，并重置项目拥有的 dev/test state；
- 将 Query contract/fusion semantics 放回 Runtime，将 retrieval/Serving mechanics 合并到 Retrieval；
- 删除旧 active Plan/Spec、测试、配置、protocol 和文档 residue，不建立 alias、双读、双写、fallback 或 compatibility migration；
- 建立精简治理与四个高信息价值项目 Skills。

## Current Spec set

本计划唯一的 Memory Reference current executable Spec set 是：

1. [Memory Authority & Provenance](../../specs/active/memory-reference-profile/01-memory-authority-provenance.md)
2. [Runtime & Use](../../specs/active/memory-reference-profile/02-runtime-use.md)
3. [Query & Serving](../../specs/active/memory-reference-profile/03-query-serving.md)
4. [Qualification](../../specs/active/memory-reference-profile/04-qualification.md)

旧 `cognitive-retrieval` active Specs 与 R2 closure plan 的有效内容已吸收到上述 current set；它们不再是 parallel Authority。

## Public acceptance requirement

必须从平台中立的 public dev entrypoint 启动 Core 与 private loopback/ephemeral Kernel，并通过官方 TypeScript Client 完成：Memory-only Subject、两个 Session、Observation→Memory、public Query recall、meaningful UseEvent 及 retry、进程重启、同一 Authority identity/revision 查询、suppress/restore、purge、Serving 不再返回被清认知、provenance trace-back。

## Completion and stop conditions

完成必须同时满足：

- current truth 只有本计划和四份 Memory Reference Specs；
- current Rust/Proto/Core 不再含 Self/Social executable slices；
- crate topology、fresh schema、tests、verification、governance 与 README/INDEX 对齐；
- public acceptance path 实际运行并按 Spec 04 记录证据；
- 所有未运行或失败项明确标记 `NOT_RUN`、`FAIL` 或 `BLOCKED`；
- structural residue search 与文档/link checks 完成。

达到这些条件后停止扩张；Episode、Journal、Offline Cognition、Motivation、Self/Social reimplementation、Heptalogos live integration 和新的 topology algorithm 不属于本计划。
