# 当前实现与维护工具链审查

日期：2026-09-27
核查代码基线：`a5d4434a7e42e30cb2f9b39746527d75393a1da2`

## 1. 结论

当前 Memory / Cognitive Query / Serving 基础已经足以继续实现独立 Self Authority。

不另开“修复轮”。本次审查确认的问题与 Self 施工一起处理。

## 2. 当前实现值得保留的部分

已确认存在并继续沿用：

- immutable cognition revision；
- object epoch / current head fencing；
- provenance root 与独立来源判断；
- caller-stable UseEvent；
- duplicate-only UseEvent 不再推进 Session revision；
- QueryPlan 在 candidate generation 前绑定；
- Entity / Temporal 使用 Authority SQL；
- Runtime 使用 Situation refs / ResidentSet；
- fixed RRF 唯一实现位于 retrieval owner；
- Serving generation / watermark；
- CognitiveSchema synthesized independent-root 检查；
- Association exact cognition endpoint；
- Association provenance 进入 topology；
- contradiction / negative / counterexample 不进入普通非负 Wave。

## 3. 查询实现仍有的真实问题

### 3.1 candidate materialization 仍存在逐候选 SQL

`memory-service/query.rs` 中：

- `apply_lane_output()` 对 candidate 逐个解析 Memory ref；
- lexical / dense hit 逐个检查 Subject ownership；
- fusion 前又逐个调用 `memory(...)` 读取完整 MemoryView。

最终状态检查虽然已经批量化，但前面仍可能出现与候选数量成正比的数据库往返。

这在引入 Self 后会更明显，因为 Cognitive Query 将不再只有一个 domain owner。

**处理：**

下一次施工把“candidate exact ref → Authority current state / materialized hit”的批量读取做成 owner 接口。Memory 与 Self 都走批量路径，不再在 orchestrator 中逐 candidate 读取。

不为此增加查询次数测试框架；代码结构消除 N+1 后，用一个具有较多 candidates 的集成场景确认行为即可。性能 benchmark 留到系统稳定后。

### 3.2 CognitiveSchema 最终复核不完整

当前 Schema direct path 初次查询时会检查：

- current revision；
- acceptance；
- integrity；
- suppression；
- purge。

但 query 末尾 `final_schema_revision_ids()` 只重新检查 `purge_state='normal'`。

因此初次查询与最终返回之间如果 Schema 被：

- withdraw；
- suppress；
- 标记 `revalidation_required`；
- current head 改变；

旧 hit 仍存在并发窗口。

**处理：**

Schema 与 Memory 统一使用 batch final state 复核语义。普通查询要求 current + accepted + valid + normal + not purging；显式 historical exact 只放宽 current-head，不能绕过 purge。

### 3.3 未执行的三项高价值场景

当前验收记录仍明确列出：

- stale lexical generation 返回旧 revision 的完整场景未执行；
- tokenizer 命中但 literal substring 不成立的 lexical 场景未执行；
- derived Association 有合法 producer 的正向场景未执行。

这些都对应已经实现的非平凡语义，因此在下一次修改相关模块时补上。

不建立“完整 deterministic corpus”或额外测试工程。

## 4. 阶段代号污染

现有仓库里已经出现了阶段标签进入代码/测试/文档路径，例如：

- `apps/nous-kernel/tests/r2_query_correctness.rs`
- `docs/specs/active/r2-cognitive-retrieval/`
- `docs/qualification/2026-10-r2-cognitive-retrieval.md`
- 测试 consumer ref 中的 `consumer:r2:*`

阶段代号不是产品语义，不应进入长期代码。

**处理：**

下一次施工顺手改成描述性名称：

- `r2_query_correctness.rs` → `query_correctness.rs`
- `r2-cognitive-retrieval` → `cognitive-retrieval`
- `2026-10-r2-cognitive-retrieval.md` → `2026-10-cognitive-retrieval.md`
- consumer ref 改成表达测试用途的名称，例如 `consumer:test:association`

已有 Git 历史不重写。

## 5. 工具链审查

### 5.1 Prettier

当前 `.prettierignore` 为了保护 authored docs，直接忽略整个 active Spec 目录及若干文档。

这虽然能避免格式改写，但“把文档全塞 ignore”不是最清楚的职责表达。

**处理：**

Prettier 只负责 TypeScript / JavaScript / JSON / YAML 等代码与配置，不负责 Markdown 文档。

通过脚本显式限定输入范围，而不是继续扩大 Markdown ignore。

### 5.2 ESLint

当前 ESLint 配置直接忽略 `*.ts` / `*.tsx`，且没有配置任何 Oxlint 尚未覆盖的框架插件规则。

因此它当前没有足够独立价值。

**处理：**

删除 ESLint 与 `eslint-plugin-oxlint`，以及对应脚本/config。

以后如果出现 Oxlint 明确覆盖不了、且确有价值的框架规则，再重新引入。

### 5.3 Oxlint

保留为 TypeScript 主 lint。

当前为 TypeScript 7 / type-aware 模式关闭的若干已知不兼容规则保留局部配置；不得为了新错误继续扩大关闭列表。

### 5.4 Knip / dependency-cruiser / Sherif / jscpd / dupehound

保留。

当前 dependency-cruiser 只有：

- cycle；
- Core 不依赖 Kernel / Rust owner；

这正好。暂不增加“大企业分层规则矩阵”。

jscpd 与 dupehound继续负责发现 AI 复制实现。

### 5.5 cargo-dupes

当前 exact duplicate baseline 已随 Self vertical slice 复核为 16 组；这些组主要来自 tonic RPC 转发、生命周期 wrapper、枚举映射和 owner 边界的机械重复。

不能靠删除工具解决，也不应为“全绿”机械抽象 tonic RPC 转发、生命周期 wrapper、枚举映射。

**处理：**

先人工检查当前 16 组：

- 真正重复业务逻辑：合并；
- 机械边界代码：保留。

检查后将当前已接受基线写成“不能继续变差”的上限：

```toml
max_exact_duplicates = 16
max_exact_percent = 6.5
```

如果人工检查后实际下降，则用新的实际值作为基线，不故意保留到 10 / 6.5。

以后新增重复会让检查失败，而不是永久忽略历史。

### 5.6 Clippy

普通 `just lint` 已经包含项目真正关心的：

- suspicious；
- perf；
- complexity；
- too_many_lines；
- excessive_nesting；
- too_many_arguments；
- type_complexity；
- large_futures；
- large_enum_variant 等。

当前 `lint-strict` 再整体启用 pedantic + nursery，产生 25 个文档/风格类失败，信息密度很低。

**处理：**

删除当前 `lint-strict`。

新增 production-only 的维护检查，只检查真正希望禁止的行为，不对 test target 禁止 `unwrap/expect`：

```text
unwrap_used
expect_used
panic
todo
dbg_macro
print_stdout
print_stderr
allow_attributes_without_reason
```

执行范围：workspace 的 lib/bin production targets。

测试里允许 `expect` / `unwrap`，因为测试失败信息本来就需要快速、直接。

### 5.7 nextest

初次审查时 35/35 通过，但 embedded PostgreSQL 场景被标记 leaky；原因是测试调用方先 drop `TempDir`、后 drop `PostgreSQL`，临时根无法在进程停止前删除。

先增加 repository `.config/nextest.toml`：

```toml
[profile.default]
leak-timeout = "2s"
```

给 embedded PostgreSQL 正常退出时间；仓库验证入口另外使用单测试线程，避免初始化阶段的资源争用。

当前已修复 test helper 的返回/绑定顺序，使 `PostgreSQL` 先停止、`TempDir` 后删除。`just nextest` 当前为 48/48 PASS，测试结束后的精确进程/临时根核验均为 0。保留 leak detection，不把测试标成永久 ignore。

### 5.8 coverage

当前约 31% line coverage。

不设置 80% 等全局覆盖率指标。

coverage 只用于定位“复杂、易错、实际修改过但没有任何有效测试覆盖”的区域。

简单映射、generated code、机械 wrapper 不为了数字补测试。

### 5.9 cargo-mutants

不运行全 workspace mutation 作为每次施工要求。

只在以下情况使用：

- 修复过真实 bug 的算法；
- 非平凡 parser / state transition；
- ranking / provenance / identity 等难靠普通 happy-path 测试证明的逻辑。

优先按文件/模块缩小范围。

### 5.10 OSV

`just osv` 当前写死 Windows PowerShell + GOPATH `.exe` 路径。

改成平台无关：

```text
osv-scanner scan -r .
```

OSV 用于发现依赖问题，不要求所有无上游修复的 transitive advisory 都让开发停止。

当前：

- `lru 0.16.4` 已有 cargo-deny 精确例外，并注明 Tantivy 约束；
- `paste 1.0.15` 没有修复版。

下一次施工先用 `cargo tree -i` 查实际依赖链。有可升级路径就升级；没有则记录来源，不 fork、不增加假修复。

`cargo deny` 继续作为 Rust 依赖政策的确定性检查。

## 6. 测试原则

项目测试遵循：

1. 已经发生过的 bug，写 regression test；
2. identity / provenance / lifecycle / idempotency / query correctness 等高代价错误，写合同测试；
3. 非平凡算法需要能区分错误实现与正确实现的测试；
4. 简单 getter、机械转换、generated code 不为了 coverage 写测试；
5. 不因“测试文件超过几百行”自动拆分；
6. 只有当测试文件已经难以定位 fixture / ownership，才按语义拆；
7. 不为了测试方便改生产架构。

## 7. 结论

下一次施工同时做：

- 上述小范围查询正确性/维护修正；
- 工具链降噪与基线化；
- 独立 Self Authority；
- Cognitive Seed 正式化；
- Cognitive Query 从 Memory-only contributor 演化为可容纳 Self 的共享认知查询。

不另开修复轮，不新增阶段代号。
