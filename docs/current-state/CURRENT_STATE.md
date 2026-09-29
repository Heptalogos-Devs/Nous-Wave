# Nous Wave 当前状态

当前唯一新增代码施工授权是 [Structural Rebase — Memory Reference Profile R1](../plans/active/2026-09-29-structural-rebase-memory-reference-profile.md)。10/27 milestone 仍是阶段目标；当前实现以四份 [Memory Reference Specs](../specs/active/memory-reference-profile/README.md) 为直接合同。

## Capability matrix

| Capability | State | Evidence |
| --- | --- | --- |
| Public dev boot | PASS | `corepack pnpm dev` printed Core endpoint and discovery record during this checkout |
| Memory-only Subject | PASS | `corepack pnpm qualification:memory-reference` |
| Observation → grounded Memory | PASS | public qualification path |
| Public Query/Serving recall | PASS | public qualification path |
| Two-session Runtime isolation | PASS | public qualification path; focused `runtime_residency` remains current regression |
| UseEvent idempotency | PASS | public qualification path; focused query/runtime regressions |
| Process restart and same Authority revision | PASS | public qualification path; `process_restart` focused test |
| Suppression/restore | PASS | public qualification path |
| Purge and shared source retention | PASS | public qualification path; `purge_shared` focused test |
| Provenance trace-back | PASS | public qualification path |
| Self/Social executable slices | NOT_RUN | removed from current checkout; long-term semantics remain in Architecture-Vault |
| Cross-platform qualification | NOT_RUN | current evidence is this Windows workstation only |

## Current verification evidence

- `corepack pnpm typecheck` — PASS.
- `corepack pnpm qualification:memory-reference` — PASS; output includes `restart=true`, `suppress_restore=true`, `purge=true`, `traceback=true`.
- `corepack pnpm qualification:official-client` — PASS; legacy write/use smoke retained as a narrower diagnostic.
- `cargo check --workspace --lib` — PASS.
- `cargo test -p nous-kernel --test purge_shared --all-features -- --test-threads=1` — PASS.
- `cargo test -p nous-kernel --test runtime_residency --all-features -- --test-threads=1` — PASS.
- `corepack pnpm check` — PASS.
- `just verify` — PASS.

精确命令、commit、平台和失败/未运行项由 [Memory Reference Qualification](../qualification/2026-10-memory-reference-profile.md) 维护；本文只保留 capability summary。
