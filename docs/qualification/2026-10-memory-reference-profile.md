# Memory Reference Profile Qualification

状态：QUALIFIED（Windows local 与 Linux Ubuntu Ready CI；各证据按 commit 区分）
计划：[Real Usage and Retrieval](../plans/active/2026-09-30-real-usage-retrieval.md)
Spec：[Memory Reference Qualification](../specs/active/memory-reference-profile/04-qualification.md)

本记录只保留当前 checkout 实际运行的证据。平台：Windows。tested commit：`45c26e7`；该记录不自引用其自身 commit。

| Capability / gate | Result | Evidence |
| --- | --- | --- |
| TypeScript typecheck | PASS | `corepack pnpm typecheck` |
| Fresh schema / public write smoke | PASS | `corepack pnpm qualification:official-client` |
| Public Memory Reference path | PASS | `corepack pnpm qualification:memory-reference`; output included `restart=true`, `suppress_restore=true`, `purge=true`, `traceback=true` |
| WorkContext / Episode public path | PASS | `corepack pnpm qualification:cognitive-runtime-episode`; output included `restart=true`, `cross_session=true`, `episode_revision=true`, `overlap=true`, `hierarchy=true`, `lifecycle=true`, `purge=true` |
| Rust workspace library compile | PASS | `cargo check --workspace --lib` |
| Runtime residency regressions | PASS | `cargo test -p nous-kernel --test runtime_residency --all-features -- --test-threads=1` |
| Shared-source purge regression | PASS | `cargo test -p nous-kernel --test purge_shared --all-features -- --test-threads=1` |
| Remaining focused Rust portfolio | PASS | `just verify` — workspace compile, Clippy, serial workspace tests, deny, shear |
| `corepack pnpm check` | PASS | Buf lint, TypeScript, Vitest, Prettier, Oxlint, Knip, dependency-cruiser, Sherif |
| `just verify` | PASS | final run after governance/test/schema rebase |
| `cargo build -p nous-kernel` | PASS | tested commit `45c26e7` |
| `corepack pnpm qualification:official-client` | PASS | official Client closure on tested commit |
| Linux Ubuntu Ready-for-review acceptance | PASS | [run 36573357286](https://github.com/Heptalogos-Devs/Nous-Wave/actions/runs/36573357286), PR #4, tested head `c286721dfb84cb70a54fda916fe8992aefc9678a`; protocol regeneration, TS checks, Rust verification/build, Memory Reference and Cognitive Runtime/Episode public paths |
| macOS / broader platform qualification | NOT_RUN | no evidence for these platforms |
| External LongMemEval/Memora mapping | NOT_RUN | no external dataset run in this acceptance slice |

Internal focused tests are evidence for their named contracts only. The public capability claim is based on the official Client path above.
