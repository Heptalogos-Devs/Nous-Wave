# 05 — 验收与测试

## 1. 不建立测试数量目标

没有：

-单测数量目标；
-覆盖率目标；
-每个 public function必须测试；
-强制 TDD。

## 2. 必须保护的 Self 语义

### Seed

- immutable Seed version；
- adoption idempotency；
- same id / different digest conflict；
- import不覆盖旧历史；
-重复 `(kind,key)`拒绝。

### Self facet

- create；
- revision fencing；
- kind/key immutable；
- `evolve`历史 valid-time保留；
- support exactness；
- lifecycle；
- purge不删除 source Seed/Memory。

### Narrative

- reference必须 exact；
- reference与 evidence分离；
- dependency失效触发 revalidation；
- purge。

### Query

- Self exact mutable binding；
- SelfDirect；
- lexical Self document；
- Memory + Self mixed query不会绕过任一 owner final validation；
- Schema最后复核 current/lifecycle。

## 3. 保留的真实回归测试

把已有阶段标签文件改名后继续保留其真正有价值的测试。

补三个当前明确缺证据的场景：

1. stale lexical generation旧 revision；
2. tokenizer hit / no literal substring；
3.合法 producer的 derived association。

## 4. 测试文件大小

不设上限。

一个 integration scenario需要 600–1000 行是可以的。

只有当：

- fixture与断言已经无法定位；
-多个完全不同语义塞在同一文件；
-改一类行为经常误伤另一类测试；

才拆文件。

## 5. 不要新增的测试

不写：

- generated Proto getter；
-纯字段 mapping；
-第三方 TOML parser自身行为；
-简单 enum `as_str`；
-仅为了 coverage数字的测试；
-仅证明“尚未实现 Social/Motivation”的测试。

## 6. 施工过程检查

每次改动运行最相关测试。

例如：

-改 Seed parser → seed/self initialization tests；
-改 Query → query correctness；
-改 lifecycle → lifecycle/purge；
-改 Serving → serving/rebuild相关现有测试。

不要每改几行就跑全 workspace。

## 7. 最终验收

至少：

```text
corepack pnpm check
just verify
```

以及本次新增/修改的 Self / query integration tests。

维护工具按 `04-maintenance-tooling.md` 运行并解释结果。

## 8. 文档

完成后更新：

- `docs/current-state/CURRENT_STATE.md`
- `docs/INDEX.md`
- active plan状态
- API docs
-当前验收记录

正文使用中文术语：

- 设计决定
- 当前状态
- 实现规范
- 验收记录

只有实际文件名、类型名、命令保留英文。
