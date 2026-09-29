---
name: repository-topology-rebase
description: Use when adding, removing, merging, renaming, or repeatedly crossing Rust crates/packages/process owners in Nous-Wave.
---

# Repository Topology Rebase

## Failure mode

Coding Agents usually accept the existing package graph and route new work through it, even when the graph is a development-history artifact. They also tend to create a new package for a new ontology noun.

## Procedure

1. Draw the current affected dependency/ownership subgraph.
2. For each package, state the independent value of its boundary: stable semantic lifecycle, reusable mechanism, process/wire boundary, or meaningful dependency isolation.
3. Identify splits that still share persistence, transaction, query, and release concerns and therefore add coordination without isolation.
4. Choose the lower continuing-cost shape; merge/delete/rename directly in PRE_PRODUCTION.
5. Update workspace manifests, imports, process composition, protocol ownership, current docs, and `crates/INDEX.md` together.
6. Remove old crate paths and re-export compatibility shims.
7. Inspect dependency direction after the change; semantic runtime contracts MUST NOT depend on concrete retrieval/provider implementation merely to share types.

Do not use line count or package count as the deciding metric. Ask whether each remaining boundary reduces future change cost and semantic confusion.
