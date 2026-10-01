# Nous Wave 当前实现架构

长期目标语义与跨系统合同由 Architecture-Vault 持有；本文只描述当前 checkout 的 owner、进程边界和依赖方向。

## 进程边界

TypeScript Core 负责公开 Connect/HTTP API、WorkContext/Projection/Managed Context、NousQL、模型编排和官方 Client 宿主。Rust Kernel 是 private loopback child process，负责 Subject、Runtime、Material、Memory、Persistence 和 Retrieval 的组合。Core 不直接访问 PostgreSQL。

## Rust owners

| Owner | 当前责任 |
| --- | --- |
| `crates/core` | typed IDs、exact references、query DTO、时间和共享语义错误 |
| `crates/protocol` | 从 `proto/` 生成的 Rust wire bindings |
| `crates/persistence` | PostgreSQL pool/transaction、fresh schema、receipts、projection watermarks 和 Authority-side queries |
| `crates/configuration` | typed registry、overrides、immutable snapshots、digest 和 capability provisioning |
| `crates/subject` | Subject identity、Memory capability、Cognitive Seed version/adoption |
| `crates/material` | Artifact、ObservationOccurrence、SourceRegion、DerivedRepresentation 和 materialization |
| `crates/memory` | Memory/CognitiveSchema/Episode/Tag/AssociationEvidence Authority、lifecycle、query contributors |
| `crates/runtime` | Session、ResidentSet、WorkContext、UseEvent、QueryPlan、lane/result contract 和 fixed fusion |
| `crates/retrieval` | lexical/dense Serving、immutable generations、provider adapters 和 explicit experimental topology |

## 主要数据流

```text
Official Client / Host
        ↓
TypeScript Core
        ↓ private authenticated loopback RPC
Rust Kernel
  ├─ Subject / Material / Memory Authority
  ├─ Runtime query and use state
  ├─ Persistence
  └─ rebuildable Retrieval generations
```

Observation 先写 Material Authority，再由 Memory owner 以 occurrence-bound EvidenceRef 形成 immutable revision。Authority commit 只发布 projection invalidation/watermark；lexical、dense、topology 和 runtime state 不成为认知真值。

Material 的 DerivedRepresentation 保存 ordered exact inputs（SourceRegion、DerivedRepresentation 或 DerivedRegion）、strategy 和 producer。Fresh schema 用 `derived_representation_inputs` 表保存不可变边，`representation_source_regions` 从当前输入图计算来源闭包。输入须已存在且同 Subject；禁止重复和自引用，已提交表示没有追加/改写输入的接口，因此不能形成回边。成功 derivation key 与结果同事务保存；同 key 重试复用已提交结果。没有 current consumer 的旧 singular-source scheduler、lease/attempt 与独立 derivation state 表已删除。

Core 的 public `DeriveMaterial` 返回实际表示链并保留部分成功结果。结构化表示保存 JSON payload 与 deterministic text projection；唯一 Zod schema 用于 SDK/raw provider request 和 local validation，其 digest 进入 invocation、ProducerSignature 与 derivation identity。Material owner 对已提交描述做 UTF-8 byte segmentation，返回稳定 DerivedRegion catalog；第二模型输出的 keys 映射成字段 supports，Kernel 在输入 DAG 内核验引用。public Material read/materialize 支持按字段 DerivedRegion 精确回读。

Memory revision 与 producer registry 同事务提交，批量 read 携带 producer reference。formation 使用调用方稳定 operation identity，先保存经过验证的 proposal，再提交 Authority；原配置/Prompt snapshot 保持操作连续性。CLI trace 通过 official Client 展开 exact derivation inputs。原始 audio/video gateway input 与 model rerank 已接通；新的 schema live 复核、External Resource continuation 和最终便携验收仍在施工。

Runtime 持有 QueryPlan、lane budgets、object-revision aggregation、fixed RRF 和 final result ordering。Retrieval 通过 Runtime-owned contract 提供 serving candidates，不被 Runtime 作为具体实现依赖。

## 当前边界

当前 executable scope 是 Memory-only Reference Profile、WorkContext continuity 和 Episode foundation。Self、Social、Motivation、Desired Condition、Journal、Offline Cognition 和 Heptalogos live integration 由 Vault 保留长期目标语义，但不在本 checkout 提供 owner、protocol 或 public capability claim。
