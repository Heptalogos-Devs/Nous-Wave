# Active Cognition

## Owner

Runtime (crates/runtime) owns this Subject/Session view; Core Projection and Managed Context consume it.

The executable Active Cognition view is:

`ResidentSet(session) ∪ WorkContextRefs(session.active_work_context) ∪ request.situation.current_refs`

References are deduplicated by canonical exact `CognitiveRef` while source family remains available for diagnostics. Current owner validation removes foreign, invalid, and purged refs. Domain owners retain durable Authority; Runtime composes their exact references into the Active Cognition view.

- retrieval hits alone do not add residency;
- meaningful UseEvent may update ResidentSet under the existing idempotent contract;
- WorkContext refs are runtime relevance input, not meaningful use;
- runtime query returns resident and active WorkContext candidates with source diagnostics;
- Projection/Managed Context reads active WorkContext refs through Kernel and revalidates them before exposure.

[返回文档目录](../../INDEX.md)
