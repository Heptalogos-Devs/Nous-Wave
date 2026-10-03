# Nous Wave 当前实现架构

本文描述当前代码的进程边界、语义 owner 和主要数据流。长期认知语义与目标架构见 [Architecture-Vault Target Design](https://github.com/Heptalogos-Devs/Architecture-Vault/blob/main/docs/Nous-Wave/TARGET_DESIGN.md)。

## 进程边界

TypeScript Core 提供 Connect/HTTP API，启动和管理 private Rust Kernel，执行模型与外部 Resource 调用，并承载 WorkContext、Projection、Managed Context 和官方 Client 集成。Core 通过 authenticated loopback RPC 调用 Kernel；PostgreSQL 访问由 Kernel/Persistence 负责。

Kernel 组合 Subject、Configuration、Material、Memory、Runtime、Persistence 和 Retrieval owners。Authority 写入由对应领域 owner 验证并持久化；Serving generation 可从 Authority 重建。

## Rust owners

| Owner | 当前责任 |
| --- | --- |
| `crates/core` | typed IDs、exact references、query DTO、时间与共享语义错误 |
| `crates/protocol` | 从 `proto/` 生成的 Rust wire bindings |
| `crates/configuration` | Rust/Core unified Catalog、JSON Schema validation、overrides、immutable snapshots、digest 与 capability provisioning |
| `crates/persistence` | PostgreSQL transaction/pool、fresh schema、MutationEnvelope、receipts、projection watermarks |
| `crates/subject` | Subject identity、Memory capability、Cognitive Seed version/adoption |
| `crates/material` | Artifact、ObservationOccurrence、SourceRegion、DerivedRepresentation、DerivedRegion 与 materialization |
| `crates/memory` | Memory、CognitiveSchema、Episode、Journal、Tag、AssociationEvidence、provenance 与 lifecycle |
| `crates/runtime` | Session、ResidentSet、WorkContext、UseEvent、QueryPlan、lane/result contracts 与 fixed fusion |
| `crates/retrieval` | lexical/dense Serving、immutable generations、provider adapters 与 experimental topology |

## Transport 与 mutation

Kernel 在同一 authenticated loopback server 上承载 canonical generated `SubjectService`、`MaterialService`、`MemoryService`、`TopologyService`、`IdentityService`、`RuntimeService`、`ResourceRegistryService`、`ConfigurationService` 和 `SystemService`。Core 通过 generated ConnectRPC descriptor adapter 转发这些 owner 操作，保留调用方的 cancellation 与 deadline。官方 Client 的 `cognition` 和 `resources` namespace 组合相应 owner 与编排接口。

Core 的 `CognitionService` 承载 Query、GrantMaintenance、Projection 和 Managed Context 编排；`ResourceService` 承载外部 Resource materialization；`ModelService` 执行 formation、derivation 和 embedding。Core System capabilities 汇总 Kernel 与当前 model runtime 的状态。

私有 workflow services 按实际执行步骤分组：`KernelQueryService` 负责认知时间读取及 query prepare/finalize/release；`KernelModelWorkflowService` 管理 model retry snapshot/proposal/outcome；`KernelMaterialWorkflowService` 提供 derivation/embedding 输入与提交；`KernelMaintenanceService` 提供 needs 的 claim/plan/finish 和 longitudinal owner commit；`KernelProjectionService` 提供 contribution batch。`KernelConfigurationService` 提供 bootstrap/snapshot，`ArtifactStreamService` 提供流式 Artifact 传输。

Persistence 的 `MutationEnvelope` 持有 Subject/operation identity、可选 owner Subject lock、operation lock、canonical digest receipt、transaction 和 projection invalidation。owner 选择 invalidation families，envelope 为同一事务分配一个 Authority sequence，并在 commit 时发布合并的 watermarks 与 receipt。Replay 返回 receipt，由领域 owner 解码结果；未提交的事务回滚；Memory purge 使用 checkpoint 和 resume 保留分阶段执行。

Memory、Episode、CognitiveSchema 和 Journal 保留独立模型、typed tables、provenance、head/epoch fence 与 lifecycle SQL。Memory crate 的内部 read、mutation、lifecycle、partition 和 provenance 按领域责任组织；共享纯 epoch/transition 检查不决定领域操作。

## 数据流

```text
Official Client / Host
        ↓
TypeScript Core
        ↓ authenticated loopback RPC
Rust Kernel
  ├─ Subject / Material / Memory Authority
  ├─ Runtime query and use state
  ├─ PostgreSQL persistence
  └─ rebuildable lexical / dense / topology Serving
```

Observation 写入 Material Authority；Memory 使用 occurrence-bound EvidenceRef 形成 revision。Authority commit 发布 projection invalidation/watermark。领域 Authority 持有认知对象与 revision；索引、向量和拓扑 generation 提供可重建 Serving，Session 与 UseEvent 等运行状态由 Runtime 持有。

Core 解析 model profiles 与 Prompts，调用 embedding、rerank、media derivation 和 Resource adapters；Kernel 验证并保存 ProducerSignature、exact material inputs、Authority revisions 与 query state。External Resource records 与本地 Artifact/Observation 保持不同身份；被选用的外部内容通过 Material owner 形成新的 Observation。

## 当前边界

当前可供给的认知能力组合为 Memory-only。Episode authority 归 Memory；Runtime 自动组织 Session Experience；Memory 持有 Episode/Journal Authority；Core 提供有界维护机会、模型 refinement、Journal synthesis 和 Memory/Schema consolidation。具体合同见 [纵向认知](../specs/active/cognitive-runtime/longitudinal-cognition.md)。

Windows x64 portable assembly 使用 LLVM-MinGW UCRT 生成 shipping Kernel，并携带所需私有运行时 DLL。独立运行目录、配置、数据和第三方 runtime 布局见 [Runtime Bundle Spec](../specs/active/deployment/runtime-bundle.md)。

[返回文档目录](../INDEX.md)
