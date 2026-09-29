# Memory Reference Profile Qualification

状态：QUALIFIED（Windows scope；本轮 continuation evidence 已补充）
计划：[Cognitive Runtime and Episode](../plans/active/2026-09-29-cognitive-runtime-episode.md)
Spec：[Memory Reference Qualification](../specs/active/memory-reference-profile/04-qualification.md)

本记录只保留当前 checkout 实际运行的证据。平台：Windows。tested commit：`4257360`；该记录不自引用其自身 commit。

| Capability / gate | Result | Evidence |
| --- | --- | --- |
| TypeScript typecheck | PASS | `corepack pnpm typecheck` |
| Fresh schema / public write smoke | PASS | `corepack pnpm qualification:official-client` |
| Public Memory Reference path | PASS | `corepack pnpm qualification:memory-reference`; output included `restart=true`, `suppress_restore=true`, `purge=true`, `traceback=true` |
| WorkContext / Episode public path | PASS | `corepack pnpm qualification:cognitive-runtime-episode`; output included `restart=true`, `cross_session=true`, `episode_revision=true` |
| Rust workspace library compile | PASS | `cargo check --workspace --lib` |
| Runtime residency regressions | PASS | `cargo test -p nous-kernel --test runtime_residency --all-features -- --test-threads=1` |
| Shared-source purge regression | PASS | `cargo test -p nous-kernel --test purge_shared --all-features -- --test-threads=1` |
| Remaining focused Rust portfolio | PASS | `just verify` — workspace compile, Clippy, serial workspace tests, deny, shear |
| `corepack pnpm check` | PASS | Buf lint, TypeScript, Vitest, Prettier, Oxlint, Knip, dependency-cruiser, Sherif |
| `just verify` | PASS | final run after governance/test/schema rebase |
| `cargo build -p nous-kernel` | PASS | tested commit `4257360` |
| `corepack pnpm qualification:official-client` | PASS | official Client closure on tested commit |
| Cross-platform qualification | NOT_RUN | current evidence is Windows only |
| Linux Ready-for-review acceptance workflow | NOT_RUN | PR has not yet reached the required Ready-for-review CI event |
| External LongMemEval/Memora mapping | NOT_RUN | no external dataset run in this acceptance slice |

Internal focused tests are evidence for their named contracts only. The public capability claim is based on the official Client path above.
