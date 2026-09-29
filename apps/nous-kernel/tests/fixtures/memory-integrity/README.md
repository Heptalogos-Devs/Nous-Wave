# Memory integrity deterministic fixture

这是 Memory integrity 场景的离线 fixture。`corpus.json` 保存 symbolic Subject/Session/Observation/Memory 场景，`queries.json` 保存查询意图，`oracle.json` 保存 `MUST_RETURN`、`MUST_NOT_RETURN`、`MAY_RETURN` 和 diagnostics 期望。

fixture 中的 symbolic label 不承担数据库身份；integration test 将它们绑定到本次 deterministic scenario 产生的 exact revision。时间和请求身份使用固定值，测试不得以 top-1 文本相等替代 oracle 判定。
