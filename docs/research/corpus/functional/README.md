# Functional cognition corpus

Original CC0 fictional material: three coherent scenarios, 22 distinct events and 15 prepared query intents. This is the natural quality track for PR #15; the adjacent `cognitive` corpus remains a separate host-explicit structural fixture.

`scenarios.json` fixes entities, aliases, independent sessions, occurrence/observation time, source text, work purposes and maintenance expectations. Expected concepts and relations are inspection oracles, never Authority inputs. The two-signature deployment gate is a private concept. The scenarios include policy replacement, recurring preference evolution, identity ambiguity, a one-off joke, distinct readiness concepts, same-concept aliases and a cross-system resource-lifecycle analogy.

`queries.json` separates a human `surface_example`, canonical third-person `prepared_query` intent and an `oracle`. Fixture Entity/WorkContext/event keys bind to exact owner receipts. Concept keys bind only after inspecting automatically produced Tags. Failed concept formation remains a maintenance failure; the runner must not repair it by injecting an expected Tag. Surface examples and oracles never enter algorithm input. Canonical intent contains the question, with explicit context, rather than a target answer. The raw text compatibility track has its own selected external/text cases.

Multi-hop oracles require actual explained supported paths; a direct lexical hit does not establish that the multi-hop behavior worked. Missing or unsupported paths remain failures. Merge/split expectations are conditional on actual early concept formation; a coherent center from the outset does not require an unnecessary lineage operation. Owner merge/split contracts are verified separately.

Execute formation and automatic maintenance first. Inspect Tag reuse, fragmentation, over-merging, relation support, broad centers and source grounding before interpreting retrieval scores. Compare the same prepared inputs using text baseline, Nous native, VCP DTSC and VCP RiverMemo, with rerank forbidden. Record actual seed/path/relation/support/hop/readout for associative successes.

Run preflight `--plan` before providers or Serving construction. The runner requires provider, new embedding item, rerank, new generation, artifact byte and runtime budgets; default provider/rerank budgets are zero. Keep generated receipts, model outputs, vectors and results under ignored `data/research`. No full external dataset or repeated embedding belongs in this workflow.

Status: fixture authored; natural execution and quality acceptance pending. No benchmark result is claimed by this directory.

Formation preflight and owner execution are executable:

```sh
pnpm exec tsx scripts/research/functional.ts --plan --root data/research/runs/functional-cognition-v1
```

It accepts all six `--max-*` limits and `--query-ids`. Formation preflight reports zero embedding/rerank work. Consolidation needs a shared lexical catalog to retrieve existing continuing claims. Its preflight estimates a bounded refresh per event plus startup, and reports the reason; dense/topology wait until query validation. Model-dependent output/disk forecasts are explicit upper bounds. Retrieval requires a second preflight over the actual frozen representations and cache inventory. The common run ledger reserves work before dispatch and survives restart; the research gateway applies provider, embedding-item and rerank accounting to real wire attempts and aborts in-flight requests at the runtime deadline. Build the local owner host with `cargo build -p nous-kernel --example functional-runtime`, then invoke the same command without `--plan`. The exact corpus/config/selection/limit identity must match the saved plan. Use a distinct `--execution-id` for a deliberately new bounded attempt; stop reasons and old counters remain in their ledgers. Optional `--config` and `--env-file` select locally stored model settings and credentials. Only episode, journal, consolidation and topology roles are enabled in this phase; paid attempts run through the budget gateway. The host reuses the installed PostgreSQL runtime and preserves run-owned Authority data. Resume uses durable observation receipts and stable model gateway ports. The observation identity derives from source events, independent of query-oracle edits.

Disk is measured at startup and after each operation, with a startup reservation of 128 MiB and per-operation reservation of 32 MiB. The ledger compares new run bytes to the pre-start baseline; database/Serving bytes, generation/retired counts, research-wide delta and largest run paths are reported. A model proposal failure stops the run and flushes state, inspection and budget results before further calls. Source quality remains pending natural inspection; a zero-provider host run is only an execution check.

Provider-free Prepared readout is executable with `scripts/research/prepared.ts --input <closed-queries.json> --root <existing-run> --plan`, followed by the identical command without `--plan`. Each input contains key, exact as_of and the canonical CognitiveQuery. Every query is bound once before any arm executes; baseline/native use `BoundQuery.for_profile` overlays. The host retains those frozen queries in memory and drops each read lease after capturing its result. This structural arm requires embedding/rerank forbidden; it does not stand in for the later VCP comparison with a shared embedding cache. Malformed native commands return a structured error and retain the runtime. Generation accounting starts before runtime initialization, so startup builds are included.

After a specific implementation defect is corrected, `--review-need-ids` requests owner review of those exact retained need IDs for the single selected Subject. It is included in preflight identity and preserves old receipts and semantic data. The host reads the stored scope and calls the normal maintenance enqueue owner; it does not rewrite Authority or model output. Leased or missing needs are rejected. This is an explicit recovery action, not an automatic retry of every invalid proposal.

For semantic arms, run `embedding-plan.ts --plan --config <local-config> --input <closed-queries.json> --root <run> --cache <run-cache.json> --output <embedding-plan.json>` first. It binds the actual representations, inventories owner source material and deduplicates exact texts against the cache. The report includes exact misses and configured batch count. `embedding-cache.ts --plan --input <embedding-plan.json> --cache <run-cache.json> --config <local-config>` adds the six explicit execution limits; run the identical command without `--plan` to fill only missing items. It uses the production ModelInvocations embedding adapter, one configured batch per reservation, with SDK retries disabled. Cache identity binds the canonical space and producer; each completed batch is saved atomically.

Re-run exact cache preflight and require zero misses. Then `prepared.ts --embedding-cache <run-cache.json> --profiles baseline-rrf,nous-node-potential-v1,vcp-dtsc-v9.2.1-adapter-v1,vcp-rivermemo-v3.1-adapter-v1` supports the four semantic arms, still with provider/rerank budgets zero and a saved matching `--plan`. Cache source vectors are installed through the material owner; every algorithm arm receives the same frozen representation vector via the normal query-material scope. Neither an arm nor a profile switch generates embeddings. These commands are bounded execution tools; acceptance still requires the full selected Functional corpus and inspection of source grounding and explained paths.

`--operation-timeout-seconds` controls the finite per-grant deadline and is part of formation preflight identity; the client deadline adds a small response allowance. The total runtime hard cap always remains in force.


Export actual owner bindings without provider work:

```sh
pnpm exec tsx scripts/research/functional-input.ts --root data/research/runs/functional-cognition-v1 --output data/research/runs/functional-cognition-v1/prepared-input.json
```

The exporter reads the durable formation state and latest inspection, checks the source digest, resolves Entity and WorkContext receipts, and binds exact event anchors to the unique smallest Episode scope. `--query-ids` selects intents. Optional `--concept-bindings` contains inspected `scenario:concept` → active Tag ID choices; the exporter verifies each choice against the same Subject's actual catalog. A composed Episode may retain the same event as its atomic source; equal-specificity alternatives remain ambiguous. Missing/ambiguous bindings are listed as `unresolved` and excluded from executable queries. Record these as formation/representation failures, rather than counting a smaller resolved subset as full-suite success. Outputs contain canonical question text and owner bindings; surface examples and oracles remain in the corpus. Run exact embedding preflight over this file before cache filling and shared-profile Prepared execution.


The owner runner first grants zero-model experience formation, then runs normal model opportunities at due opportunities before the corpus query time, persisting the opportunity clock across resumes so delayed Journal/Memory needs compete at their normal priorities rather than leaving only immediate Tag work eligible. Grant leases cover the requested execution envelope plus bounded workflow release and acknowledgement; timeout still stops model work and records a normal retry disposition.


Query intents use 2026-09-16 19:00 UTC as their as-of, leaving a bounded interval for delayed maintenance after deterministic formation. The runner persists each Subject's opportunity instant, advances to a higher-priority future due need when appropriate, and stops at the query horizon. Resumes preserve the simulation clock and source receipts; maintenance delays and owner queue priorities stay authoritative.


Prepared readouts retain a separate `candidatePool` and pass through the same owner retention/finalization used by public queries, including final revision validation, `ResultNeed.limit` and final ranks. Profiles run outside the query loop so a compatible Subject generation serves multiple frozen questions before switching profiles. Each run captures its owner inspection, including Journal sources and Schema evidence links.

`functional-grade.ts --result <run.result.json> --receipts <formation-state.json> --output <ignored-grade.json>` grades all 15 authored intents for each requested profile, including absent readouts. It follows actual occurrence/Episode/Journal/Memory/Schema/Tag support paths, checks source coverage, forbidden/wrong-scenario sources, final limits/ranks and contiguous positive activated paths with provenance. These are structural observations; semantic relevance and concept correctness remain a separate manual review. A numerical VCP path score alone does not establish a supported cognitive path.


Embedding preflight itself uses the shared six-limit execution ledger, stored next to its `--output`. It reserves startup disk and a possible generation refresh per queried Subject before opening the host, observes actual startup generations/disk, and reserves the report bytes before writing. A zero generation allowance stops before host launch. Reuse an existing exact vector cache; preflight adds no embeddings. A stopped or expired preflight ledger remains stopped, so a deliberate new bounded attempt uses a new output path.
