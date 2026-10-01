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
