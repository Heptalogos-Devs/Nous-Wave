# Capability Composition 当前参考

当前 process/Subject capability set 为 Memory。Subject 创建时保存展开后的 typed capability set；默认配置变化不改写既有 Subject。

Memory-only Subject 提供 Memory formation/query/Serving、UseEvent、restart/rebuild、suppression/restore 和 purge。

当前 wire contract 在 `proto/nous/wave/v1alpha1/types.proto` 的 `SubjectCapabilities` 中声明 `memory`；Rust/TypeScript bindings 由 Buf 生成。Self/Social/Motivation 的长期语义见 Architecture-Vault。

[返回文档目录](../INDEX.md)
