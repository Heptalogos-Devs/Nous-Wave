# Nous CLI

第一方 reference consumer，通过 `@nous-wave/client` 调用 Core；命令使用 citty 0.2.2，复杂输入使用 smol-toml 与 Zod。CLI 的选择、查询结果索引和操作 receipt 保存在 InstanceRoot，属于 consumer 本地状态。

按[根 README](../../README.md)准备开发环境并运行 `corepack pnpm dev`，另一个终端可执行：

```sh
corepack pnpm nous subject create
corepack pnpm nous session open
corepack pnpm nous observe text --text "实际来源的有界原文" --source "https://example.com/source"
corepack pnpm nous form <occurrence-id> --tag tag:<uuid>,tag:<lexical-ref>
corepack pnpm nous context create --purpose "继续部署评估" --text "当前任务、限制和未决问题"
corepack pnpm nous context foreground
corepack pnpm nous query '她的项目进展怎样？ $return(memory,schema) $limit(5)'
corepack pnpm nous show result:1
corepack pnpm nous trace result:1
corepack pnpm nous context pin --cognition result:1 --entity entity:alice --tag tag:<uuid>
corepack pnpm nous use result:1 --kind referenced
```

默认输出语义文本；`--json` 返回 `schemaVersion="nous.cli.v1"` 的 CLI-owned envelope，int64 使用十进制字符串。`--raw --developer` 显式选择原始 Client DTO 诊断。成功仅写 stdout，错误仅写 stderr 并返回非零码；错误保留 code、message、details、candidates 和未知结果的 receipt。

Launcher 提供 RunRoot/InstanceRoot。`--subject`、`--session`、`--work-context` 覆盖本地选择。查询和幂等修改需要 InstanceRoot 保存续接状态；只读查询准备、配置与 status 可仅指定 RunRoot。Query 的 `result:N` 始终保存实际命中的 immutable revision，并限定到原 Subject；`show/trace/use` 不将该引用替换成最新 head。Memory、Schema、Episode、Journal 均有 exact revision 读取。

`context set --text <text>` 或 `--file <path|->` 更新自由文本；`--purpose` 可同时修改目的。`pin/unpin` 使用 `--cognition`、`--entity`、`--tag` 的逗号分隔引用；cognition 只接受 exact revision 或 Occurrence。`clear --scope text|anchors|all` 清理相应字段；`pause/resume/end` 管理生命周期，`select <id>` 保存选择，`foreground [id]` 在选中 Session 激活，`foreground --clear` 解除激活。所有更新保留未修改字段并提交读取到的 expected revision。context_text 上限 64 KiB，与 query representation 预算分离。

修改前保存 operation identity、规范输入、时间戳和 expected revision。未知结果返回 `retry <receipt>`；重试读取原 receipt，不重新读取新的 revision、重新解析引用或生成 operation ID。Observe、formation、config override、Tag/Association、WorkContext 和 UseEvent 使用各 owner 的幂等身份。Session lifecycle、Identity bind 和 derivation 的领域合同单独决定重试行为。

复杂 Tag/Association 文件为语义 TOML，上限 64 KiB；没有 Client DTO JSON 输入模式。例如：

```toml
# revise-tag.toml
 target = "tag:<实际 UUID 或 LexicalRef>"
 label = "新的概念名称"
 description = "概念语义"
```

```toml
# merge-tags.toml
survivor = "tag:<实际 UUID>"
retired = ["tag:<实际 UUID>"]
[[basis]]
ref = "memory_revision:<实际 UUID>"
role = "direct"
epistemic_relation = "corroborates"
```

```toml
# association.toml
from = "memory_revision:<实际 UUID>"
to = "tag:<实际 UUID>"
relation = "tag_attachment"
polarity = "positive"
basis_class = "host_explicit"
[[basis]]
ref = "occurrence:<实际 UUID>"
role = "contextual"
```

`tag revise|merge|split --request-file <file>` 解析目标 Tag 当前修订后冻结请求；split 使用 `parent`、`[[children]]` 的 label/description/kind_hint 与 `[[basis]]`。`association create --association-file <file>` 读取 from/to/relation/polarity/basis_class/basis。`tag attach <exact cognition> --tag <tag> --association-file <file>` 的文件只有 basis，CLI 固定 exact cognition→Tag attachment。Basis role 为 direct/interpretation/contextual，epistemic_relation 独立表达 supports/contradicts/corroborates/weakens/corrects/counterexample/inferred_from；Association 也可用 `[[basis]]` 内的 `use_event={consumer="...",event="UUID"}`。Authority 验证 Subject、exact references、catalog 与 lifecycle。

`help` 与 `help nousql` 不连接 daemon。[NousQL Agent 手册](../../docs/agent/NOUSQL.md) 提供 Unicode 意图、可选语法岛、五轴时间、history/asof、direct/explore 与 projection 的可执行示例。`query prepare|inspect` 不调用 provider，输出冻结 representation、context snapshot 和能力降级；普通 query 使用同一 preparation 协议执行。

`maintenance grant --max-operations 2 --max-model-calls 1 --max-elapsed-ms 30000` 明确授权有界维护；每次实际模型调用都计入预算，包括 execution fallback。Use 接受 exact cognition revision，refutation 是负反馈，不能自动授权模型活动或概念修改。Tag list/search 与 association neighborhood 维持有界分页和遍历。
