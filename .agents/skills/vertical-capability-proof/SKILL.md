---
name: vertical-capability-proof
description: Use before claiming a current capability or milestone is closed, runnable, recoverable, or usable through the supported public surface.
---

# Vertical Capability Proof

## Failure mode

Internal service tests and mocks can all pass while the supported public path is missing configuration, transport wiring, query exposure, restart behavior, or lifecycle semantics.

## Procedure

1. State the exact capability claim in user-visible terms.
2. Start at the supported public entrypoint / official Client, not an internal service.
3. Exercise the minimum real path that would falsify the claim, including persistence/restart/lifecycle when those words are part of the claim.
4. If a necessary operation is only possible through an internal API, record a public contract gap instead of bypassing it.
5. Distinguish source-tree execution, platform, provider availability, and unavailable lanes.
6. Record a compact stage result and identifiers needed for trace-back.
7. Update current-state wording only to the proven boundary.

For the 2026-10-27 Memory Reference Profile, the primary proof includes public boot, Memory-only Subject, multi-session use, Observation→Memory, public Query recall, stable UseEvent retry, full process restart, lifecycle operations, Serving freshness, and provenance trace-back.
