# R2 Cognitive Retrieval & Evaluation Qualification

日期：2026-09-27

状态：CURRENT QUALIFICATION EVIDENCE

实施提交：`14f644376ce0664c0d3d6b1195a836eb96ba3c19`

Spec set：`docs/specs/active/r2-cognitive-retrieval/`

## 1. Toolchain and environment

- Rust: `rustc 1.98.1`, `cargo 1.98.1`
- Node.js: `v24.20.0`
- pnpm: `12.4.1`
- PostgreSQL qualification fixture: embedded PostgreSQL `18.6.0`
- Embedding provider / embedding space: `NOT_RUN` in the reference qualification fixture; the R2 reference path preserves Dense as an unavailable planned lane when no provider is configured.

## 2. Static and build gates

| Gate | Status | Evidence |
| --- | --- | --- |
| `corepack pnpm check` | PASS | Buf lint, TypeScript `tsc --noEmit`, Vitest: 3 files / 7 tests passed |
| `just verify` | PASS | fmt, workspace check, Clippy `-D warnings`, workspace tests, cargo-deny, cargo-shear, source-shape |
| `git diff --check` | PASS | verified before commit and for `HEAD^..HEAD` |
| Generated protocol bindings | PASS | `corepack pnpm generate`; Rust and TypeScript outputs regenerated from Proto |

## 3. R2 regression qualification

| Item | Status | Evidence / boundary |
| --- | --- | --- |
| Q-01 stale old revision | NOT_RUN | Exact mutable binding fencing and explicit historical read are PASS; stale lexical-generation fixture was not run as a complete scenario |
| Q-02 arbitrary first-N elimination | PASS | `r2_query_correctness::authority_lanes_reach_matches_beyond_first_n_objects`: 10,000 current Memory rows; tail-only Entity and Temporal matches returned |
| Q-03 Runtime resident isolation | PASS | Session A/B resident fixture; Session A query returned A only |
| Q-04 multi-value include | PASS | Entity query accepted both declarative and experiential roles |
| Q-05 lexical provider authority | NOT_RUN | Non-provider substring fallback case is PASS; tokenizer-hit/non-literal fixture was not run as the complete pair |
| Q-06 UseEvent duplicate side effect | PASS | Duplicate-only retry returned `accepted=0`, `duplicate=1`, unchanged runtime revision |
| Q-07 same-batch coalesce | PASS | Two same-digest new events returned `accepted=1`, `duplicate=1`, one runtime revision increment |
| Q-08 Schema provenance gate | PASS | Same-root synthesized rejection and two-known-root synthesized acceptance |
| Q-09 Memory identity drift | PASS | Disjoint aboutness revision rejected with `FailedPrecondition` |
| Q-10 exact Association endpoint | PASS | Mutable Memory endpoint rejected; exact revision endpoint accepted |
| Q-11 Association producer contract | NOT_RUN | Missing-producer rejection path is implemented but no positive derived-producer fixture was run |
| Q-12 Association provenance to topology | PASS | Projection input emitted non-empty provenance root for association support; root dedup unit test passed |
| Q-13 contradiction non-propagating | PASS | Positive `contradicts` and negative edges excluded from ordinary non-negative graph |
| Q-14 single fusion owner | PASS | Canonical RRF owner is `crates/memory-retrieval/src/ranking.rs`; no duplicate RRF constants/formula in Memory Service; ranking unit tests passed |

## 4. Existing R1 integrity continuation

| Integrity area | Status |
| --- | --- |
| Operation idempotency and revision fencing | PASS |
| Source-root independence and provenance cycle protection | PASS |
| Suppression / restore and two-phase purge | PASS |
| Dependent revalidation and shared-source retention | PASS |
| Serving watermark publication fence and artifact checksum | PASS |
| Embedding-space isolation | PASS in existing unit/integration coverage; live provider qualification NOT_RUN |
| Wave propagation state key including remaining budget | PASS |

Evidence: `cargo test --workspace --all-features`, including `reference_profile`, `qualification_edges`, `schema_gate`, `purge_shared`, and retrieval unit tests.

## 5. Evaluation and benchmark gates

| Item | Status | Reason |
| --- | --- | --- |
| Versioned deterministic evaluation corpus | NOT_RUN | No R2 corpus artifact was added or executed |
| Query oracle with MUST_RETURN / MUST_NOT_RETURN | NOT_RUN | Regression fixtures were executed directly; the Spec 05 corpus/oracle artifact was not run |
| Full scale fixture | NOT_RUN | The 10,000-row Entity/Temporal structural fixture passed, but the complete Memory/history/Association/Tag/Schema scale matrix was not run |
| Performance baseline | NOT_RUN | No p50/p95/query-count/process-memory report was collected |
| Restart matrix | NOT_RUN | Full process restart and post-restart authority/use/resident proof was not run |
| Serving rebuild/reopen matrix | NOT_RUN | Existing serving unit coverage passed; the complete Spec 05 rebuild/reopen matrix was not run |
| Correctness admission gate | NOT_RUN | Deterministic corpus, full scale matrix, and restart/rebuild matrix were not all PASS |
| Baseline vs Wave benchmark | NOT_RUN | Correctness admission gate was not PASS; benchmark was intentionally not run |

No qualification item produced a `FAIL` or `BLOCKED` result. `NOT_RUN` items are explicit missing evidence, not inferred passes.

## 6. Scope and deviations

- No `SPEC_CONFLICT` found against scoped AGENTS, Architecture-Vault Target Design/Decisions, Target Engineering Plan, or the active milestone.
- No `SPEC_GAP` found for an Authority, identity, lifecycle, transaction, idempotency, public protocol, purge, ranking, Serving-consistency, or algorithm-default decision required by R2.
- Intentional differences from the R2 Specs: None.
- Benchmark and learned/provider-dependent qualification remain outside the executed reference evidence because the Spec 05 admission gate is not met.
