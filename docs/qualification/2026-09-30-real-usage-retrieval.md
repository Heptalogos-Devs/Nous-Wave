# 真实使用与检索 Qualification

计划：[Real Usage and Retrieval](../plans/active/2026-09-30-real-usage-retrieval.md)

## 基线

Nous-Wave base：`37e75b8382592fe09ff536dac5a14a84dee47902`；分支：`feat/real-usage-retrieval`。
Architecture-Vault base：`2614d65e8775c1f9652f44d69737ac7c57afd0d4`；Heptalogos 参考：`7bf22136c7bcc97953dd5c719e6671ca01e64eaf`。
mid-term-paper 本地：`877c392de175b93640cad9d39cf0de985bd64b8f`；fetch 后 default：`746b513a38b9297764b3e35dcb00deb082da98c0`。已审查新增 commit 列表与 diff summary；实际修改前须完整读取对应最新稿件与 scoped instructions。

此前 Linux Ubuntu acceptance：[run 36573357286](https://github.com/Heptalogos-Devs/Nous-Wave/actions/runs/36573357286)，head `c286721dfb84cb70a54fda916fe8992aefc9678a`，PASS。它不证明本分支后续改动。

## 本轮证据

| Slice | Result | Reason |
| --- | --- | --- |
| Deterministic implementation acceptance | NOT_RUN | implementation in progress |
| Material input graph persistence | PASS | focused PostgreSQL integration: two-stage roots, successful-result reuse, producer hash ownership, duplicate/self-cycle/missing-input rejection; public multi-strategy/model formation remains pending |
| Public text derivation / partial result / Memory producer trace | PASS | official Client local smoke: verified decoding, retain description on unavailable structuring role, exact representation support, producer registry read and restart; no live model claim |
| Official Client reference consumer local wiring | PASS | Windows public Core main entrypoint, CLI discovery, subject/session, text observation, streamed file upload, deterministic Memory, NousQL, trace, meaningful use and restart; model derivation and full producer trace still pending |
| Live gateway protocols / formation / embedding / rerank | NOT_RUN | gateway/model configuration and credential reference supplied; actual invocation and complete workflow pending |
| Image / video live derivation | NOT_RUN | configured roles and LGPL-only FFmpeg available; real source/model path pending |
| Audio live derivation | BLOCKED | speech identifier remains unset |
| Real corpus import / controlled / end-to-end metrics | NOT_RUN | runner and corpus acquisition pending |
| Wave real contribution | NOT_RUN | real sourced topology signal not yet established |
| This branch Ready CI / PR / squash / deletion | NOT_RUN | acceptance pending |

Observed live source/unit/query/media counts：0/0/0/0；latency、usage、cost 和 retrieval metrics 尚无实际观测，不填估计值。

Windows 当前迭代证据：`corepack pnpm typecheck` PASS；`corepack pnpm exec vitest run apps/nous-core/tests/model-boundaries.test.ts apps/nous-core/tests/runtime.test.ts` PASS。协议 fixture 验证显式 `/v1/chat/completions`、`/v1/embeddings`、`/v1/rerank` 路径、producer prompt digest、unsafe destination、Prompt path/size/UTF-8 边界和非法 rerank index。它不证明 live provider conformance。`ai@7.0.102` 配套 `@ai-sdk/openai@4.0.67`，共享 provider 4.0.15 / utils 5.0.41。

`cargo build -p nous-kernel` PASS（Windows，linker stdout warning）；`corepack pnpm qualification:real-consumer-local` PASS：Subject `01a0f199-1130-7de0-bb9a-09721ab12efa`、Memory `01a0f199-1664-7812-8586-a210652e6f59`、revision `01a0f199-1664-7812-8586-a2209ab992c1`，`publicBoot/cli/upload/trace/meaningfulUse/restart=true`。该 corpus 明确为 synthetic local wiring，liveModel=NOT_RUN。首轮 smoke 暴露 CLI consumer ref 少一层 namespace，改为 `consumer:nous-cli:default` 后重跑 PASS；没有绕过 Runtime validator。

上传配置收敛后重新 build / public smoke PASS：Subject `01a0f1a1-9b95-7382-ba9e-489e454c6598`、Memory `01a0f1a1-9ff8-7e92-a36c-e797d6f3afc8`、revision `01a0f1a1-9ff8-7e92-a36c-e7ad1d7ab34a`。测试从 public API 读取 Kernel 配置的 1 MiB，并拒绝 1 MiB + 1 byte 上传；Core/Client 独立的 8 GiB cap 和 Core TOML duplicate 已删除。`corepack pnpm proto:check`、typecheck、Knip、窄 Oxlint 与模型/上传 fixture 均 PASS。Rerank parser 已移除 deprecated `passthrough()`，只保留经过校验的字段。

SPEC_GAP / SPEC_CONFLICT：目前未报告未决语义冲突。附件本轮决定已安装为 current Specs；缺 live 配置是 prerequisite blocker。

Material fresh schema rebase 后：`cargo check -p nous-kernel`、TS typecheck、Buf lint、model-boundaries fixture 与 `git diff --check` PASS；`cargo test -p nous-kernel --test material_derivation -- --test-threads=1` PASS。更新 schema 的 Windows public local consumer smoke PASS（Subject `01a0f1ad-cf94-7520-bc2f-9a315ae64f30`、Memory `01a0f1ad-d41b-7090-8ff2-348326bb4946`、revision `01a0f1ad-d41b-7090-8ff2-3496311d6a07`）。该 smoke 仍为 synthetic wiring，不证明实际模型或媒体。旧 singular source 表示字段、Material Embedding enum、未被调用的 scheduler/attempt API 与独立状态表已删除；保留 source-region coverage，因为 Observation 仍是它的真实 producer。

公开 derive/producer rebase 的 Windows local smoke PASS：Subject `01a0f1d5-07b6-7403-8a24-0a6db69c3711`、Memory `01a0f1d5-0bec-79a0-8819-832427aec216`、revision `01a0f1d5-0bec-79a0-8819-8330682767cc`。实际验证 ExtractedText producer、不可用 structuring role 的 partial chain、Memory producer id/hash/read、exact derived support、CLI transitive trace 与 restart。`corepack pnpm check` PASS；`cargo check -p nous-kernel --tests` PASS；Material graph regression 重跑 PASS。完整 `just verify`、Memory Reference 和 Episode acceptance 尚未在最终分支运行。Shear 暴露已删除 scheduler 留下的 `async-trait`，已移除该 dependency。

Kernel launch 现在从 child environment 排除配置引用的 gateway credential variables，保留 Core 原值；case-insensitive name boundary 的窄测试 PASS。Kernel 不接收 app gateway token；其独立 RPC token 仍通过 parent-owned stdin 提供。此修复不涉及 upstream credentials 或外部 gateway administration。

## 2026-10-01 路径与便携 candidate

当前 working-tree candidate 仍位于 `feat/real-usage-retrieval`，不是最终 acceptance。用户配置已迁移到 ignored `data/dev/config/nous.toml`，credential 位于 ignored SecretRoot。RuntimeLocations、RunRoot discovery、稳定 instance identity/private PostgreSQL port 与独立 direct-structuring role 已接通。

`corepack pnpm qualification:real-consumer-local` PASS：Subject `01a0f2d8-43f9-72c0-8e4b-5031ad579f61`，Memory `01a0f2d8-4736-7260-b348-4bd88a8a77b1`。新路径仍为 synthetic wiring，不计入真实 corpus。

`corepack pnpm qualification:portable-local --bundle data/releases/candidate-07` PASS（Windows x64）：复制到仓库外，使用 private Node、仅 System32 PATH、分离 instance home，通过 bundle 内 official Client/CLI 完成 manual Memory、NousQL、trace、meaningful use 和 restart；private database port 保持不变。Subject `01a0f30f-7306-7f23-9590-de37e99fd8c2`，Memory `01a0f30f-752c-7390-8b74-7f4a7d20b872`。liveModel=NOT_RUN；corpus=synthetic_portable_wiring。此 candidate 早于后续 workflow/query/provenance 改动，不能外推为最终产品证据。

Private runtime packs的实际校验：Node 24.20.0、PostgreSQL 18.6.0、FFmpeg 9.0.2。FFmpeg 从官方 source SHA256 `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e` 构建；实际 Windows binary声明LGPL并禁用GPL/nonfree/autodetect。运行时install/list/verify PASS；普通serve不调用下载。最终native closure/license audit和完整SPDX仍须在最终candidate完成。

Material graph regression再次PASS，并新增实际SQL/Memory provenance summary断言：同源两阶段只有一个root，多输入图展开两个独立source roots。调用预算精准测试PASS：并发reservation保持cap，重启不清零。该阶段尚未执行真实模型调用；后续真实调用见下节。

## 2026-10-01 ongoing real gateway evidence

User increased maximum to 10000 total attempts; defaults and both configured ignored instances updated without changing model identifiers or secrets. Current live ledger count: 14, including the 3 failed transport attempts. Counts come from the durable budget ledger, not CLI RPC count. Completed formation replay did not charge a second call.

PASS: actual PostgreSQL license observation → doubao-seed-2.0-mini formation (4166.7457 ms, input470/output407/total877) → doubao-embedding-vision vectors → public NousQL recall → provenance trace. This is one real source/unit/query smoke, not the formal experiment. Cost unknown.

PASS: direct-media encoding/response/usage boundary checks, budget restart/concurrency checks, TypeScript typecheck/lint, Kernel build. These internal checks do not claim live media capability.

PASS: gateway restored after Docker interruption. Official Client direct audio committed AudioDescription 01a0f371-1837-77d1-951d-67dd16dad8e7 from actual MP3 (76570 bytes, 6.36 seconds; occurrence 01a0f36c-7bf7-7f90-a348-94bfd1972eef), latency7817.8332ms, input181/output1050/total1231. User confirmed reported Chinese promotional content corresponds to audio. FAIL: response mislabels audio as visible text and incorrectly claims no audio. Token counts alone do not establish audio omission; separate audio-token usage unknown. Refined modality-explicit invocation NOT_RUN. Direct video NOT_RUN; frames NOT_RUN by user instruction.

PASS: acquisition cache contains Apple Intelligence introduction, Apple foundation models, Apple Private Cloud Compute, and OpenAI Learning to reason with LLMs. SHA256/source URLs recorded in ignored data/research/corpus/sources-acquisition.json. Two additional OpenAI pages BLOCKED at acquisition. Paragraph candidates are unreviewed; semantic unit/oracle admission and actual formation NOT_RUN. No candidate/latency/metric fixture is accepted as live evidence.

PASS: modality-explicit direct-audio refinement committed representation 01a0f374-5c16-7b53-8bcd-ae75fad79b94, superseding the first interpretation while preserving evidence. It identifies audio and Chinese promotional speech; latency4597.4609ms, input183/output528/total711. Fine-grained brand transcription/tone/background-sound oracle NOT_RUN; no exact-ASR accuracy claim.

PASS: official Client audio formation → Memory 01a0f374-9c80-79f3-bffa-a3a504b98085 / revision 01a0f374-9c80-79f3-bffa-a3b94a4fca22 → actual embeddings → public NousQL returns this Memory at rank1 → provenance trace reaches AudioDescription/SourceRegion/MP3 source URL. Formation3543.5307ms/input542/output465/total1007. Query01a0f375-650d-7821-96e2-b99f1b60b303. This is a two-Memory smoke, not formal retrieval metrics. Native media adapter evidence now identifies gateway-chat-media-v1, distinct from AI SDK generation. Cost unknown.

## 2026-10-01 query expression progress

PASS: focused runtime allocation/scoped constraint checks and Memory owner integration: role-disjoint leaf sets intersect empty, union has2 identities, parent declarative constraint narrows to1. PASS: official Client/public NousQL on two existing real-source Memories: OR query01a0f386-dc46-7793-abb6-da7590ec78cd returned2; AND query01a0f387-09cd-75e2-88b5-a0b54b6b1403 returned0. Model ledger remained14. This verifies logical identity semantics, not retrieval recall quality.

PASS: typed cues/root-only compiler/recent(axis) parser checks, Kernel build and Clippy. Removed obsolete duplicate integration.test.ts which incorrectly preserved @e as exact read. Preference contribution, model rerank delay revalidation, formal corpus and final portable acceptance remain NOT_RUN.

## 2026-10-01 real-corpus measurement progress

PASS: six traceable sources admitted into research/corpus/manifest.json: Apple foundation models, Apple Intelligence, Apple Private Cloud Compute, OpenAI reasoning, PostgreSQL license and FFmpeg license. 105 semantic paragraph units, 40 grounded queries; source facts were re-read and every oracle anchor/fact marker verified. Raw HTML/text remains ignored. The old prefilled research runner and fixture are deleted.

PASS: exact candidate-10 ZIP 1f01ba052f0c591ec14380a069954c7889f7ff0e2ab6e96be2210a6ec1f6013a was extracted outside Git and ran private Node/PostgreSQL/FFmpeg with System32-only PATH and arbitrary CWD. It contains uncommitted implementation inputs; release manifest records source_head, source_dirty and source_input_sha256. Final frozen bundle acceptance NOT_RUN.

PASS: official Client controlled import105 and actual gateway formation105. Initial formation failures were retained and explicitly retried; three retries resolved the last two units. Actual embedding preparation encountered HTTP429. Resumption reused existing workflow results and committed vectors; operation pacing and configured embedding.max_batch_size=1 were used, with every emitted request charged to the durable ledger. No automatic provider fallback.

PASS: controlled/model-rerank and end-to-end/model-rerank each measured40 public NousQL queries. Both observed Recall@1/5/10=1, MRR=1, provenance precision=1, wrong-source/entity counters=0 and stale leakage=0. Controlled latency p50/p95=398.4849/672.0192ms,80 model requests; end-to-end=147.6890/190.6825ms,40 requests with shared query-vector cache warm. Cross-track latency is not a cold-cache comparison. Cost unknown.

PASS: controlled/baseline observed Recall@1=.85, Recall@5=.975, Recall@10=1, MRR=.90625, latency p50/p95=235.3544/473.4385ms. End-to-end baseline and Wave/combined summary pending. Important limitation: both model-rerank variants returned partial for all40 queries because the fixed validation budget was exhausted. Measurement PASS does not claim complete query execution. Formal comparative interpretation remains pending all variants and final candidate.

PASS: source-less distributed official Client sent complete raw video Artifact to doubao-seed-2.0-mini using openai-chat video_url content extension. Official demo source https://ark-project.tos-cn-beijing.volces.com/doc_video/ark_vlm_video_input.mp4, SHA256ccd285afd30d7104c3948fcf9e4a84af1019141e0ad772c4c550cab7f2d22ec6,695110bytes,5.041667seconds,H264,no audio stream. Model describes Elizabeth Tower, traffic/red double-decker bus and visible AI-generated label;9786.1103ms,input6479/output898/total7377. This is an AIGC protocol demonstration, not real-world corpus or audiovisual joint-understanding proof. Human oracle inspected a JPEG locally; model input remained raw video, frames mode NOT_RUN.

PASS: natural-language lexical regression (apostrophe, punctuation, field/operator-looking text) and cargo check nous-retrieval. PASS: pnpm check (15 focused tests, types, format, lint and dependency gates). Newly discovered launcher Windows signal shutdown was corrected to close owned stdin; final packaged shutdown proof NOT_RUN. Full just verify, Memory/Episode qualification, final source-less qualification, PR/Ready CI/merge remain NOT_RUN.
PASS: end-to-end/baseline observed Recall@1=.875, Recall@5/10=1, MRR=.93125, latency p50/p95=24.2969/29.0525ms with warm query-vector cache. Baseline and model-rerank used the same Authority sequence within each track. Both baseline variants also returned partial because of the fixed validation budget; none is claimed complete.

## 2026-10-01 Cognitive IO 合同接续

远端核对 PASS：master `37e75b8382592fe09ff536dac5a14a84dee47902`，本地/远端 topic head `8c2e50f1e28f33b957ef72a60360347384a94844`。Vault main 本地/远端 `2614d65e8775c1f9652f44d69737ac7c57afd0d4`。已完整读取新 Cognitive IO 执行包、当前 Plan/Specs、current state/architecture 和 Vault authority。Draft [PR #5](https://github.com/Heptalogos-Devs/Nous-Wave/pull/5) 已创建；ruleset `24177202` active，required check `acceptance`，仓库只允许 Squash Merge。Ready/最终 Acceptance/merge NOT_RUN。

PASS：`corepack pnpm generate`；`corepack pnpm typecheck`；`corepack pnpm lint:ts`；`corepack pnpm exec vitest run apps/nous-core/tests/model-boundaries.test.ts apps/nous-core/src/model/schemas/material-interpretation.test.ts`（7 tests）；`git diff --check`。这些是当前 working-tree 窄验证，不能证明公共 Material vertical 完成。

已建立 richer canonical Zod schema 与 Material semantic validator/projection；SDK/raw strict schema 由同一 Zod owner 生成，实际请求与 schema digest 对齐，公开 invocation summary 增加 output_schema_digest。测试包含真实本地 HTTP wire、strict 标识、SDK/raw 两路径、完整结束检查、nullable/extra-field、未知支持、直接观察支持、时间与缺失 modality，以及 uncertainty projection。测试 server 为 deterministic 协议验证，不计 live provider PASS。

FAIL：`corepack pnpm lint:knip` 当前报告 richer Material schema 尚未接入 production derivation。structured payload/ProducerSignature/workflow schema identity、description segmentation/stable supports、Resource continuation/RAGFlow、研究 guard 移出 normal runtime、最终 source-less closure 和统一 acceptance 仍在施工，NOT_RUN。未扩大 ignore 或降低 gate。

后续修复 PASS：richer schema 已接入三策略；`corepack pnpm lint:knip`、`corepack pnpm lint:ts`、`corepack pnpm typecheck` 均 PASS。ProducerSignature 与 fresh SQL 保存 output_schema_digest，DerivedRepresentation 保存 payload_json；workflow identity 包含 schema/projection identity。描述分段在 Material owner 中按 UTF-8 byte span 持久化，retry 返回同一 DerivedRegion；提交 payload 的支持 refs 按输入图批量核验。

PASS：`cargo test -p nous-material segmentation -- --test-threads=1`；`cargo test -p nous-kernel --test material_derivation -- --test-threads=1`；`cargo check -p nous-kernel --tests`；`cargo clippy -p nous-material -p nous-persistence --all-targets -- -D warnings`；`cargo build -p nous-kernel`。精确回归覆盖 UTF-8 保真、稳定 segment identity、JSON SQL round-trip、schema 变化形成新 derivation，以及输入图外的字段支持拒绝。

PASS：`corepack pnpm qualification:real-consumer-local`（Windows，Subject `01a0f6db-a633-7bd2-ba5c-c083d8fa514b`）。public Core/official Client 验证 direct/two-stage payload、选中 structured 表示、producer/invocation schema digest、stable DerivedRegion 与精确 materialize、描述阶段成功后第二阶段失败保留、重复成功操作不新增 provider request，并保持 CLI/上传/trace/use/restart。deterministic local provider 实际 3 requests，包含 1 个可控失败；liveModel NOT_RUN，corpus synthetic_local_wiring。最终新 schema 的 live 媒体/provider、Resource 与最终 Acceptance 仍 NOT_RUN。

PASS：projection identity 补入全部 structured producer config digest 后再次运行 `corepack pnpm qualification:real-consumer-local`，Subject `01a0f6dd-e718-7e61-b9a9-17512921360c`；`corepack pnpm proto:check` 和 `cargo clippy -p nous-kernel --all-targets --all-features -- -D warnings` PASS。两项跨生命周期数据库 scenario 保留带原因的 too_many_lines expect，所有行为断言保持；未调整 duplication threshold 或扫描范围。

## Optional External Resource 接续

用户 2026-10-01 修正授权：RAGFlow 为 optional external provider，不由 Nous 安装、启动或配置，不是本地材料默认后端，不属于默认 acceptance 依赖。用户确认尚未配置实例与 dataset，live RAGFlow BLOCKED。Native Material Corpus / MaterialCollection 研究留待独立任务。

PASS：`corepack pnpm qualification:real-consumer-local`，Windows Subject `01a0f716-d999-70c3-b5d2-a42573bb286d`。official Client 完整 QueryResponse 保留 resource_records/诊断；冻结 action 经 Core adapter 后沿同一 ticket finalize。选中记录形成 Artifact/SourceRegion/Observation，重复同 operation 返回同 Occurrence 且不增加 provider request；不同 operation 形成不同 Occurrence、复用相同 Artifact。public boot、CLI、upload、trace、meaningful use、restart 均 PASS。deterministic local model 3 requests（含受控失败），resource provider 3 requests；liveModel/liveRagflow 在该命令均 NOT_RUN，corpus synthetic_local_wiring。

PASS：`corepack pnpm exec vitest run apps/nous-core/tests/resource-ragflow.test.ts`；`corepack pnpm typecheck`；`corepack pnpm lint:ts`；`corepack pnpm lint:knip`。adapter 校验 pinned RAGFlow retrieval/chunk wire、nullable version、content digest、selector 越界、变化/删除/拒绝、输出上限、取消与凭据不回显。profile drift 为 stale；真实缺失为 missing。

PASS：`cargo test -p nous-kernel --test query_correctness resource_continuation -- --test-threads=1`；`cargo clippy -p nous-kernel --test query_correctness -- -D warnings`。同一 Resource cohort 验证 Subject/action/provider identity、结果上限、必需当前权限的 denied/stale 拒绝、descriptor 查询期间删除后丢弃，以及 ticket 一次消费。没有扩大 duplication exclude/threshold。

完整 Resource policy/diagnostics 收敛、最终 source-less candidate、统一完整 Acceptance 与 Ready run 仍 NOT_RUN；上述窄验证不声明整个分支完成。

## Acceptance 入口与窄扫描

`just acceptance` 已成为唯一完整 deterministic 入口；Ready/manual workflow 只安装 Node 24.20.0、Rust 1.98.1、fixed Buf plugins、just 1.58.0/cargo-deny 0.20.2/cargo-shear 1.13.4/cargo-dupes 0.2.1/OSV 2.6.0，安装 pnpm dependencies 后调用它。`just --dry-run acceptance` PASS，实际完整入口 NOT_RUN，等待最终候选；paid model/private RAGFlow 不进入默认 CI。

PASS：`corepack pnpm lint:dupes`，132 clones，overall lines 2.47%，TypeScript lines 1.73%；配置未改变。初次 `just dupes` FAIL（18 groups，上限 16）；修复共享职责后 PASS（15 groups，exact lines 6.4%，阈值 6.5%）。Subject operation transaction lock 统一到 Persistence；transport RPC reply 转换共享；typed opaque ref 构造统一 macro，保留各 namespace/type。

PASS：`cargo test -p nous-core --test primitives`（3 tests）、`cargo test -p nous-kernel --test query_correctness resource_continuation -- --test-threads=1`、`cargo clippy -p nous-kernel --all-targets --all-features -- -D warnings`、`cargo build -p nous-kernel`、`corepack pnpm typecheck`、`corepack pnpm proto:check`、`corepack pnpm lint:ts`、`corepack pnpm lint:knip`。nalgebra 升至 0.35.0 后 `cargo test -p nous-retrieval cue_sensing --lib` PASS，weighted/expanded PCA alignment 与 energy 的精度断言保持。

FAIL：`just osv`（OSV 2.6.0，`scan source -r .`）。初次命中 lru 0.16.4/RUSTSEC-2026-0253、paste/RUSTSEC-2024-0436、yauzl 3.2.0/GHSA-gmq8-994r-jv83。yauzl 升至 3.2.1、nalgebra 升级移除 paste 后再扫描，剩 lru 0.16.4 的 panic-safety advisory。Tantivy 0.26.1 与最新 published 0.26.2 均限制 lru 0.16.x；upstream main 已升级 0.18.2，但不能将大量未评估的 unreleased index/query 变化当成窄修复。此依赖仍需收敛，未添加 OSV exception、降低扫描范围或宣称 PASS。

PASS：共享职责与 dependency 更新后再次执行 `corepack pnpm qualification:real-consumer-local`，Windows Subject `01a0f74b-eeb2-7542-a771-85ad3615dd1c`，structuredPayload/fieldSupport/noChargeReplay/resourceContinuation/resourceObservation/stop-restart 均 true；仍为 synthetic deterministic provider proof，liveModel/liveRagflow NOT_RUN。

## Research guard composition

普通 Core/ModelInvocations/nous.toml 已移除 model_budget 与实例级 hard call guard；未保留 dual reader。`scripts/research-gateway.ts` 是显式研究入口，以现有标准 gateway HTTP attempt 为计数边界，将 guard 放在 run-owned proxy/ledger。真实历史 ledger 17 + 1079 = 1096 attempts，迁入 ignored `data/research/live/model-call-ledger.json`，未清零。用户模型与 credential 内容保留，active local config 删除旧 model_budget section；历史 immutable candidates 未修改。

PASS：`corepack pnpm exec vitest run scripts/research/model-call-guard.test.ts scripts/research/gateway.test.ts`，2 tests；实际 local HTTP forwarding 验证 success/failure attempts、path rejection、重启后 cap 与请求内容保真。`corepack pnpm typecheck`、`corepack pnpm lint:ts`、`corepack pnpm lint:knip` PASS。研究模块为 dev-only，未加入 bundle/application closure。

PASS：删除 normal runtime guard 后执行 `corepack pnpm qualification:real-consumer-local`，Subject `01a0f76e-924d-77c1-8a09-d7c6c843d3a0`；structured/resource/selected Observation/replay/restart 均 true。liveModel/liveRagflow NOT_RUN。后续真实研究必须显式连接 run-owned proxy；final live remeasure 尚 NOT_RUN。

## 私有 native closure 窄验证

PASS：Windows release Kernel 使用 `RUSTFLAGS="--remap-path-prefix=C:\dev\Heptalogos-Devs\Nous-Wave=. -C target-feature=+crt-static" cargo build -p nous-kernel --release --target x86_64-pc-windows-msvc`。`wsl objdump -p` 检查 Kernel/PostgreSQL/FFmpeg PE imports，未出现 MSVCP/VCRUNTIME，只有 Win32/UCRT 系统 imports；此检查不外推其他 OS。

新 source-built PostgreSQL pack SHA256 `63b455e3eef03d6d2e009abe4442278ffc9c4d91e5c8e552b7cdd4b3f99ed744`；FFmpeg LGPL-only pack SHA256 `39d426ec77482e38f6d57b83f0e1d5b6808155a3c3eab11480eebc3aba514b41`。精确 source/build/patch/compiler/runtime notices 保留在各 pack。旧 catalog/archive 保存于 ignored previous-catalog，历史候选未修改；普通启动没有 acquisition。

PASS：`corepack pnpm assemble:portable --output data/releases/candidate-11`；`corepack pnpm qualification:portable-local --bundle data/releases/candidate-11`。ZIP SHA256 `5188bd7dbe7472f86df15cf63f32f393316b7eff4a39416016b45d7e7e4af700`，仓库外安装 `C:\Users\Arsvine\AppData\Local\Temp\nous-portable-mrWkER\installation`；Subject `01a0f773-bbb8-7af0-a8dd-14fecdff643a`，无 developer PATH/source，private Node/PostgreSQL、official Client 与 restart/stableDatabasePort true。liveModel NOT_RUN，corpus synthetic_portable_wiring；该候选含 working-tree input digest，仍不是 final live/acceptance candidate。

删除 21 个 superseded Plan/Spec 文件，当前导航改为 Git history/Vault；未新增 parallel plan。剩余完整 Cargo/runtime/native SBOM、最终 live remeasure、多路径/搬移与 final acceptance NOT_RUN。

## 官方依赖修复与发布清单

Tantivy 改为官方精确 commit `5ca39332002c2c87fb5d2abc707cf527b3319d42`（含 lru 0.18.2 修复）；Cargo.lock 固定 git source，deny sources 仅允许该官方仓库，旧 RUSTSEC-2026-0253 ignore 删除。该 upstream revision 含未发布索引改动，最终 Serving/live qualification 必须使用新 candidate，旧测量不能自动沿用。

PASS：`just osv`（OSV 2.6.0，source scan Cargo.lock/pnpm-lock.yaml，No issues found）；`cargo deny check advisories`；`cargo deny check sources licenses`；`cargo test -p nous-retrieval --lib`（7 tests）；`just dupes`（15 groups/6.4%）。没有修改扫描范围或 duplication threshold。

首次 `cargo clippy -p nous-kernel --all-targets --all-features -- -D warnings` FAIL 于 postgresql_embedded 的 wildcard build-time release 获取（GitHub response decode），固定 POSTGRESQL_VERSION=18.6.0 重试 PASS；justfile 同步固定已采用版本。Windows static CRT release build PASS，PE imports 无 MSVCP/VCRUNTIME。

`cargo test -p nous-kernel --test query_correctness -- --test-threads=1` FAIL，11/12；exact mutable binding 先被普通 stale revision filter 遮蔽，返回错误 drop reason。Memory owner 把 frozen-binding fence 放在普通 revision policy filter 前；原 `exact_mutable_binding_is_fenced_and_explicit_history_is_readable` 窄回归 PASS，`cargo clippy -p nous-kernel --test query_correctness -- -D warnings` PASS。全套在最终 acceptance 重跑，当前不声明全套 PASS。

PASS：`corepack pnpm qualification:real-consumer-local`，Subject `01a0f788-a763-7c80-b0e3-6dd6340c8e15`；公开 query/material/resource/selected Observation/no-charge replay/restart 均 true，liveModel/liveRagflow NOT_RUN，synthetic_local_wiring。`corepack pnpm typecheck`、`corepack pnpm lint:ts`、`corepack pnpm lint:knip` PASS。

发布组装已扩展 Cargo release/build dependency closure、SPDX relationships 和 runtime packs；保存每个 crate 的 metadata/许可文本与精确来源，build-only 输入区别于链接库。candidate-16 inventory 实际 377 Cargo inputs、缺失许可文本 0；组装器缺许可即失败。published htmlescape crate 已声明 Apache/MIT/MPL 选项，保留原 metadata/README 并按 Apache-2.0 选项附官方文本；其旧 repository 不可用，未虚构 publisher NOTICE。最终完整 native/static CRT SBOM 与精确 final candidate 仍待复核。

## 配置中的 embedding public proof

真实 portable 启动 FAIL：`resolved embedding signatures are not canonical`，调用计数未增加。ProducerSignature 增加 nullable output_schema_digest 后，Core embedding canonical bytes 遗漏了该字段；Kernel 严格校验正确。Core owner 补入 null 参与 hash，typed TOML 对缺省 Option 继续省略，未在消费者中绕过 signature。

在既有 public qualification 中增加配置后的 embedding boot/commit/query，用真实 HTTP fixture 验证跨语言 canonical signature。修复前 `corepack pnpm qualification:real-consumer-local` FAIL，修复后 PASS；Subject `01a0f799-0180-71c3-951f-1880d432c1dd`，embeddingCalls=2、canonicalEmbedding=true，保留 resource-only 无 embedding 的先前阶段、schema/support/selected Observation/replay/restart。`corepack pnpm typecheck`、`corepack pnpm lint:ts`、`corepack pnpm lint:knip`、`corepack pnpm lint:dupes` PASS。

candidate-18 ZIP SHA256 `46a7e0e75231902cc2012b6be1c91d0c1db61d74de51f76ad4ae3ca601eec7b1`。仓库外完整 ZIP 解包，使用私有 Node/PG/FFmpeg、System32-only PATH、新分离 instance home 和用户真实模型配置；actual boot PASS，Core endpoint `127.0.0.1:14653`。研究 GatewayProfile 指向显式 loopback accounting proxy，该 proxy 透传到用户 New API `/v1`，不改变 upstream provider/credentials。通过分发 official Client 的新 rich-schema 媒体 qualification 正在运行；此处尚不声明整组 live PASS。

媒体 runner 汇总已区分 pipelineStatus/qualityStatus/queryStatus。缺 independent source oracle 的内容质量记 NOT_RUN；有 oracle 的缺事实或查询不完整记 FAIL，不再只凭 derivation pipeline 成功汇总 PASS。frames 仍 NOT_RUN，直接发送 raw image/audio/video Artifact。

## Rich-schema 真实媒体执行

`corepack pnpm qualification:live --run-root <outside-home>/run --client-module <outside-installation>/program/client/node.js --state data/research/live/media-rich-schema-state.json` 已执行；exact candidate-18 digest 如上。formation/direct/vision=`doubao-seed-2.0-mini`，embedding=`doubao-embedding-vision`/2048，rerank=`qwen3.7-text-rerank`，同一 gateway 标准协议/媒体扩展；未推断网关 alias 的实际 upstream revision。

Subject `01a0f79b-e4bc-7b11-bddc-721ff2a8b582`；5 Apple 图片、2 NASA raw video clips、2 同来源 audio clips，9 units/27 strategies。完整查询 9/9 PASS；pipeline 23/27 PASS、4 FAIL（Writing Tools/Notes 的 two-stage structuring、Siri direct、press video two-stage）。整个 live run FAIL，未当作 acceptance PASS。两段 video 经私有 FFprobe 检查均有 video+audio stream；原始 bytes 直接发送，模型端联合音画理解的独立质量证明 NOT_RUN。

独立回读 Compose screenshot 后发现旧 oracle 的 compose|prompt 不是可见文本；它错误惩罚了描述的正确画面。tracked oracle 改为实际可见 Include All Text，原 run state 保存于 ignored media-rich-schema-original-oracle.json；只重新评估既有输出，没有新增付费请求。原两项 quality FAIL 得到纠正，但 direct structured 确实遗漏新可见标签，仍 FAIL。当前 quality FAIL=1、audio independent oracle NOT_RUN=6；没有提升未验证音频质量。

研究 ledger 从1096增至1230，实际134 attempts，包含失败/prepare/query/rerank。成功响应中已收集108个 invocation requests、provider-reported total usage140884、invocation latency累计471793.1325ms；这些是已知子集，不是全量账单或 wall-clock time，query/partial/failure 的未收集 usage 不推填，cost unknown。数据为真实 gateway/公开 media/official Client 路径；无预填 latency/candidate/指标。

## Formation failure and media continuation

`corepack pnpm test apps/nous-core/tests/model-boundaries.test.ts` PASS (3 tests), `corepack pnpm typecheck` PASS, `corepack pnpm lint:ts` PASS. The current public deterministic proof (Subject `01a0f7f3-aa93-76e1-ab60-4f5e530bcca1`) verifies an HTTP503 formation failure returns bounded degradation without provider body/secret, then the same operation succeeds on retry. Cancellation and semantic errors retain RPC handling. Image content uses the current AI SDK file part; wire assertions still verify PNG data URI encoding. Live failure reasons require the rebuilt payload before interpreting the two remaining formation failures.

Media retries reused successful receipts. The research-only structuring timeout was explicitly increased to120seconds for new producer snapshots; original user settings and product defaults were preserved. Latest candidate-19 pipeline is27/27 PASS and downstream query9/9 PASS, with one independently observed structured-image omission FAIL and six independent audio factual checks NOT_RUN. The overall media qualification remains FAIL. Direct video bytes were used; frames NOT_RUN. Exact final bundle acceptance remains NOT_RUN.

Controlled import is PASS:105 grounded source units and105 real embeddings, Subject `01a0f7fc-b8e1-7481-9db5-c49f4339a732`. End-to-end Subject `01a0f7dc-207e-7e52-bdd6-bf03d5e9e9a7` has103/105 successful formations; two failed inputs remain explicit. Each of four measured variants returned40 complete responses, using the same Authority sequence per track (controlled421, end-to-end522). Separate cold-start measurements are being collected to remove process-cache asymmetry from latency comparisons; earlier warm measurements remain raw evidence, not comparable cold latency.

Binary path audit FAIL for candidate-19: Kernel contained local Cargo source paths despite checkout remapping. The release rebuild with Cargo/Rust toolchain remaps is PASS; ASCII string audit finds zero local checkout/user source paths. PostgreSQL inherited compiler debug paths. LLVM-MinGW `llvm-strip --strip-debug` removed them from a fresh pack stage; postgres.exe audit finds zero source paths. New pack digest `c1d8b7e81dcc389a60e43bfdedb4ce820fb61278b053dd175c8ba9d63d96e600`, manifest digest `8eb181db0346a2613c477a265c12298d5b3de28876ba35301f61c67e16681968`. All-binary inventory, native SBOM and exact reassembled bundle qualification remain pending. No final source-less PASS is claimed from the existing candidate.

## VCP route/source audit

审读 VCPToolBox 当前 commit `ead5a021d81baca3233eb8a726f8ea7a69fabc70` 的公开生产文档及 LICENSE：文档 SHA256 `377a1d1161a4ba530a883b1b2fd03f58f773f1fdd9cebffafe8a92e4773662d7`，LICENSE SHA256 `5392a8b3f46108fa3494e8c15b57da9e14e9bbd93335590cb5b50364fcf770a8`。ignored research cache 保留精确文本；没有把上游 source/fixtures 加入本分支或 bundle。Active Spec 标注 current route、CC BY-NC-SA 4.0 来源隔离与独立实现要求。

reviewed cue_sensing/residual/wave/topology_lane 当前路径只提供 PCA/residual/bounded propagation/node potential。新增 typed mechanism 摘要 experimental-node-potential-v1，Runtime diagnostics 回传同一 owner 字段；没有添加 Ω/curve/field scoring。`cargo clippy -p nous-retrieval -p nous-runtime --all-targets -- -D warnings` PASS。完整 VCP、formal Wave/combined 本分支 NOT_RUN；下一独立任务才能施工。

## Candidate-19 cold-query retrieval measurements

Each track/variant starts a fresh Core. Public Client timing includes query embedding and model rerank where enabled. Some runs overlapped a release build; latencies are observations on this host. Provider cost unknown.

| Track / variant | Recall@1 | Recall@5 | Recall@10 | MRR | p50 ms | p95 ms | Authority | Response |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| controlled / baseline | 0.85 | 0.975 | 1 | 0.90625 | 256.447 | 316.42 | 421 | 40 complete |
| controlled / model-rerank | 1 | 1 | 1 | 1 | 605.668 | 1346.619 | 421 | 40 complete |
| end-to-end / baseline | 0.875 | 0.975 | 0.975 | 0.920833 | 239.88 | 482.784 | 522 | 40 complete |
| end-to-end / model-rerank | 0.95 | 0.975 | 0.975 | 0.9625 | 392.619 | 493.966 | 522 | 40 complete |

Each track retains one fixed Authority dataset across mechanisms. Provenance precision=1; measured wrong-source/entity/stale leakage/empty-result rates=0. These source-grounded measures do not independently establish all factual correctness. End-to-end formation coverage=.975 over query oracles; conditional Recall@5/10=1. Wave/combined NOT_RUN because no qualified topology signal is present. Formal results remain associated with candidate-19 digest783ae5ddfeb4b86f4848d86037ed78ab661fe042c1377cc487602b58e09090ce, not a future final payload.

## Candidate-20 and Resource boundary repair

Candidate-20 ZIP digest `636dc625f5269757637b375852232a7cf41f4b6bf7f31e120735bafdfce47275`, clean source head `4c45c05a84449c4e92bb0ebd04a004847d9ac93c`. Exact ZIP extracted outside the Git checkout, then started with private Node/PostgreSQL and System32-only PATH: PASS. All38 EXE/DLL files passed development-path string audit. `corepack pnpm qualification:portable-local --bundle data/releases/candidate-20` PASS for synthetic public Client/CLI/trace/use/stop/restart/stable database port; Subject `01a0f81e-e698-7a60-aec4-6e2caa61dc1b`. This separate directory-copy test is not an exact final ZIP/live complete acceptance.

Live formation retries through candidate-20's distributed Client preserved operation IDs and reused103 committed results: apple-pcc-p28 recovered; apple-intelligence-p8 returned `memory_formation_failed/output_schema_invalid`, with no provider body/token. Current end-to-end import remains FAIL (104/105), one new Memory embedding committed; raw state retains the failure. Earlier measured421/522 Authority snapshots are unchanged artifacts and must not be attributed to this modified dataset. Wire ledger2065/10000 actual attempts; final cost unknown.

Resource max-material regression reproduced the Core1MiB/Kernel256KiB mismatch (RED: bound action rejection). Kernel now permits1MiB per record while retaining2MiB aggregate serialized records. Exact maximum is accepted, maximum+1 rejected, identity/access/descriptor checks retained. `cargo test -p nous-kernel --test query_correctness --all-features -- --test-threads=1` PASS12/12; narrow regression RED→GREEN; `cargo clippy -p nous-kernel -p nous-runtime --all-targets --all-features -- -D warnings` PASS. Removed dead internal HistoricalOrStale enum with no compiler/public producer; no compatibility parser was added. Provider evidence/diagnostics remain pending. Candidate-20 predates this boundary repair, so final payload reassembly remains NOT_RUN.

## Resource execution evidence

Canonical proto generation and protocol lint PASS. Current private result carries provider profile digest, initiated HTTP attempt count, elapsed milliseconds and bounded diagnostics; public `resource_invocations` is returned by the official Client. The adapter increments attempts at fetch initiation (including failure); unconfigured profiles count0. Kernel validates evidence bounds/profile consistency, restores fixed action order, and exposes discarded execution when descriptor Authority changed.

`corepack pnpm qualification:real-consumer-local` PASS, Subject `01a0f830-e220-7833-8729-7e5f784ea18e`: public successful query count equals mock provider's actual counter; unconfigured Resource produces zero-request unavailable evidence and keeps the other external record. Existing structured payload/field supports/formation failure retry/selected Observation/replay/restart remain PASS. Provider is explicitly synthetic_local_wiring; live RAGFlow BLOCKED.

`corepack pnpm test apps/nous-core/tests/resource-ragflow.test.ts` PASS1/1; typecheck/TS lint/Knip/format PASS; targeted Kernel continuation fence PASS with invalid provider count rejection; Kernel/Runtime all-targets/all-features Clippy PASS after extracting payload validation from Authority lifecycle handling. `just dupes` PASS15 groups/6.4%; TS duplication PASS2.57% (same scope/bounds). Final full acceptance and rebuilt payload qualification NOT_RUN.

## LLVM-MinGW shipping closure and exact ZIP

2026-10-02 user approved replacing the MSVC shipping path with LLVM-MinGW. Official20260922 Windows UCRT compiler archive verified SHA256 `e3ad77d117a4bea19a7a3b333341824d79a5a371004a10e25b8504e7b3047666`. Initial windows-gnu build compiled USearch/NumKong but failed GCC libstdc++/libgcc linkage. Replaced with native windows-gnullvm target and explicit libc++ configuration, without fake library aliases. `scripts/build-windows-kernel.ps1` PASS; standalone Kernel --help PASS after including actual libc++.dll/libunwind.dll closure. Import audit shows LLVM DLLs and Windows system/UCRT imports; no MSVCP/VCRUNTIME dependency. Kernel local source-path audit PASS; stripped libunwind source-path audit PASS. Six upstream LLVM libc++ assertion source strings remain, separate from operator paths and runtime location resolution; no binary string rewriting was used.

SPDX candidate inventory now475 packages/1132 relationships, including21 embedded Node dependencies from private Node's reported versions, retained Node notices/SQLite dedication, LLVM/MinGW and Rust standard-library notices with source/version references. Private compiler binaries are build inputs, not shipping payload. Current typecheck/TS lint/Knip/format PASS; TS duplication2.56% PASS at unchanged bounds. Final payload audit remains separate from assembly.

Portable qualification now takes the exact ZIP, extracts it outside the repository, and exercises colocated or fully split locator paths with optional installation relocation. Candidate-24 ZIP digest `b17c8de2efd99e464e48f1f3c3374527409d9682a45cc2446bd7f7a28173d137`: locator+relocation/public Client/trace/use/restart/stable port PASS, Subject `01a0f85b-a6af-79e1-8aa5-55b99c777d3b`. Live import through its distributed Client recovered apple-intelligence-p8 with the original operation ID:105/105 formations PASS, one new embedding committed. Ledger2067/10000; earlier measured Authority dataset is historical and final variants must be measured again.

The colocated missing-pack proof initially FAIL: ENOENT lacked the explicit install command and launcher stdin kept the failed child alive until timeout. Fixed runtime verification's missing-manifest error and paused launcher stdin on child exit/error. Candidate-25 ZIP digest `6245c5d486f019916b151682b6173337f2632a69e51c9f9f15d7f5de6d18a126`: exact ZIP colocated/public Client/restart/stable port/missing-pack rejection PASS, Subject `01a0f860-5925-78f1-950c-4ef259bc6855`. Missing pack returns UNAVAILABLE/install command promptly and preserves database state. No acquisition is present in this verify-only startup path; liveModel NOT_RUN for this deterministic proof. An initial proof command raced unfinished archive assembly and FAILed before launch; it was rerun only after the assembly handle completed. Locator+relocation on candidate-25 and final clean-head acceptance are pending.

Candidate-25 exact ZIP locator+relocation+restart+stable port+missing-pack rejection PASS, Subject `01a0f861-b56e-7fb1-9294-bdbefd5de37f`; all instance roots are independent of Program/Runtime and CWD. Both colocated and split layout proofs use private Node/PostgreSQL, shipping Kernel/LLVM DLLs and System32-only PATH. Final whole-branch acceptance NOT_RUN. Assembly inventory's Cargo platform filter was then corrected from historical MSVC to shipping gnullvm; final rebuilt SPDX counts and payload digest must be recorded before Ready. Normal developer Cargo target remains incremental.

## Candidate-26 与音频语料修正（2026-10-02）

candidate-26 精确 ZIP SHA256 `e83c4324bb4464d7e9d2276e03ae17325a90e3f723dbe21294ee3d8bd12eac69`，source `a38cd07a26f93cdd36cbf0e3e8701989323605d8`，source input SHA256 `f45f6fcc6d3ce674582ddb461ee928a9cc027c973f0163c9e25fa277fc6567d3`，source_dirty=false。locator/安装搬移/official Client/trace/use/停止重启/固定端口/缺包拒绝 PASS，Subject `01a0f871-e934-7580-ba24-55663f90574d`。该包包含 corrected gnullvm SPDX；后续文档/语料修改尚未作为最终分支包重新组装，final acceptance/Ready CI NOT_RUN。

真实文本 formation 已补齐105/105，replay audit105/105 PASS、无新付费请求。controlled Authority421、end-to-end Authority526；每变体在新 Core 上运行40条真实 NousQL，四变体全部 complete，数据在各轨内固定。使用真实 embedding 与 `qwen3.7-text-rerank`；研究 proxy2067→2307，共240实际 HTTP 尝试。

| Track / variant | Recall@1 | Recall@5 | Recall@10 | MRR | p50 / p95 ms |
| --- | --- | --- | --- | --- | --- |
| controlled / baseline | .85 | .975 | 1 | .90625 | 252.0808 / 415.0156 |
| controlled / model-rerank | 1 | 1 | 1 | 1 | 393.1584 / 468.2948 |
| end-to-end / baseline | .9 | 1 | 1 | .945833333333333 | 238.5688 / 413.0121 |
| end-to-end / model-rerank | .975 | 1 | 1 | .9875 | 375.6403 / 567.7227 |

六个文本来源、105单元、40条 grounded queries。formation coverage/provenance precision=1；wrong-source/entity、stale leakage、empty rate=0，限定于本语料和既定 oracle。Wave/combined NOT_RUN：缺少合格的真实 topology 信号。cost unknown。ignored `data/research/live/candidate-26-{controlled,e2e}-{baseline,rerank}.json` 保存逐条响应；不能沿用旧 Authority522 的指标。

原9个媒体单元在新包上的 fresh run 为26/27 pipeline PASS、9/9 query PASS；Compose 图片两阶段 formation 因 output_schema_invalid FAIL，已提交 derivation 保留。用户随后独立回听两个 NASA WAV，报告两者基本相同：背景白噪音/电流声及间歇性周期提示音，第一段较轻，无可辨语音；设备身份未知。模型多识别重复嗡鸣/提示音且未制造语音，但遗漏或否认底噪，完整性 FAIL。原数据和结果保留，`research/corpus/audio-negative-controls.json` 记录负例；主音频语料移除这两个近重复样本。历史记录的独立音频 oracle NOT_RUN 由本次用户事实审查补充，不能据此倒改历史模型响应。

主音频改为 Kevin MacLeod 的《Carefree》和《Gymnopedie No. 1》，来自作者 Incompetech，CC-BY-4.0，需要署名，属于免费授权音乐。保留原 MP3 SHA256 和作者 instruments/feel metadata；私有 FFmpeg 截取10–35秒、PCM16 mono24kHz，ffprobe 两段实际25秒 PASS。上传文件名 music-a.wav/music-b.wav；oracle 乐器信息不传入模型。作者 metadata 支撑音乐分类和乐器族 oracle；精确时长识别、节奏和细节听辨 NOT_RUN，不声称 agent 亲自听过。

通过 candidate-26 distributed official Client 运行6个三策略 workflow 和2条 public NousQL，Subject `01a0f887-754c-7882-884e-a483f483f50c`，ignored `data/research/live/music-candidate26-state.json`。description_only 两段均 pipeline/metadata-scope quality PASS；direct_structured Carefree 提交 PASS、遗漏预定 ukulele/guitar 信息使 quality FAIL，Gymnopedie PASS；describe_then_structure Carefree PASS，Gymnopedie 第二阶段因 invents unavailable visual input 被校验拒绝 FAIL，第一阶段 AudioDescription 保留。两条查询2/2 PASS。未降低 oracle 或关闭校验来取得 PASS。

此次音乐调用 ledger2450→2479，共29实际尝试；已返回并保存的成功 derivation/formation summary 为12 requests、total usage18386、模型 latency 累计73363.942ms，仅成功结果子集，不能当全量账单或端到端耗时。累计 ledger2479/10000；失败 usage/cost unknown。新音乐实验整体 pipeline/quality FAIL，最终媒体能力仍未通过完整质量验收。

## 完整分支审查修复与干净环境证明

对 `37e75b8..20e83c9` 的完整审查发现6项 Important，审查结果 FAIL；用户随后禁止子代理，后续修复和验收均由当前执行者完成。修复覆盖现有 owner，不增加 crate、兼容路由或新 workflow。完整 acceptance/Ready CI 仍为 NOT_RUN。

- PostgreSQL preparation：独立不存在的 runtime 目录使公开 Memory qualification FAIL。新增显式 development/qualification `cargo run -p nous-kernel --example qualification_postgres`，复用固定18.6.0 dependency，同卷 staging 后发布真实安装目录，拒绝错误版本/partial install；加入 `just acceptance`，YAML 保持薄入口。首次实现误留 versioned 子目录，公开 boot 仍 FAIL，修正后 fresh-v2 runtime 的 Memory Reference、WorkContext/Episode 均 PASS。两个旧 qualification producer 补齐 QueryExpr `operation=atom`；没有恢复省略 operation 的 parser。
- Resource ticket 与 rerank：无正文本意图、required reranker、两个文本候选的复现 FAIL；Core 独立判断 rerank eligibility，同时 finalize/release Resource ticket。精准测试 RED→GREEN PASS，不声称错误路径产生过付费请求。
- Scoped current authority：official Client 的嵌套 required 原只触发1个 provider read，缺失版本核验，FAIL。current-authority 移到内部 QueryConstraints 单一 owner，删去 ResourceIntent 副本，按父约束继承/子细化冻结 action；同场景现在2次真实 fixture HTTP read，public qualification PASS。旧 historical_or_stale 编译值删除，当前只接受 none/prefer/required。
- Exact/domain：精准测试确认 exact child 丢失父 Memory fence，FAIL；继承的 domain 与 exact selector 独立保留，冲突 domain 产生空 branch。现有 query correctness cohort 实际验证 Artifact exact 不穿过 Memory root、合法 Memory exact 仍返回，PASS。
- Resource proposal/outcome：1 MiB 内容在完整 JSON proposal 中超过旧1 MiB限制，public selected materialization FAIL。原始内容上限不变，完整 JSON 使用8 MiB bounded envelope以容纳最坏6倍转义，private ModelMaterial RPC 配套上限；RAGFlow JSON response 上限8 MiB，query records2 MiB aggregate bound不变。official Client 选用1 MiB ASCII及1 MiB转义控制文本、Observation admission、同 operation replay 无新增 provider read均 PASS，Subject `01a0f8dc-1bae-7021-9331-8ff9f7b4fe34`。fixture proof不计入真实模型/corpus实验。
- Frames fresh TempRoot：现有 regression 改用不存在的独立 TempRoot，RED ENOENT；media owner先创建该 root，再只清理操作子目录，GREEN PASS。真实 frames model invocation仍 NOT_RUN。

窄检查 PASS：query correctness12/12；Runtime unit6/6；TS focused4 files/7 tests；Kernel/Runtime all-targets/all-features Clippy；TypeScript type/lint/Knip。PostgreSQL patch 的空白 context 已精简，官方原始18.6 source上的 `patch --dry-run` PASS；未改编译后的代码效果。当前 `git diff --check` PASS，最终提交后会复核整个分支范围。

此前 candidate-27 source20e83c9、input digest `1e3cf99ae95e66914740df1b7064e96d20bc1c6d67663c4b814172a6cd2b6116`、ZIP `eafc00c31b05debcaf401a967f1e66d10431e9bab3f4674d014757f163f686ba`：共置与完全分离 locator/搬移/official Client/restart/固定端口/缺包拒绝 PASS，分别 Subject `01a0f8c8-0bcb-7a70-9520-35a67d57123b`、`01a0f8c8-b085-7b73-a9ab-c2783c0d5dfc`。2239 checksum entries、40 native operator-path audit、credential/instance/development inventory排除 PASS；SPDX475 packages/1132 relationships。该包早于上述修复，不能成为最终 changed-code payload证据；新包与实际模型关联仍待执行。

`68a69382731a4f452f96432b0777928a0168842c` 干净 checkout 的首次完整 `just acceptance` FAIL 于现有 maintainability gate：tree diagnostics 与 prepared ticket 各有一处 `expect()`。没有降低 lint 或扫描范围；改为明确 Infrastructure error 与单次 occupied entry/remove，`just lint-maintainability` PASS、Runtime unit6/6 PASS。完整验收尚须在修复后的干净 head重跑。

同提交的 candidate-28 ZIP `16273a31a741c2d512056937df1113f4693241d9df338ba195f3f5599f6dde95` 共置 private-runtime/official Client/restart/固定端口/缺包拒绝 PASS，Subject `01a0f8e7-5a46-7683-84bd-6caa13ee16b7`。受控轨 baseline40条真实查询 complete，Recall@1=.85、Recall@5=.975、Recall@10=1、MRR=.90625，p50/p95=224.4293/260.5228ms，Authority421不变；ledger2479→2519。该候选早于上述维护性修复，其余变体暂未执行，不能替代最终候选测量。

`4110a91be134df0c023fba62c6c66864eb9ee931` 的完整 `just acceptance` FAIL 于旧 `process_restart` fixture，尚未进入剩余 gates。该 fixture 使用已删除的 `NOUS_WAVE_POSTGRES_URL` 和缺少当前 required roots 的 config。依 COST/test-value-pruning 删除这个 private Kernel 启动/Session/WorkContext 重启 scenario：它与保留的 official Client Memory Reference、Cognitive Runtime/Episode public restart proof重叠，且不验证 managed-private 数据库或 Core 路径；保留独立 owner regression 与更强 public proof。没有恢复旧 env reader、减少 acceptance 命令或降低阈值；历史测试由 Git 保存。

candidate-29 source4110a91、input digest `7469add1d25b03c0449e32d1ac842718acc7caaf24a8345b5ee92f32e2783e64`、ZIP `45cc0d4d9669db9fb3647b3103b96e23113ee850adac6fb8882fbc1941364de7`：共置、split locator/安装搬移、private runtimes/official Client/完整停止重启/固定端口/缺包拒绝均 PASS，Subject分别 `01a0f8ef-de29-7b32-b076-3aec75456280` 与 `01a0f8ef-de54-7043-95f5-074bb57cd592`。2239 checksum entries PASS，SPDX475/1132。受控 baseline40条查询 complete、Authority421不变，Recall@1=.85、Recall@5=.975、Recall@10=1、MRR=.90625，p50/p95=207.2335/269.3868ms；ledger2519→2559。其余 formal variants尚未完成，Ready/merge仍 NOT_RUN。
