# Self Authority 验收记录

日期：2026-09-27

范围：已 superseded 的 Self Authority plan 与 `docs/specs/superseded/self-authority/`；实现保留，当前不授权 Self 扩展。

## Semantic slices

| Slice | Status | Evidence |
| --- | --- | --- |
| shared cognition lifecycle/support primitives in `nous-core` | PASS | `cargo check --workspace --all-targets --all-features`; Memory uses shared re-exports and Self has no `memory-domain` dependency |
| Cognitive Seed version/adoption persistence | PASS | migration `0005_self_authority.sql`; `self_authority` fixture creates and consumes an immutable Seed version |
| Seed adoption idempotency / digest conflict | PASS | `self_authority::cognitive_seed_adoption_is_idempotent_and_digest_fenced` |
| Seed TOML parser and automatic Seed→Self import | BLOCKED | `SPEC_GAP`: package format ends at Narrative `text =` |
| Self Facet identity/create | PASS | `self_authority::self_facet_revision_fencing_lifecycle_and_direct_query` |
| Self revision parent/object-epoch fencing | PASS | same integration test; stale parent/epoch returns `Conflict` |
| Self lifecycle and purge receipt | PASS | same integration test; suppressed facet is excluded, purge removes the object path, and projection refresh removes its Serving document |
| Narrative exact references and revision | PASS | `self_authority::narrative_reference_is_exact_and_source_purge_revalidates_narrative` |
| Narrative/dependency invalidation after Self lifecycle and purge | PASS | same integration test observes narrative and dependent Self facet `RevalidationRequired` |
| SelfDirect, historical exact binding, lexical Serving and context materialization | PASS | `self_authority` integration assertions; Self lexical document and exact context path are exercised |
| Self/Narrative meaningful-use resident | PASS | same integration test reports a Self revision and observes it in the session resident set |
| Memory + Self mixed query owner/final validation | NOT_RUN | no dedicated mixed-owner fixture was run |
| Memory candidate batch materialization | PASS | owner batch loader plus all 10 `query_correctness` tests, including 10,000-row tail-lane case |
| CognitiveSchema final current/lifecycle validation | NOT_RUN | implementation changed; dedicated concurrent revalidation fixture is pending |
| public Proto / Kernel / Core / official client Self surface | PASS | `corepack pnpm generate`, `corepack pnpm typecheck`, `cargo check -p nous-kernel` |

## Qualification commands

- `cargo test -p nous-kernel --test self_authority --no-fail-fast -- --test-threads=1` — PASS, 4/4; fixture teardown leaves 0 embedded PostgreSQL process and 0 exact temporary root.
- `just verify` — PASS; includes 10/10 `query_correctness` tests under the serial workspace test gate.
- `cargo check --workspace --all-targets --all-features` — PASS.
- `corepack pnpm check` — PASS (Buf lint, TypeScript check, Vitest 3 files/7 tests, formatting, Oxlint, Knip, dependency-cruiser, jscpd, Sherif).
- `just lint-maintainability` — PASS after excluding generated `nous-protocol` from authored production lint.
- `just dupes` — PASS with reviewed baseline 16 exact groups / 6.5%; no production directory was excluded.
- `just nextest` — PASS, 48/48 with `--test-threads 1`; no embedded PostgreSQL residual process/root after completion.
- `just feature-check` — PASS.
- `just dupehound` — PASS (12 duplicate clusters, grade A; advisory output reviewed).
- `just typos` — PASS.
- `just coverage` — PASS (summary generated; no coverage threshold is imposed by the Spec).
- `just mutants crates/self-domain/src/lib.rs` — PASS: 50 mutants caught, 2 unviable.
- `just osv` — FAIL: OSV reports transitive `lru 0.16.4` through `tantivy` and `paste 1.0.15` through `simba`/`nalgebra`; dependency chains were checked with `cargo tree -i`, and no compatibility/ignore exception was added.

## Authority schema layout

The fresh-schema migration set is organized by semantic ownership rather than calendar or milestone labels: foundation, memory/schema, runtime/identity/material, retrieval indexes, and Self Authority. Superseded ontology migrations and compatibility patches are removed. Serving projection inputs are invalidated in the same Self mutation transaction as the Authority sequence bump.

## Scope

Social Cognition、Motivation、Episode/Journal、自动人格学习、Heptalogos live integration 和新图算法未实现。无 `SPEC_CONFLICT`；存在上表所述 Seed format `SPEC_GAP`。
