# Memory Reference Profile Qualification

状态：IN PROGRESS
计划：[Structural Rebase — Memory Reference Profile R1](../plans/active/2026-09-29-structural-rebase-memory-reference-profile.md)
Spec：[Memory Reference Qualification](../specs/active/memory-reference-profile/04-qualification.md)

本记录只保留当前 checkout 实际运行的证据。平台：Windows；commit 以最终闭合时重新记录。

| Capability / gate | Result | Evidence |
| --- | --- | --- |
| TypeScript typecheck | PASS | `corepack pnpm typecheck` |
| Fresh schema / public write smoke | PASS | `corepack pnpm qualification:official-client` |
| Public Memory Reference path | PASS | `corepack pnpm qualification:memory-reference`; output included `restart=true`, `suppress_restore=true`, `purge=true`, `traceback=true` |
| Rust workspace library compile | PASS | `cargo check --workspace --lib` |
| Runtime residency regressions | PASS | `cargo test -p nous-kernel --test runtime_residency --all-features -- --test-threads=1` |
| Shared-source purge regression | PASS | `cargo test -p nous-kernel --test purge_shared --all-features -- --test-threads=1` |
| Remaining focused Rust portfolio | PASS | `just verify` — workspace compile, Clippy, serial workspace tests, deny, shear |
| `corepack pnpm check` | PASS | Buf lint, TypeScript, Vitest, Prettier, Oxlint, Knip, dependency-cruiser, Sherif |
| `just verify` | PASS | final run after governance/test/schema rebase |
| Cross-platform qualification | NOT_RUN | current evidence is Windows only |

Internal focused tests are evidence for their named contracts only. The public capability claim is based on the official Client path above.
