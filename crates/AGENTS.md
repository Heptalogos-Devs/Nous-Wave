# Rust Owners

[INDEX.md](INDEX.md) routes current owners. `persistence` owns database mechanics; `runtime` owns query/runtime semantics; `retrieval` owns rebuildable Serving. Shared contracts must not introduce a concrete retrieval dependency into `runtime`.
