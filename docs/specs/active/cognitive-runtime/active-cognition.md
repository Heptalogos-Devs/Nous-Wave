# Active Cognition

状态：IMPLEMENTATION-AUTHORIZING

The executable Active Cognition view is:

`ResidentSet(session) ∪ WorkContextRefs(session.active_work_context) ∪ request.situation.current_refs`

References are deduplicated by canonical exact `CognitiveRef` while source family remains available for diagnostics. Current owner validation removes foreign, invalid, and purged refs. Active Cognition is a Runtime view, not a second durable Authority.

- retrieval hits alone do not add residency;
- meaningful UseEvent may update ResidentSet under the existing idempotent contract;
- WorkContext refs are runtime relevance input, not meaningful use;
- runtime query returns resident and active WorkContext candidates with source diagnostics;
- Projection/Managed Context reads active WorkContext refs through Kernel and revalidates them before exposure.
