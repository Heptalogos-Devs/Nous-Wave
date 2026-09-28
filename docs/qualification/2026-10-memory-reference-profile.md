# Memory Reference Profile Closure R2 Qualification

日期：2026-09-29

状态：PARTIAL / ACTIVE

直接执行包：`Nous-Wave-Memory-Reference-Closure-Spec-R2`。包内 `MANIFEST.md` 列出的 11 个合同 Markdown 加 `README.md` 共 12 个文件；SHA-256 全部匹配（`SPEC_HASH_MISMATCHES=0`）。

实现证据范围：`ecc4435..ebb2a4c`（最终 docs-only 状态以本记录所在提交为准）。

## A. Documentation

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| Architecture-Vault multi-session intent | PASS | Vault `c1c1bb9` 更新 Target Design/Decisions，明确同一 Subject 共享 durable cognition、Session/consumer runtime 隔离。 |
| Active Cognition 未被当前实现永久缩窄 | PASS | Vault Target Design 保留长期 Active Cognition/WorkContext；Nous current-state 只描述当前 runtime slice。 |
| Episode / Journal / Dream lineage | PASS | Vault Target Design 与 VCP/联想谱系 research 文档保留长期目标和历史术语。 |
| Anchor research question / VCP lineage | PASS | `VCP_AND_ASSOCIATIVE_LINEAGE.md` 已加入 research index。 |
| Self/Social current expansion authorization | PASS | Self、Configuration/Social plan/spec 已移至 `superseded/`；实现保留但不再授权扩展。 |
| Memory closure current authorization | PASS | `docs/plans/active/2026-09-28-memory-reference-profile-closure.md` 成为当前唯一新增代码授权。 |

## B. P0 correctness

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| Observation → ResidentSet schema / batch / rollback / retry | PASS | `ecc4435`；`runtime_residency` 4/4。包含 subject ownership、atomic batch、no-op revision、eviction 单次推进、坏 persisted ref 显式错误。 |
| Configuration frozen scoped outcome | PASS | `08c27ae`；`configuration` 2/2。receipt 保存 subject/system resolved `active_digest` 与 `desired_digest`，replay 不随后续 mutation 漂移。 |
| Social provenance read model | PASS | `c0fdc60`；`social_cognition` 4/4。Relationship/Convention supports 与 formation evidence 稳定 round-trip。 |
| EPA weighted PCA geometry | PASS | `0619066`；`cargo test -p nous-cognitive-retrieval --lib` 8/8，240/40/20 compressed-vs-expanded regression。 |

## C. Memory semantic closure

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| deterministic multi-session reference scenario | PASS | `8a61d87`；`memory_reference_closure` 1/1。固定 Subject/Session/Observation/request IDs，覆盖 M1/M2/M1-R2/M3、Alice/Bob aboutness、Use、lifecycle、Serving、reopen。 |
| MUST_RETURN / MUST_NOT_RETURN oracle | PASS（fixture scope） | `tests/fixtures/memory-reference-r1/{corpus,queries,oracle}.json`；scenario 对 entity、exact historical、runtime、restore/purge oracle 实际断言。 |
| provenance trace to source | PASS | scenario 读取 returned revision support，并由 Occurrence materialization 回溯到 source material。 |
| retrieval hit 与 meaningful use 分离 / UseEvent idempotency | PASS | scenario + `query_correctness`；重复 event 不重复 durable/runtime effect，consumer scope 独立。 |
| suppression / restore / purge / serving invalidation | PASS | scenario 与既有 `reference_profile`；purged exact target 显式 `NotFound`，旧 use retry 返回 duplicate。 |
| full R2 corpus categories（lexical/dense/topology/budget/unavailable 的统一 oracle） | NOT_RUN | 现有 targeted tests 分别覆盖部分行为；尚未将全部 lane/diagnostic category 合并到同一版本化 oracle。 |

## D. Multi-session Runtime

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| one Subject / two open Sessions | PASS | `memory_reference_closure`；A/B 同时 open。 |
| durable Memory shared | PASS | 两个 Session 对同一 Subject current cognition 可见。 |
| ResidentSet / runtime revision / Focus isolated | PASS | Observation admission、Focus checkpoint、A/B runtime snapshot 断言。 |
| ConsumerWorkingSet independent | PASS | `consumer:alpha:reference` 与 `consumer:beta:reference` 各自 materialize 不同 exact revision。 |
| consumer-scoped UseEvent | PASS | A/B 使用同一 Subject cognition，event identity 与 side effect 分离。 |
| close-one-session leaves other alive | NOT_RUN | 现有 close API 有 focused coverage，但 closure scenario 尚未把 close A 后 B 的完整 oracle 纳入。 |

## E. Recovery

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| composition teardown/reopen | PASS | closure scenario drop/reopen；`reference_profile` reopen + exact serving rebuild。 |
| Subject/Session/ResidentSet/current Memory/Use receipt restoration | PASS | closure scenario + existing `reference_profile` evidence。 |
| serving generation reopen/rebuild | PASS | `reference_profile` corrupt artifact → reopen rebuild；generation identity changed and Authority content remained. |
| stale generation cannot expose invalid cognition | PASS | existing `reference_profile`/`query_correctness` stale regression。 |
| actual subprocess process restart | PASS | `cargo test -p nous-kernel --test process_restart -- --test-threads=1`：Kernel child process 两次启动，Subject/Session/runtime checkpoint 跨进程恢复。 |

## F. Retrieval / Serving qualification

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| Q-01 stale old revision | PASS | `query_correctness::stale_lexical_generation_cannot_return_old_revision`。 |
| Q-05 lexical provider authority | PASS | `lexical_lane_does_not_create_rank_from_substring` 与 `lexical_provider_hit_survives_non_literal_case_difference`。 |
| Q-11 association producer contract | PASS | `association_requires_exact_cognition_and_valid_support_class`，包含 meaningful-use 与 derived producer path。 |
| deterministic closure corpus | PASS（closure scope） | fixed fixture + executed scenario。完整 lane category oracle 仍见上方 NOT_RUN。 |
| full scale fixture | PASS | `f65a74c`；`scale_fixture` 1/1，10,000 current / 3,000 historical / 30,000 associations / 2,000 entities / 1,000 tags / 200 schemas。 |
| temporal instant boundary correctness | PASS | `0f5bf1c`；scale instant-at-query-start gate PASS。 |
| serving rebuild/reopen | PASS | existing reference profile matrix。 |
| performance baseline | PASS（observed, no SLA） | `scale_fixture --nocapture`：20 queries，p50 约 5.27 ms，p95/max 约 84.17 ms，Serving build 约 4003.49 ms；query count/RSS 未 instrument。 |
| correctness admission gate | NOT_RUN | 完整 lane/diagnostic oracle、subprocess restart 和 official client path 尚未全部 PASS。 |
| baseline vs Wave | NOT_RUN | 按 Spec gate，未在 admission gate PASS 前运行。 |

## G. Official API / Client

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| official TypeScript Client surface exists | PASS | `packages/client/src/index.ts` exposes subject/material/cognition/memory/query/use/lifecycle wrappers；generated protocol unchanged by this R2 slice。 |
| closure scenario driven through official client/Core path | PASS | `corepack pnpm exec tsx apps/nous-core/official_client_closure.ts`：真实 Core + Kernel + `@nous-wave/client` 驱动 Subject、Session、Observation、Memory form、ReportUse/retry 与 readback。 |

## H. Full verification / repository gates

本轮 R2 新增提交后，完整命令尚未重跑，暂记：

| 命令 | 状态 |
| --- | --- |
| `corepack pnpm generate` | PASS |
| `corepack pnpm check` | PASS（Buf、tsc、Vitest 3 files/7 tests、Prettier、Oxlint、Knip、dependency-cruiser、jscpd、Sherif） |
| `cargo nextest run --workspace --all-features` | PASS（`just nextest` exit 0，serial test threads） |
| `just verify` | PASS（fmt/source-shape/check/clippy/workspace tests/deny/shear） |
| `git diff --check` | PASS（每个 commit 前） |
| `just dupes` | FAIL（32 exact groups，threshold 16；未扩大 threshold/exclude） |
| `just osv` | FAIL（`osv-scanner` executable 未安装/不可识别；未增加 ignore） |

## I. Remaining status

- `SPEC_CONFLICT`: None observed against scoped AGENTS, Architecture-Vault Target Design/Decisions and R2 Spec.
- `SPEC_GAP`: None observed. Remaining items are missing qualification/path evidence, not undecided public semantics.
- Intentional deviation from Spec: None.
- R2 status remains `PARTIAL / ACTIVE`;不得写 `Memory Reference Profile R1: PASS`，直到 full oracle、subprocess restart、official client path 和 full verification gates 完成。
