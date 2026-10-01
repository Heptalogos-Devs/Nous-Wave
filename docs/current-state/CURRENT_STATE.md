# Nous Wave 当前状态

当前唯一新增代码施工授权是 [Real Usage and Retrieval](../plans/active/2026-09-30-real-usage-retrieval.md)。10/27 milestone 仍是阶段目标；当前实现以 Memory Reference、Cognitive Runtime 和 Episode Specs 为直接合同。

## Capability matrix

| Capability | State | Evidence |
| --- | --- | --- |
| Public dev boot | PASS | `corepack pnpm dev` printed Core endpoint and discovery record during this checkout |
| Memory-only Subject | PASS | `corepack pnpm qualification:memory-reference` |
| Observation → grounded Memory | PASS | public qualification path |
| Public Query/Serving recall | PASS | public qualification path |
| Two-session Runtime isolation | PASS | public qualification path; focused `runtime_residency` remains current regression |
| UseEvent idempotency | PASS | public qualification path; focused query/runtime regressions |
| Process restart and same Authority revision | PASS | public qualification path; `process_restart` focused test |
| Suppression/restore | PASS | public qualification path |
| Purge and shared source retention | PASS | public qualification path; `purge_shared` focused test |
| Provenance trace-back | PASS | public qualification path |
| Structured Material payload / field supports | PASS | Windows qualification:real-consumer-local；deterministic local provider；含 stable segmentation、精确 materialize、无额外调用 replay |
| External Resource continuation / selected Observation | PASS | Windows official Client deterministic adapter qualification；query 不产生 Memory，选用后形成 Observation，同 operation 无额外 provider request |
| Optional live RAGFlow | BLOCKED | 用户确认尚未配置实例/dataset；不属于默认启动或默认验收依赖 |
| WorkContext create/pause/resume/end and cross-session continuation | PASS | corepack pnpm qualification:cognitive-runtime-episode |
| Episode identity/revision/history, overlap/hierarchy, lifecycle and purge | PASS | corepack pnpm qualification:cognitive-runtime-episode |
| Self/Social/Motivation executable slices | NOT_RUN | outside current checkout; long-term semantics remain in Architecture-Vault |
| Linux Ubuntu Ready CI | PASS | previous acceptance run 36573357286; exact commit/platform recorded in Qualification |
| macOS / broader platform qualification | NOT_RUN | no executed evidence |

## Current verification evidence

- `corepack pnpm typecheck` — PASS.
- `corepack pnpm qualification:memory-reference` — PASS; output includes `restart=true`, `suppress_restore=true`, `purge=true`, `traceback=true`.
- `corepack pnpm qualification:official-client` — PASS; legacy write/use smoke retained as a narrower diagnostic.
- `cargo check --workspace --lib` — PASS.
- `cargo test -p nous-kernel --test purge_shared --all-features -- --test-threads=1` — PASS.
- `cargo test -p nous-kernel --test runtime_residency --all-features -- --test-threads=1` — PASS.
- `corepack pnpm check` — PASS.
- `just verify` — PASS.
- `corepack pnpm qualification:cognitive-runtime-episode` — PASS; output included restart, cross-session continuation, Episode revision, overlap, hierarchy, lifecycle and purge evidence.
- `corepack pnpm research:retrieval` — NOT_RUN for actual retrieval: the previous runner aggregated pre-filled fixture candidates and latency; it provides no real algorithm or semantic recall evidence.

本轮 Windows official Client 路径的结构化表示、字段支持链、流式上传、trace、meaningful use、restart 与 Resource evidence 均有 public proof。candidate-26 的真实 gateway 检索实验使用6 sources/105 units/40 queries，controlled 与 end-to-end 均105个 Memory；每轨固定 Authority，baseline/model-rerank 共160条查询 complete。用户回听确认旧 NASA 音频为近似的底噪/提示音，已移入负例记录；主要音频换为两段署名授权音乐，三策略6个流程中5个提交成功、2/2召回 PASS，但一项乐器信息遗漏和一项被拒绝的音频视觉幻觉使媒体质量保持 FAIL。最终完整 acceptance/Ready CI 为 NOT_RUN；live RAGFlow 为 BLOCKED。精确 ZIP、指标、原始失败与许可范围见 [本轮 Qualification](../qualification/2026-09-30-real-usage-retrieval.md)。

精确命令、commit、平台和失败/未运行项由 [Memory Reference Qualification](../qualification/2026-10-memory-reference-profile.md) 维护；本文只保留 capability summary。
