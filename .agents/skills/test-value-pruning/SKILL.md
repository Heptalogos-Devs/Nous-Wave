---
name: test-value-pruning
description: Use when a refactor, milestone closure, or growing test suite has overlapping integration scenarios, slow/noisy gates, or tests coupled to obsolete implementation shape.
---

# Test Value Pruning

## Failure mode

Coding Agents naturally add tests and rarely remove them. Test count then becomes a proxy for confidence even when several tests prove the same path or only preserve an obsolete structure.

## Procedure

1. Inventory affected tests by the unique current contract, observed risk, or uncertainty they can falsify.
2. Mark overlap: which stronger test or public scenario already covers the same failure mode?
3. Mark cost: database/process startup, fixture maintenance, runtime, flakiness, duplicated helper surface, and coupling to private structure.
4. Delete tests with no unique current information value.
5. Move performance/scale/algorithm comparisons out of the default correctness runner.
6. Prefer one public milestone acceptance scenario plus small owner-focused regressions over several all-in-one integration tests.
7. Remove test-only production seams and helpers that lost their only consumer.
8. Re-read the milestone claim after pruning; add a test only for a concrete remaining proof gap.

Keep four classes only: pure unit, focused semantic integration, public milestone acceptance, and research/benchmark. Do not create coverage targets or test-count goals.
