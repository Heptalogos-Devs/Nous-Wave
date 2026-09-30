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
| Official Client reference consumer local wiring | PASS | Windows public Core main entrypoint, CLI discovery, subject/session, text observation, streamed file upload, deterministic Memory, NousQL, trace, meaningful use and restart; model derivation and full producer trace still pending |
| Live gateway protocols / formation / embedding / rerank | BLOCKED | user gateway/model configuration and credential environment reference not yet supplied |
| Image / audio / video live derivation | BLOCKED | live role prerequisites not yet supplied |
| Real corpus import / controlled / end-to-end metrics | NOT_RUN | runner and corpus acquisition pending |
| Wave real contribution | NOT_RUN | real sourced topology signal not yet established |
| This branch Ready CI / PR / squash / deletion | NOT_RUN | acceptance pending |

Observed live source/unit/query/media counts：0/0/0/0；latency、usage、cost 和 retrieval metrics 尚无实际观测，不填估计值。

Windows 当前迭代证据：`corepack pnpm typecheck` PASS；`corepack pnpm exec vitest run apps/nous-core/tests/model-boundaries.test.ts apps/nous-core/tests/runtime.test.ts` PASS。协议 fixture 验证显式 `/v1/chat/completions`、`/v1/embeddings`、`/v1/rerank` 路径、producer prompt digest、unsafe destination、Prompt path/size/UTF-8 边界和非法 rerank index。它不证明 live provider conformance。`ai@7.0.102` 配套 `@ai-sdk/openai@4.0.67`，共享 provider 4.0.15 / utils 5.0.41。

`cargo build -p nous-kernel` PASS（Windows，linker stdout warning）；`corepack pnpm qualification:real-consumer-local` PASS：Subject `01a0f199-1130-7de0-bb9a-09721ab12efa`、Memory `01a0f199-1664-7812-8586-a210652e6f59`、revision `01a0f199-1664-7812-8586-a2209ab992c1`，`publicBoot/cli/upload/trace/meaningfulUse/restart=true`。该 corpus 明确为 synthetic local wiring，liveModel=NOT_RUN。首轮 smoke 暴露 CLI consumer ref 少一层 namespace，改为 `consumer:nous-cli:default` 后重跑 PASS；没有绕过 Runtime validator。

上传配置收敛后重新 build / public smoke PASS：Subject `01a0f1a1-9b95-7382-ba9e-489e454c6598`、Memory `01a0f1a1-9ff8-7e92-a36c-e797d6f3afc8`、revision `01a0f1a1-9ff8-7e92-a36c-e7ad1d7ab34a`。测试从 public API 读取 Kernel 配置的 1 MiB，并拒绝 1 MiB + 1 byte 上传；Core/Client 独立的 8 GiB cap 和 Core TOML duplicate 已删除。`corepack pnpm proto:check`、typecheck、Knip、窄 Oxlint 与模型/上传 fixture 均 PASS。Rerank parser 已移除 deprecated `passthrough()`，只保留经过校验的字段。

SPEC_GAP / SPEC_CONFLICT：目前未报告未决语义冲突。附件本轮决定已安装为 current Specs；缺 live 配置是 prerequisite blocker。
