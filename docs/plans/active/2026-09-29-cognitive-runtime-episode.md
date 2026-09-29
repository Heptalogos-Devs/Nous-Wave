# Nous Wave Cognitive Runtime and Episode

状态：ACTIVE IMPLEMENTATION AUTHORIZATION
日期：2026-09-29

本计划执行本轮 WorkContext、Active Cognition、Episode foundation 和 Memory Reference semantic closure。长期语义服从 [Architecture-Vault Target Design](https://github.com/Heptalogos-Devs/Architecture-Vault/blob/main/docs/Nous-Wave/TARGET_DESIGN.md) 与 [Decisions](https://github.com/Heptalogos-Devs/Architecture-Vault/blob/main/docs/Nous-Wave/DECISIONS.md)；本仓库维护当前 executable truth。

## Current contracts

- [Memory Authority & Provenance](../../specs/active/memory-reference-profile/01-memory-authority-provenance.md)
- [Runtime & Use](../../specs/active/memory-reference-profile/02-runtime-use.md)
- [Query & Serving](../../specs/active/memory-reference-profile/03-query-serving.md)
- [Memory Reference Qualification](../../specs/active/memory-reference-profile/04-qualification.md)
- [WorkContext](../../specs/active/cognitive-runtime/work-context.md)
- [Active Cognition](../../specs/active/cognitive-runtime/active-cognition.md)
- [Episode Authority](../../specs/active/episode/episode-authority.md)
- [Episode Runtime Integration](../../specs/active/episode/episode-runtime-integration.md)

## Scope

本轮闭合 configuration mutation serialization、ResidentSet transaction ownership、synthesized provenance independence、typed temporal axes、Retrieval topology ownership、Focus→WorkContext replacement、Active Cognition runtime view、Memory-owned Episode foundation、research baseline/Wave harness 和 repo-native evidence。

WorkContext 属于 Runtime，拥有 Subject 级稳定身份并可跨 Session 延续；Episode 属于 Memory，支持 exact revision、members、provenance、hierarchy、track 和显式 lineage。两者都不复制 Memory Authority。

## Explicit deferrals

本轮不实现 Journal、Dream/Offline Cognition workflow、Self、Social、Motivation、Desired Condition、Heptalogos live connector、UI、automatic Episode boundary detector、automatic split/merge、learned fusion 或新的 PPR product path。

## Acceptance

Nous-Wave 必须通过 active qualification gates、官方 TypeScript Client 的 Memory Reference path、WorkContext cross-session continuity path、Episode foundation path 和 bundled deterministic retrieval research command。每项证据标记为 `PASS`、`FAIL`、`NOT_RUN` 或 `BLOCKED`，平台范围保持明确。

Git integration follows the repository-root execution contract: semantic feature branch, draft PR, Ready-for-review acceptance run, no post-PASS branch changes, squash merge and branch deletion.
