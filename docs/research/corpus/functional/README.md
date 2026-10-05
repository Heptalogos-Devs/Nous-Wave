# Functional cognition corpus

Original CC0 fictional material: three coherent scenarios, 22 distinct events and 15 prepared query intents. This is the natural quality track for PR #15; the adjacent `cognitive` corpus remains a separate host-explicit structural fixture.

`scenarios.json` fixes entities, aliases, independent sessions, occurrence/observation time, source text, work purposes and maintenance expectations. Expected concepts and relations are inspection oracles, never Authority inputs. The two-signature deployment gate is a private concept. The scenarios include policy replacement, recurring preference evolution, identity ambiguity, a one-off joke, distinct readiness concepts, same-concept aliases and a cross-system resource-lifecycle analogy.

`queries.json` separates a human `surface_example`, canonical third-person `prepared_query` intent and an `oracle`. Fixture Entity/WorkContext/event keys bind to exact owner receipts. Concept keys bind only after inspecting automatically produced Tags. Failed concept formation remains a maintenance failure; the runner must not repair it by injecting an expected Tag. Surface examples and oracles never enter algorithm input. Canonical intent contains the question, with explicit context, rather than a target answer. The raw text compatibility track has its own selected external/text cases.

Multi-hop oracles require actual explained supported paths; a direct lexical hit does not establish that the multi-hop behavior worked. Missing or unsupported paths remain failures. Merge/split expectations are conditional on actual early concept formation; a coherent center from the outset does not require an unnecessary lineage operation. Owner merge/split contracts are verified separately.

Execute formation and automatic maintenance first. Inspect Tag reuse, fragmentation, over-merging, relation support, broad centers and source grounding before interpreting retrieval scores. Compare the same prepared inputs using text baseline, Nous native, VCP DTSC and VCP RiverMemo, with rerank forbidden. Record actual seed/path/relation/support/hop/readout for associative successes.

Run preflight `--plan` before providers or Serving construction. The runner requires provider, new embedding item, rerank, new generation, artifact byte and runtime budgets; default provider/rerank budgets are zero. Keep generated receipts, model outputs, vectors and results under ignored `data/research`. No full external dataset or repeated embedding belongs in this workflow.

Status: fixture authored; natural execution and quality acceptance pending. No benchmark result is claimed by this directory.

Formation preflight is executable:

```sh
pnpm exec tsx scripts/research/functional-plan.ts --plan
```

It accepts all six `--max-*` limits and `--query-ids`. Formation preflight reports zero embedding/rerank/Serving work because this phase only forms and inspects cognition. Model-dependent output/disk forecasts are explicit upper bounds. Retrieval requires a second preflight over the actual frozen representations and cache inventory. The common run ledger reserves work before dispatch and survives restart; the research gateway applies provider, embedding-item and rerank accounting to real wire attempts and aborts in-flight requests at the runtime deadline. Owner execution and its disk/Serving accounting are still being connected; this preflight does not claim natural execution has passed.
