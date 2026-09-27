# Self Service Instructions

This crate owns Self Authority persistence and the Self query contributor.

- Keep canonical Self mutations, receipts, fencing, lifecycle, purge, dependency invalidation, and projection invalidation inside the Authority transaction.
- Serving remains a rebuildable projection; retrieval or context materialization MUST NOT create meaningful UseEvent state.
- Validate supports and exact revision references against the subject owner before persistence. Do not add legacy ontology, fallback reads, or compatibility adapters without an explicit current obligation.
- Keep query/context code behind `SelfService`; do not move generic query orchestration or transport semantics into this crate.
- Run the focused serial `self_authority` integration test after changes, then `just verify` at acceptance.
