# 配置基础与社会认知 Qualification

状态：PARTIAL / ACTIVE

本记录只保存本轮实际运行的证据；长期语义见 Architecture-Vault，直接施工合同见 [active Spec set](../specs/active/configuration-foundation-and-social-cognition/README.md)。

## 已运行门禁

| Gate | 状态 | 实际命令/证据 |
| --- | --- | --- |
| Protobuf / TypeScript | PASS | `corepack pnpm check`：Buf lint、`tsc --noEmit`、Vitest 3 files/7 tests、Prettier、Oxlint、Knip、dependency-cruiser、jscpd、Sherif。 |
| Rust verification | PASS | `just verify`：fmt、source-shape、Self focused tests、workspace check、Clippy、workspace serial tests、`cargo deny`、`cargo shear`。 |
| Nextest | PASS | `just nextest`：54 tests passed，single test thread。 |
| Configuration focused | PASS | `cargo test -p nous-configuration-service --lib`：2/2；`cargo test -p nous-kernel --test configuration -- --test-threads=1`：1/1。覆盖 precedence、权限、subject override、RestartProcess pending。 |
| Cognitive Seed focused | PASS | `cargo test -p nous-cognitive-seed --lib`：2/2；Self Authority suite 4/4。覆盖 multiline Narrative、unknown field、duplicate key、v1 validation、Self semantic path/adoption。 |
| Social focused | PASS | `cargo test -p nous-kernel --test social_cognition -- --test-threads=1`：1/1。覆盖 Relation Type、directed Relationship create/revise、suppress/restore fencing、LanguageConvention SeedDirect formation、exact revision materialization 和 Social cue query。 |
| Duplicate detector | FAIL | `just dupes`：33 exact groups，threshold 16；新增 protocol adapter/owner boundary duplicates 与既有 groups 需后续逐组审计。未扩大 exclude 或提高阈值。 |
| Dupehound | PASS | `just dupehound`：grade A，slop score 1.6%。 |
| OSV | FAIL | `just osv`：当前 checkout 未安装 `osv-scanner` executable，命令未能启动；未增加 ignore。 |

## 尚未完成的 Qualification

以下状态不是实现失败，而是本轮尚未运行的专门语义场景：

- `NOT_RUN`：Social purge dedicated integration matrix；包括 purge retry/tombstone、source retention、restart/rebuild 和 dependency invalidation。A→B、revision fencing、suppress/restore 已在 focused scenario 中覆盖。
- `NOT_RUN`：LanguageConvention external actor/provenance independence 全量 acceptance matrix；当前实现已保存 `formation_policy_digest`，但完整多来源 fixture 尚未执行。
- `NOT_RUN`：Seed→Social deferred import、Relation Type conflict/unchanged、Convention unchanged/conflict recovery matrix。
- `NOT_RUN`：clean restart/rebuild 后的 mixed Memory+Self+Social serving/query recovery、full corpus/oracle/scale/benchmark gates。

## 真实告警与未决项

- `SPEC_CONFLICT`：None observed between the package and Architecture-Vault canonical design/decisions.
- `SPEC_GAP`：None for the implemented configuration, capability, Seed v1, Query lane, Relation Type, Relationship and LanguageConvention contracts. Remaining uncertainty is qualification coverage, recorded above as `NOT_RUN`.
- 有意偏离规范：None；没有通过 legacy fallback、双读双写或宽泛 advisory ignore 隐藏缺口。
