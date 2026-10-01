# Nous Wave 真实使用与检索

状态：ACTIVE IMPLEMENTATION AUTHORIZATION
日期：2026-09-30

## 目标与合同

以 official Client、真实公开材料、实际 gateway 和 NousQL 证明 Memory 形成、召回、来源追踪与重启复用。10/27 Memory Reference milestone 不变。长期语义服从 Architecture-Vault Target Design/Decisions。

本计划落实用户授权的 `Nous-Wave-Real-Usage-and-Retrieval-Execution-Spec-2026-09-30` 执行包。直接合同：

- [Gateway、模型、Prompt 与 provenance](../../specs/active/model-runtime/gateway-model-and-prompts.md)
- [Material derivation graph](../../specs/active/model-runtime/material-derivation.md)
- [Reference consumer](../../specs/active/model-runtime/reference-consumer.md)
- [Structured Material、支持链与 External Resource 续接](../../specs/active/model-runtime/cognitive-io-and-resources.md)
- [Runtime Bundle、逻辑路径与 runtime packs](../../specs/active/deployment/runtime-bundle.md)
- [Rerank 与真实评估](../../specs/active/retrieval/rerank-and-live-evaluation.md)
- [Memory Authority](../../specs/active/memory-reference-profile/01-memory-authority-provenance.md)、[Runtime/Use](../../specs/active/memory-reference-profile/02-runtime-use.md)、[Query/Serving](../../specs/active/memory-reference-profile/03-query-serving.md)、[Qualification](../../specs/active/memory-reference-profile/04-qualification.md)
- [WorkContext](../../specs/active/cognitive-runtime/work-context.md)、[Active Cognition](../../specs/active/cognitive-runtime/active-cognition.md)、[Episode Authority](../../specs/active/episode/episode-authority.md)、[Episode integration](../../specs/active/episode/episode-runtime-integration.md)

## 执行约束

Core 拥有外部模型协议与编排；Kernel 拥有 Authority、查询与 Serving。Runtime 不依赖 concrete Retrieval。标准协议为 openai-chat、openai-responses、openai-embeddings、openai-audio-transcription、rerank-v1；不建立 upstream provider zoo。Token 只来自 credential environment reference。角色策略独立，Prompt 为受限本地 UTF-8 资产（每份最多 128 KiB），digest 进入 ProducerSignature。

PRE_PRODUCTION 直接替换 current producer/consumer 和 fresh schema，不留旧路由、双读写、兼容迁移。模型输出只能作为提案；model rerank 在 Authority validation 后、最终用户 limit 前，score 不写回认知。公开实验不得预填候选、latency 或指标。

全部开发、实验、文档和 acceptance 留在从 `37e75b8382592fe09ff536dac5a14a84dee47902` 建立的 `feat/real-usage-retrieval`。一个完整 PR；Ready CI PASS 后不再修改分支，squash merge 并删除分支。live slice 缺凭据时保持 BLOCKED，允许其余验收完成后按如实记录集成。

## 施工顺序

2026-10-01 新执行包收敛当前分支：canonical Zod structured schema、schema digest、structured payload、DerivedRegion 字段支持链、provider-neutral Resource/RAGFlow、同一 Query ticket continuation，以及唯一 `just acceptance`。普通 runtime 移出研究调用次数硬限制；研究运行仍显式计数。本文旧硬预算措辞只适用于 research/qualification。现有 Draft PR #5 持有执行 checklist。

- [ ] 完成 richer structured schema → strict provider request → local semantic validation → payload/projection → stable field support refs。
- [ ] 完成 External Resource descriptor/adapter、RAGFlow、Query continuation 和 official Client materialize/Observation；真实 provider 缺环境记 BLOCKED。
- [ ] 移出 normal runtime 的 model-call hard guard；研究 composition 保留显式可恢复 guard。
- [ ] 完成 VCP current route/source audit；当前 experimental propagation 不声明完整 RiverMemo。完整 VCP、CognitiveClock/longitudinal 不在本分支施工。
- [ ] `just acceptance` 统一 generation/diff/check/verify/dupes/TS dupes/OSV/build/既有 qualification/新增 cognitive-io-resource qualification；CI 只准备固定工具与调用该命令。
- [ ] 删除 superseded 文档树，吸收仍有效语义，更新导航与当前证据。

2026-09-30 追加决定已经由用户批准：完整 QueryExpr/scoped constraints、bounded signed preferences、typed recent、caller-stable formation workflow、resolved-mention aboutness、独立 direct-structuring role、paid-call 前 derivation reservation、Core-only editable embedding profile，以及 source-less Windows x64 Runtime Bundle。下面的原任务继续执行，不建立第二轮计划。

- [ ] 完成独立 RuntimeLocations、统一配置、RunRoot discovery 和私有 runtime 生命周期；普通启动不下载，显式 runtime install 才 acquisition。
- [ ] 完成 whole-workflow replay、付费前 reserve/check、完整 DAG roots、独立 direct role 和 aboutness 三模式。
- [ ] 完成 source-less application/client closure、Node/PostgreSQL/LGPL-only FFmpeg packs、license/checksum/SPDX manifest；在仓库外使用任意 CWD 和无开发 PATH 验证。

- [x] 完整读取执行包、current 契约和 Vault；fetch 并检查基线变化；建立语义分支。
- [x] 安装 current Plan/Specs，更新 scope 与已执行 Linux CI evidence，撤回 fake research claim。
- [ ] 替换 `apps/nous-core/src/config.ts` 和 `src/model/runtime.ts`：GatewayProfile → ModelProfile → RoleBinding、显式 SDK clients、role readiness、受限 PromptRegistry；建立 `prompts/`。精准验证 URL/secret 边界、prompt escape/digest、损坏 model output。
- [ ] 修改 core/material/memory、canonical fresh SQL、public/kernel Proto 与所有 consumer：immutable ordered derivation inputs、DAG、ProducerSignature persistence、rich text selection、三种 derivation strategy；仅运行 `corepack pnpm generate` 生成 bindings。验证同主体、输入重复/cycle、provenance root 和无 binary TextDecoder 路径。
- [ ] 扩展 `packages/client/src/node.ts` streaming artifact upload；创建只依赖 official Client 的 `apps/nous-cli`，提供 discovery、subject/session、observe/upload、derive/form/embed、NousQL、trace、use、WorkContext 与 restart 复用。运行无付费的 public consumer smoke。
- [ ] 修改 Runtime/kernel query envelope 和 Core model material pipeline：bounded validated pool、稳定 textual intent、model rerank seam、response validation、requirement degradation、typed score/provenance diagnostics；验证最终 limit 与 malformed index。
- [ ] 删除 `apps/nous-core/research_retrieval.ts` 和预填 fixture；建立 official Client 真实 import/run/metrics runner、硬预算、ignored raw cache、tracked source/query manifests；逐条回读来源确认 oracle。
- [ ] 用用户提供的协议、模型与环境变量运行 text、embedding、rerank、image/audio/video、三策略比较与 controlled/end-to-end 四种 variants；缺 prerequisite 的局部记录 BLOCKED，缺 topology signal 记录 NOT_RUN。修复实际暴露问题，raw per-query results 保留在 ignored run directory。
- [ ] 更新 reference、README/INDEX、current architecture/state、Qualification 和 project-specific evaluation skill；只有长期语义改变时更新 Vault，实验完成后同步 mid-term-paper 事实。
- [ ] 完成本地 acceptance、审查完整 diff、建立 draft PR、Ready CI PASS、冻结分支、squash merge、删除分支并报告。

## 验收与报告

本地：`corepack pnpm generate` 后确认 bindings 无漂移；`corepack pnpm check`、`just verify`、`cargo build -p nous-kernel`、`corepack pnpm qualification:memory-reference`、`corepack pnpm qualification:cognitive-runtime-episode`、新增 `corepack pnpm qualification:real-consumer-local`。验证 Markdown links 和 `git diff --check`。迭代只运行能证伪当前改动的窄检查，不要求机械 TDD 或扩张测试数量。

live：`corepack pnpm qualification:live` 和 `corepack pnpm research:retrieval-live`，不进入付费 CI。真实 Recall@1/5/10、MRR、provenance precision、wrong-source/entity、stale leakage、empty rate、p50/p95、rerank/Wave 统计、formation coverage、conditional 与 end-to-end recall；latency/usage/cost 只记实际观测。

全部真实模型调用的合计硬上限为 10000 次，包括失败、warm-up 和复跑。使用实际 batch/reuse 降低重复开销；预算耗尽的 slice 明确 BLOCKED。最终产品 proof 必须使用仓库外的精确 Windows x64 ZIP：私有 Node/PostgreSQL/FFmpeg → official Client → real gateway/source → Memory/NousQL/trace/use → 完整停止及 restart。共置、分离 roots、安装位置搬移和缺包启动不联网均须验证。最终报告增加 bundle/runtime pack digest、路径 profile、license/SBOM 和 source-less evidence。

结果仅用 PASS、FAIL、NOT_RUN、BLOCKED。报告各 base HEAD、分支、PR、Ready run、squash commit、branch deletion、协议/模型（不含 token）、source/unit/query/media 数量、各 live slice、真实指标、被删除 shape、limitations 和 SPEC_GAP/SPEC_CONFLICT。

Journal、Dream、Self、Social、Motivation、automatic Episode segmentation、Heptalogos live integration、UI 和 Wave 默认启用均不在范围。完成授权行为与证据后停止。

## 2026-10-01 用户修订

使用 Apple、OpenAI 等公开真实文章，逐条回读来源确认 oracle。总调用上限改为 10000，包含失败与复跑，不要求耗尽。音频默认多模态理解，使用 material_description / material_direct_structuring 的 audio_input。video.input_mode=direct 为默认，frames 仅显式启用且本轮 NOT_RUN；audio.input_mode=transcription 保留标准 ASR 选项。直接媒体限定 openai-chat：input_audio {data,format} 与 video_url {url:data URI}；video_url 为网关内容扩展，不声称 OpenAI 原生标准能力。不做 provider zoo 或静默抽帧/转写 fallback。发送已上传 immutable Artifact bytes，配置声明须经实际调用验证。
