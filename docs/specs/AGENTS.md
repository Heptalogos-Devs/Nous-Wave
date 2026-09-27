# Executable Spec Instructions

This scope contains implementation-facing contracts derived from the approved design and active plans.

- Preserve the authority order documented in `README.md`; an active Spec does not replace Architecture-Vault or scoped repository instructions.
- Do not silently weaken or reinterpret identity, revision, lifecycle, transaction, concurrency, idempotency, purge, query, Serving, or algorithm-default semantics.
- Keep direct Spec text stable while it is the active contract. If code and Spec disagree, report the implementation gap or `SPEC_CONFLICT`; do not edit the contract merely to make verification green.
- Keep `README.md` as the collection entry point and link each active set from `docs/INDEX.md`.
- Report every qualification command as `PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED` with the exact command and boundary.
