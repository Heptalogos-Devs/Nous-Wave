# Cognitive Model & Evolution Research — 2026-10-08

[返回 Research](README.md) · [v2 报告](core-cognition-2026-10-08.md)

本轮研究正在执行。基础代码为当前 `master` `20e9bf87c2d7b0aab177ff94ec13412e29b7a8d0`，长期语义 Authority 为 Vault `5b96c63da34b0a4c697b6961ae10ba6aa4de3ee1`。新实验使用独立身份和 ignored `data/research/runs/cognitive-model-evolution-2026-10-08/`，既有 v1/v2 结果保留。

## 首批实际模型观测

New API 的启用渠道包含 `doubao-seed-2.1-lite` 与 `doubao-seed-2.1-pro`，原 Nous token 的三个模型 whitelist 未包含它们。已扩展到这两个模型，`/v1/models` 实际列出五项。Lite 的首次调用被本地 token 预扣费额度拒绝，账户额度充足；解除该 token 的额度限制后真实调用成功。失败仍计入 gateway attempt，不能算作模型能力失败。

同一 PEP 703 首单元、原 Prompt 和结构化合同、32768 output budget 的一次 default 对照中：Mini 约 9 秒、Lite 约 49 秒、Pro 约 15 秒。Lite 保留 Created 与 Accepted 的区别、3.13 目标版本与 Council provisos；Pro 也保留这些条件，但把作者 Sam Gross 错写为 Sam Gong。Mini 此次没有复制此前错误接受日期，但省略创建日期和目标版本。单次结果不能支持稳定性判断，后续使用少量重复与 reasoning 对照。

实际回包 model identifiers 为 Mini `doubao-seed-2-0-mini-260428`、Lite/Pro `doubao-seed-2-1-*-260915`；这些是 provider 返回的身份，没有独立 weights 证明。网关给所有模型的通用 endpoint labels 也不是能力实测。速度、上下文与 reasoning 结论以本轮实际 input、wire controls 和 usage 为准。

## 当前研究问题

继续比较 formation 的日期、归因、限制与否定保留；正确 Memory 的首次概念形成与错误概念复审；真实 Episode 的 Journal、consolidation/Schema 与跨 Session 分段。长期轨迹检查新来源、候选发现、MaintenanceNeed、固定输入、proposal、owner outcome 与 recall，区分自然 revision、明确 Host correction 和纯模型重新生成。

固定模型输入对照复用 [现有 longitudinal 脚本](../../scripts/research/longitudinal.ts) 的 `--invocation`，从既有 gateway capture 只取实际 user input，通过当前 ModelRuntime、PromptRegistry 和 Structured Contract Registry 生成 proposal。它记录 producer/attempt telemetry，不提交 Authority。真实演化经普通 Core 和 official Client；rerank、enrichment、Context 与 Native/VCP 的判断继续使用实际 cognition。

费用不作为主要选择约束。角色/default 配置及必要的 Prompt、RolePolicy、owner/算法修正由实际观测决定，最终结果在本页与现有 Research 体系提交。
