# Self Service

`self-service` is the Self Authority persistence and query owner. It performs transactional Self Facet and Narrative Identity mutation, receipt/idempotency handling, support/provenance validation, lifecycle and purge, projection invalidation, `SelfDirect` contribution, Serving candidate filtering, and exact context materialization.

It does not own long-term Self ontology, public transport, generic Cognitive Runtime orchestration, or Serving authority. See [`docs/reference/SELF.md`](../../docs/reference/SELF.md) and the [current implementation architecture](../../docs/architecture/current-implementation.md).

Run the focused validation with `cargo test -p nous-kernel --test self_authority --no-fail-fast -- --test-threads=1`; use `just verify` at the authorized acceptance boundary.
