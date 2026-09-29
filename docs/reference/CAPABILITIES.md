# Capability Composition 当前参考

当前 process/subject capability 只声明 Memory。Subject 创建时保存展开后的 typed capability set；默认配置变化不改写既有 Subject。

Memory-only Subject 可以形成 Memory、Query、Serving、UseEvent、restart/rebuild、suppression/restore 和 purge。未实现的长期认知领域不通过空表、占位对象或 disabled owner 模拟存在。

当前 wire contract 在 `proto/nous/wave/v1alpha1/types.proto` 的 `SubjectCapabilities` 中声明 `memory`；Rust/TypeScript bindings 由 Buf 生成。长期 Self/Social/Motivation 语义见 Architecture-Vault，不属于当前 executable capability。
