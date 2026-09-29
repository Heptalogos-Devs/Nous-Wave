# Rust Crate Topology

`crates/` contains the Rust semantic and mechanism owners composed by the private Kernel.

crate boundary 只有在它提供稳定的 semantic lifecycle、可复用机制、process/wire boundary 或真正的 dependency isolation 时才成立。Ontology noun、`Domain`/`Service` 命名或文件数量本身不构成独立 crate 的理由；重复跨同一组边界时，优先重新检查 ownership 并直接合并或删除。

`persistence` 提供 PostgreSQL 机制，不拥有产品 Authority；`retrieval` 提供可重建 Serving 机制；`runtime` 拥有 QueryPlan、lane contract、fusion 和 Runtime state。`runtime` 不依赖 concrete retrieval implementation，retrieval 通过 Runtime contract 提供候选。

当前 crate 目录见 [`INDEX.md`](INDEX.md)。修改重要边界时同步更新该索引、Cargo workspace、Kernel composition 和当前实现文档。
