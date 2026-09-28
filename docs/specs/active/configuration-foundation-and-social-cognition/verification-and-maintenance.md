# 验收、测试与维护

## 1. 原则

没有测试数量目标，没有覆盖率目标。只保护已观测 bug、高代价 Authority/identity/lifecycle错误、非平凡 query/fusion、social evidence acceptance、Seed import恢复。

## 2. 必须有的 focused scenarios

### Composition / Configuration

0. Memory-only capability下 Self/Social均关闭，Memory形成、query、Serving、restart正常。
0.1 显式查询 disabled Self/Social返回 Unavailable，不返回假空结果。
0.2 修改 RRF/Wave/accessibility representative config无需改代码即可改变对应 deterministic fixture。
0.3 Serving config digest随 topology policy变化。


Seed/Self：完整 Narrative multiline；unknown field拒绝；SeedSupportRef path；Self key边界；scope规则；Self Seed import重试；新 Seed不覆盖已演化 Self。

Query：Memory+Self+Social同一次 fusion；同 exact ref多 lane只聚合一次；仓库只有 cognitive-retrieval拥有RRF公式；Self/Social无逐 candidate materialization SQL；stale exact binding继续成立。

Relationship：A→B不自动B→A；Symmetric/Inverse view返回同一 revision不生成第二 object；degree semantics校验；degree与epistemic独立；evolve保留历史；lifecycle/purge保留 source。

LanguageConvention：explicit external explanation accepted；contextual guess拒绝；same-root repeated use不满足；reference default 下 2 个 independent repeated use 满足；修改 `repeated_external_use_min` 后 acceptance 随配置变化；successful_understanding+later continued use遵守 same-actor policy；actor mismatch拒绝；Subject output不计外部证据；Dyad canonicalization；scope过滤；同 expression排序遵守配置的 scope preference。

Seed→Social：Relation Type same definition unchanged/different conflict；Relationship不覆盖演化；Convention seed_direct保留 path；Social disabled deferred。

## 3. 不需要的测试

不写 generated getter、trivial constructor、第三方 serde/toml行为、纯 coverage branch、无语义 smoke duplication。

## 4. 测试文件

不设行数限制。可按 `social_authority.rs`、`social_query.rs`、`cognitive_seed.rs` 语义拆分；完整场景数百行允许。

## 5. 当前认知检索验收记录

更新 `docs/验收/2026-10-cognitive-retrieval.md`：标题/正文用中文“认知检索验收记录”；删除活动正文旧阶段编号措辞；single fusion owner仅在 query refactor完成后记 PASS；stale lexical/tokenizer/derived producer用当前已执行测试更新证据；benchmark仍 NOT_RUN，不为文档数字补跑。

## 6. 命令

施工时运行 touched crate最窄检查。完成后至少：

```text
corepack pnpm check
just verify
just nextest
```

涉及 duplicate/source shape自然变化时运行 just dupes/dupehound/structure。

`just osv`若仍只有无上游修复/transitive advisory，记录 dependency chain，不 fork或宽泛 ignore。

## 7. source shape

调整 source-shape：把 `tests` 加入 `EXCLUDED_PARTS`；对 authored Rust production 继续 warn >600 lines、fail >1000。测试文件不参与行数阈值。warning不是自动拆文件理由，按职责判断。

## 8. cargo-dupes

当前 baseline 16 exact groups。本轮真实值下降就同步下降；新增 group先判断业务重复还是机械边界代码；不扩大 exclude隐藏 production duplicate，不为了数字过度抽象。

## 9. 文档

更新 CURRENT_STATE、current implementation architecture、受影响 API reference、docs/INDEX、plans README、active plan状态、当前验收记录。

正文用中文“设计决定 / 实现规范 / 当前状态 / 验收记录”；类型名、crate、命令、路径保持英文。

## 10. 完成报告

必须给出 commit SHA、Seed/Self收口、query唯一融合、Social对象、persistence/Proto/Rust/TS Client、focused tests保护的语义、corepack pnpm check/just verify/just nextest、source-shape/duplicate/OSV真实结果、未完成项、SPEC_CONFLICT/SPEC_GAP、有意偏离规范（正常应无）。

不要用测试数量、覆盖率、工具全绿代替语义完成说明。


## Configuration Service 验收补充

必须至少有少量高信息量场景证明：

- Memory-only 默认组合完整工作；
- Self/Social process/subject disabled 不污染 Memory query/Serving；
- duplicate config key与 unknown settings key被拒绝；
- file/system/subject override顺序正确；
- Advanced user不能修改 Developer key，Standard user不能修改 Advanced key；
- system invariant没有对应可修改 key；
- RestartProcess变更显示 pending而不偷换当前 active snapshot；
- operation固定 snapshot；
- Accessibility/RRF/Wave各选一个代表参数证明不改代码即可改变 deterministic behavior；
- Social LanguageConvention revision保存 formation policy digest。

不为每个 config leaf重复写 getter test。
