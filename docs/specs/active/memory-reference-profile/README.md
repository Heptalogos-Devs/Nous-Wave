# Memory Reference Profile — Executable Specs

状态：IMPLEMENTATION-AUTHORIZING

本目录是当前 Memory Reference executable Spec set。它服从 root `AGENTS.md`、当前 Cognitive Runtime and Episode Plan、10/27 milestone，以及 Architecture-Vault Target Design/Decisions。

## 阅读顺序

1. [Memory Authority & Provenance](01-memory-authority-provenance.md)
2. [Runtime & Use](02-runtime-use.md)
3. [Query & Serving](03-query-serving.md)
4. [Qualification](04-qualification.md)

## 执行原则

- 按 semantic vertical slice 闭合 `domain → persistence → protocol → service → client → query/serving → tests`。
- PRE_PRODUCTION current shape 直接替换；没有真实兼容义务时不得新增 alias、双读、双写、fallback 或 compatibility migration。
- Authority、identity/revision、lifecycle、transaction/concurrency、idempotency、purge、query consistency 和安全语义未决时，记录 `SPEC_GAP` 或 `SPEC_CONFLICT` 并停止对应局部。
- internal test PASS 不能扩大 public capability claim；Qualification 只记录实际执行的路径。
- topology/Wave/Residual/EPA 属于实验机制，除非 Qualification 明确接纳，不进入默认 Memory Reference path。
