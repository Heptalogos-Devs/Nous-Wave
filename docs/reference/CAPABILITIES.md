# Capability Composition 当前参考

Kernel 当前组合持有 `ConfigurationService`，并按 Process Capability 构造 optional owner：`memory: Option<MemoryService>`、`self_cognition: Option<SelfService>`、`social: Option<SocialService>`。

默认 process/新 Subject capability 为：

```text
Memory=true
Self=false
Social=false
```

`CreateSubject` 的运行策略入口已收敛为 `metadata` 加 typed `SubjectCapabilities`；metadata 不承载 configuration settings。Subject capabilities 必须是 process capabilities 的子集，并写入 `subject_capabilities`。

Memory-only Subject 可以形成 Memory、Query、Serving 和恢复；未启用的 Self/Social 不产生 placeholder cognition，也不使 Memory query/Serving degraded。显式查询 disabled domain 返回 `Unavailable`。

当前公共 wire contract 在 `proto/nous/wave/v1alpha1/types.proto` 中声明 `SubjectCapabilities`、`CreateSubject.metadata` 和 `Subject.metadata`；Rust/TypeScript bindings 由 Buf 生成。
