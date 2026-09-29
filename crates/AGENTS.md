# Rust Crate Topology Instructions

A crate boundary must pay for itself through a stable semantic lifecycle, reusable mechanism, process/wire boundary, or meaningful dependency isolation. Ontology nouns and `Domain`/`Service` naming alone do not justify separate crates.

Current owners and dependencies are documented in [`INDEX.md`](INDEX.md). Keep `persistence` as database mechanics, `runtime` as query/Runtime semantics, and `retrieval` as rebuildable Serving mechanics. Runtime MUST NOT depend on concrete retrieval implementation merely to share contracts.

When a change repeatedly crosses crates, inspect the dependency graph before adding another interface. Merge, rename, or delete directly in PRE_PRODUCTION; remove old paths and re-export shims.
