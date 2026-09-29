# Memory Reference Qualification

状态：ACCEPTANCE AUTHORITY

本 Spec 定义能证伪 10/27 Memory Reference claim 的最小证据组合。结果只使用 `PASS`、`FAIL`、`NOT_RUN`、`BLOCKED`。

## Public acceptance

必须从平台中立的 public dev command 启动 Core，并只通过 official TypeScript Client 完成：

`Memory-only Subject → Session A/B → Observation/Artifact → grounded Memory → public typed Query recall → meaningful UseEvent + idempotent retry → full process restart → same Authority identity/revision query → suppress → restore → purge → no stale Serving recall → provenance trace-back`。

输出应包含紧凑的 stage summary、Subject/Memory/revision identity 和 restart/lifecycle/trace-back 结果；失败输出 stage 与可行动错误。

## Semantic proof

保留能保护独特 current risk 的 unit/integration tests，至少覆盖：identity/revision fencing、same-root provenance independence、aboutness isolation、Schema independent-root gate、exact Association endpoint/producer contract、query hard constraints/no first-N、stale current-head validation、lane/fusion determinism、Session isolation、UseEvent batch idempotency、suppression/restore/purge、watermark rebuild/restart。

每个 scenario 必须声明 `MUST_RETURN`、`MUST_NOT_RETURN`、`MAY_RETURN` 和 expected diagnostics；非法返回即 FAIL，不能用平均 Recall 掩盖。

## Verification classes

默认 correctness gate 只包含必要的 format/static/lint、Rust compile/Clippy、一个 Rust test runner、当前 TypeScript checks、协议生成/检查和 public acceptance。不要重复运行同一 focused test 与 full test，也不以 test count、coverage、jscpd、dupes、benchmark 或 external dataset ranking 作为完成条件。

Scale、Wave/Residual/EPA、provider comparison、latency/memory 和 LongMemEval/Memora 映射属于 research/benchmark qualification；只有 correctness gate PASS 后运行，不进入默认 product gate。

## Evidence record

Qualification 记录当前 checkout 的 commit SHA、平台/toolchain、DB/provider/embedding space、精确命令、结果、失败测试、已知限制和未运行项。`CURRENT_STATE` 只维护 capability summary 并链接本记录；历史 commit 的 PASS 不得外推为当前证据。
