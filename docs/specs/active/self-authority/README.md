# Self Authority 实现规范

状态：实施规范  
目录：`docs/specs/active/self-authority/`

本文档集直接约束本次代码施工。

## 阅读顺序

1. `01-shared-cognition-primitives-and-seed.md`
2. `02-self-authority.md`
3. `03-query-and-serving-integration.md`
4. `04-maintenance-tooling.md`
5. `05-verification.md`

## 上游语义

必须服从 Architecture-Vault：

- Nous Wave `TARGET_DESIGN.md` 第 2、3、4、7、10、12、13、15 章；
- `DECISIONS.md` 中关于 Cognitive Seed、Self granularity、cross-domain proposal、Authority/Runtime/Serving separation 的设计决定。

## 实施自由

Agent 可以自行决定：

- 私有 helper；
-不会改变合同的文件拆分；
-等价 SQL 写法；
-测试 fixture 的组织。

以下内容已经由本规范决定，不得交回 Agent 自行设计：

- Self 对象身份；
- revision 语义；
- Cognitive Seed format 与 adoption；
- lifecycle；
- support/provenance；
- idempotency；
- query owner；
- Serving projection；
-测试范围；
-工具链使用方式。

未决设计：无。
