# Self Domain Instructions

This crate owns Self Authority domain types and pure validation.

- Keep identity, revision, lifecycle, support, and exact-reference semantics here; do not add SQL or transport ownership.
- Preserve typed references and enum vocabulary from the current protocol/domain contract.
- Add tests for changed domain boundaries and observed invalid-input risks; do not invent Seed parser semantics while the active Spec `SPEC_GAP` remains open.
- Update `docs/reference/SELF.md` when the current public domain surface changes.
