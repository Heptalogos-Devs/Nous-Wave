# Nous Wave 当前实现架构

长期目标语义与跨系统合同由 Architecture-Vault 持有；本页只描述当前 checkout 的代码 owner 和运行边界。

## 进程边界

TypeScript Core 负责公开 API、Focus、Projection、Managed Context、NousQL、模型/资源编排和官方 Client 的宿主边界。Rust Kernel 负责私有 Connect/gRPC、Subject、Cognitive Runtime、Material、Memory、Authority Store 和 Serving owner 的组合。Core 不直接访问 Kernel PostgreSQL。

## Rust owners

| Owner                                     | 当前责任                                                                                                                                     |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/core`                             | typed IDs、exact references、时间范围、Query contracts、错误和 operation digest 基础。                                                       |
| `crates/subject-core`                     | Subject identity、Character Seed lineage 和 Subject authority sequence 读取。                                                                |
| `crates/material` / `material-service`    | Artifact、ObservationOccurrence、SourceRegion、DerivedRepresentation、材料化和上传 admission。                                               |
| `crates/memory-domain` / `memory-service` | Memory/Revision、EvidenceRef、CognitionDependency、CognitiveSchema、AssociationEvidence、lifecycle、Accessibility 和 R1 query contribution。 |
| `crates/cognitive-runtime`                | Session、ResidentSet、QueryPlan、UseEvent scoped idempotency、runtime checkpoint 和 workset。                                                |
| `crates/authority-store`                  | PostgreSQL migrations、mutation receipts、authority sequence、projection watermarks、reference validation 和 serving records。               |
| `crates/memory-retrieval` / `serving`     | lexical/dense/topology artifacts、fixed RRF support、Wave R1 bounded propagation 和 immutable rebuildable generations。                      |

## 主要流程

Observation 先写 Material Authority，再由 Memory owner 以 occurrence-bound EvidenceRef 形成 immutable Memory revision。Memory mutation 在 receipt、object epoch、authority sequence 和 projection watermark 的同一 Authority transaction 中提交；外部模型、embedding、reranker 和大对象读取位于 transaction 外。

Query 在 candidate generation 前建立固定 QueryPlan。Candidate identity 使用 exact revision；lane 内先聚合 view，再使用固定 RRF，最终由当前 Authority/lifecycle/accessibility 批量校验。Serving generation 只提供可重建候选来源，不拥有认知真值。

Context/Projection 仍属于 Core/Runtime；进入调用方上下文的呈现和后续 referenced/acted_on/result use 通过 UseEvent 独立记录。Purge 是 cognition-scope 的可恢复两阶段操作，不删除共享 source Authority。

## 当前边界

Self、Social Cognition、Motivation、Desired Condition、Episode/Journal 和 Heptalogos live integration 没有当前实现 owner。它们的目标语义不由本仓库的代码状态推断。
