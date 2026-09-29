# Nous Wave Repository Execution Contract

Nous Wave is PRE_PRODUCTION. Architecture-Vault owns long-term cognition semantics; this repository owns current executable implementation truth, active Plans/Specs, and verification evidence.

## Current authority

- Implement only the active Plan and the current executable Specs it names.
- The 2026-10-27 Memory Reference Profile remains the milestone until explicitly replaced.
- When code, tests, or old documents disagree with the active contract, treat them as migration inputs. Do not preserve them as authority.
- Report unresolved changes to Authority, identity/revision, lifecycle, transaction/concurrency, idempotency, purge, public protocol, Serving consistency, security, or algorithm defaults as `SPEC_GAP` or `SPEC_CONFLICT` and stop that local slice.

## PRE_PRODUCTION replacement default

Development history creates no compatibility obligation. When replacing a current internal API, schema, protocol, crate path, fixture, or durable shape, update current producers and consumers together and delete the old route in the same change.

Do not add aliases, dual readers, dual writers, fallback parsers, compatibility migrations, or permanent adapters unless a real current external obligation is documented in the active Plan.

## Structural responsibility

Reconsider topology when work repeatedly crosses the same owners, an internal adapter is required, or a third similar owner/flow would be added. Merge, rename, or delete directly when that lowers continuing cost. Domain nouns do not imply crates, services, processes, or tables.

Keep `runtime` independent of concrete retrieval/provider implementation. `persistence` supplies database mechanics; `retrieval` supplies rebuildable Serving mechanics; semantic owners retain mutation and lifecycle meaning.

## Product boundary

The current executable profile is Memory-only. Do not reintroduce Self, Social, Motivation, Episode, Journal, Offline Cognition, or Heptalogos behavior authority through code, protocol, configuration, tests, or docs. Their long-term semantics remain in Architecture-Vault.

## Generated and durable state

- Edit canonical `.proto` files and run `corepack pnpm generate`; never hand-edit generated bindings.
- Fresh schema lives in `crates/persistence/migrations/0001_foundation.sql` through `0004_indexes.sql`. Project-owned dev/test databases may be reset when a schema rebase requires it.
- Keep Cargo `target/` for incremental compilation. Do not traverse or clean `node_modules/`.
- Do not push, merge, deploy, or mutate external production state without explicit authorization.

## Governance economy

Persistent governance records recurring, project-specific executor failures. Do not add AGENTS rules, Skills, tests, validators, checklists, or gates for ordinary coding competence or one-time incidents. Keep README for purpose/shortest path, INDEX for navigation, AGENTS for agent behavior, active Plans for authorization, Specs for current contracts, and Qualification for executed evidence.

## Verification and claims

Use the narrowest useful check during iteration. At acceptance, run the active Plan's gates, including `corepack pnpm check`, `just verify`, and `corepack pnpm qualification:memory-reference` when the public Memory capability is in scope.

Use only `PASS`, `FAIL`, `NOT_RUN`, and `BLOCKED`. Internal tests do not prove a public capability; a capability claim requires the supported public entrypoint and official Client path. Do not turn one-platform evidence into cross-platform claims.

When an important boundary changes, update the relevant README/INDEX and current architecture/state documents. Keep AI-facing instructions concise technical English.
