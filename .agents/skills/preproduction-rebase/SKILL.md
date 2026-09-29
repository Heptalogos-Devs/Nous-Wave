---
name: preproduction-rebase
description: Use when replacing an internal current API, schema, protocol shape, crate path, durable state shape, fixture identity, or executable identifier during PRE_PRODUCTION.
---

# PRE_PRODUCTION Rebase

## Failure mode

Coding Agents tend to preserve a recent internal shape as if it were an external compatibility contract, creating aliases, patch migrations, dual readers, stale tests, and two current truths.

## Procedure

1. Identify the current canonical shape and every current reader/writer/caller.
2. Check whether a real current external compatibility obligation exists. A previous commit, local database, old fixture, old Spec, or generated output is not one.
3. Replace current producers and consumers together.
4. For durable state, rewrite/squash the fresh baseline and reset project-owned development/test state when needed.
5. Delete old APIs, aliases, fallback parsers, compatibility migrations, old package paths, obsolete tests/fixtures, and stale current docs.
6. Search the maintained tree for old identifiers and routes.
7. Report only unresolved real consumers as `SPEC_GAP` / `SPEC_CONFLICT`; do not invent a bridge.

## Output

Record the replaced shape, actual compatibility obligations, deleted routes, state reset, and residue result. Verification commands come from the active Plan; this Skill does not create another gate list.
