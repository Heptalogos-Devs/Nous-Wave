# Cognitive Retrieval & Evaluation Qualification

日期：2026-09-27

状态：CURRENT QUALIFICATION EVIDENCE

实施基线：`214a31f`（本记录保留历史实施提交引用，R2 新增修正见当前 HEAD 历史）。

Spec set：`docs/specs/active/cognitive-retrieval/`

## 1. Toolchain and environment

- Rust: `rustc 1.98.1`, `cargo 1.98.1`
- Node.js: `v24.20.0`
- pnpm: `12.4.1`
- PostgreSQL qualification fixture: embedded PostgreSQL `18.6.0`
- Embedding provider / embedding space: `NOT_RUN` in the reference qualification fixture; the reference path preserves Dense as an unavailable planned lane when no provider is configured.

## 2. Static and build gates

| Gate | Status | Evidence |
| --- | --- | --- |
| `corepack pnpm check` | PASS | Buf lint, TypeScript `tsc --noEmit`, Vitest: 3 files / 7 tests passed |
| `just verify` | PASS | fmt, workspace check, Clippy `-D warnings`, workspace tests, cargo-deny, cargo-shear, source-shape |
| `git diff --check` | PASS | verified before commit and for `HEAD^..HEAD` |
| Generated protocol bindings | PASS | `corepack pnpm generate`; Rust and TypeScript outputs regenerated from Proto |

## 3. Cognitive Retrieval regression qualification

| Item | Status | Evidence / boundary |
| --- | --- | --- |
| Q-01 stale old revision | PASS | `query_correctness::stale_lexical_generation_cannot_return_old_revision`；普通 recall 不返回 stale revision，exact historical 另行允许 |
| Q-02 arbitrary first-N elimination | PASS | `query_correctness::authority_lanes_reach_matches_beyond_first_n_objects`: 10,000 current Memory rows; tail-only Entity and Temporal matches returned |
| Q-03 Runtime resident isolation | PASS | Session A/B resident fixture; Session A query returned A only |
| Q-04 multi-value include | PASS | Entity query accepted both declarative and experiential roles |
| Q-05 lexical provider authority | PASS | `query_correctness::lexical_lane_does_not_create_rank_from_substring` 与 `lexical_provider_hit_survives_non_literal_case_difference` |
| Q-06 UseEvent duplicate side effect | PASS | Duplicate-only retry returned `accepted=0`, `duplicate=1`, unchanged runtime revision |
| Q-07 same-batch coalesce | PASS | Two same-digest new events returned `accepted=1`, `duplicate=1`, one runtime revision increment |
| Q-08 Schema provenance gate | PASS | Same-root synthesized rejection and two-known-root synthesized acceptance |
| Q-09 Memory identity drift | PASS | Disjoint aboutness revision rejected with `FailedPrecondition` |
| Q-10 exact Association endpoint | PASS | Mutable Memory endpoint rejected; exact revision endpoint accepted |
| Q-11 Association producer contract | PASS | `query_correctness::association_requires_exact_cognition_and_valid_support_class` 覆盖 meaningful-use 与 positive derived producer |
| Q-12 Association provenance to topology | PASS | Projection input emitted non-empty provenance root for association support; root dedup unit test passed |
| Q-13 contradiction non-propagating | PASS | Positive `contradicts` and negative edges excluded from ordinary non-negative graph |
| Q-14 single fusion owner | PASS | Canonical RRF owner is `crates/cognitive-retrieval/src/ranking.rs`; no duplicate RRF constants/formula in Memory Service; ranking unit tests passed |

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
| Versioned deterministic evaluation corpus | PASS (closure scope) | `apps/nous-kernel/tests/fixtures/memory-reference-r1/` plus `memory_reference_closure` executed fixed scenario |
| Query oracle with MUST_RETURN / MUST_NOT_RETURN | PASS (closure scope) | closure test asserts entity/aboutness, exact historical, runtime, restore and purge oracle; full lane/diagnostic category oracle remains NOT_RUN |
| Full scale fixture | PASS | `scale_fixture` 1/1: 10,000 current, 3,000 historical, 30,000 associations, 2,000 entity refs, 1,000 tags, 200 schemas |
| Performance baseline | PASS (observed, no SLA) | 20-query sample p50 ≈5.27ms, p95/max ≈84.17ms, Serving build ≈4003.49ms; query count/RSS NOT_INSTRUMENTED |
| Restart matrix | NOT_RUN | Composition restart/reopen PASS; independent Kernel subprocess restart was not run |
| Serving rebuild/reopen matrix | PASS (composition scope) | `reference_profile` corrupt-artifact reopen/rebuild and closure lifecycle refresh passed |
| Correctness admission gate | NOT_RUN | Full lane/diagnostic oracle, official client path and subprocess restart remain incomplete |
| Baseline vs Wave benchmark | NOT_RUN | Correctness admission gate was not PASS; benchmark was intentionally not run |

No qualification item produced a `FAIL` or `BLOCKED` result. `NOT_RUN` items are explicit missing evidence, not inferred passes.

## 6. Scope and deviations

- No `SPEC_CONFLICT` found against scoped AGENTS, Architecture-Vault Target Design/Decisions, Target Engineering Plan, or the active milestone.
- No `SPEC_GAP` found for an Authority, identity, lifecycle, transaction, idempotency, public protocol, purge, ranking, Serving-consistency, or algorithm-default decision required by this plan.
- Intentional differences from the Cognitive Retrieval Specs: None. Remaining `NOT_RUN` items are qualification boundaries, not fallback behavior or threshold changes.
- Benchmark and learned/provider-dependent qualification remain outside the executed reference evidence because the Spec 05 admission gate is not met.
