# Nous Wave 实现文档

本目录维护会随当前 checkout 变化的工程知识。它是实现层文档系统，不是长期目标设计的第二份 Authority；长期语义、已接受决定、理由和研究由 [Architecture-Vault](https://github.com/Heptalogos-Devs/Architecture-Vault) 持有。

## 本目录的分工

- 本页（`docs/README.md`）说明读者路径、文档类别和维护规则。
- [`docs/INDEX.md`](INDEX.md) 是维护文档的唯一目录，按读者目标组织链接。
- `architecture/` 只描述当前代码 owner 和运行边界。
- `current-state/` 只记录可由源码、合同、测试或命令核对的当前状态和 gap。
- `plans/active/` 负责当前施工授权；`specs/active/` 负责直接实施合同；`qualification/` 负责实际证据。
- `reference/` 负责当前接口行为，不承担长期 ontology 设计。

## 建议阅读路径

1. 想了解系统边界：读[当前实现架构](architecture/current-implementation.md)。
2. 想判断能力是否已实现：读[当前状态](current-state/CURRENT_STATE.md)。
3. 想施工：先读[计划目录](plans/README.md)，再读对应 active Spec。
4. 想核对当前接口：读[参考目录](reference/README.md)。
5. 想查看证明：读[Qualification 目录](qualification/README.md)。

计划链为 `Target Design → Target Engineering Plan → Active Milestone Plan → Executable Specs → Code → Qualification`。实现文档必须区分 Current、Planned、Historical、NOT_RUN 和 `SPEC_GAP`，不能把目标或历史证据写成当前实现。

Active Executable Spec 是直接实施合同，保留其原始 revision 和技术文本；它不参与普通人类 prose 的翻译或改写。
