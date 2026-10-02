---
name: repository-topology-rebase
description: Change Nous Wave crates, packages, process owners or repeatedly crossed ownership boundaries.
---

# Owner Topology

Reconsider the affected owner graph when a change repeatedly crosses the same boundaries. Merge, rename or delete when this removes duplicated responsibility. Update workspace manifests, imports, process composition and `crates/INDEX.md` together.

Core owns host integration; Kernel composes semantic owners. Persistence owns database mechanics, Retrieval owns rebuildable Serving, and Runtime owns query contracts independently of concrete providers. Domain nouns and line counts do not determine packages.
