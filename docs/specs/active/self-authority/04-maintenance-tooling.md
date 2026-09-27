# 04 — 维护工具链配置

目标：让工具约束 AI 编码最容易发生的退化，同时保持单人 Vibe Coding 的低维护成本。

## 1. TypeScript 命令

### 日常

```text
pnpm typecheck
pnpm lint:ts
pnpm test
```

Proto修改时：

```text
pnpm proto:check
pnpm generate
```

### format

Prettier只处理代码/配置，不处理 docs Markdown。

建议：

```json
{
  "format": "prettier \"apps/**/*.{ts,tsx,js,json}\" \"packages/**/*.{ts,tsx,js,json}\" \"*.{json,js,cjs,mjs,yml,yaml}\" --write",
  "format:check": "prettier \"apps/**/*.{ts,tsx,js,json}\" \"packages/**/*.{ts,tsx,js,json}\" \"*.{json,js,cjs,mjs,yml,yaml}\" --check"
}
```

若实际目录需要增加 pattern，按真实代码添加。

### 删除 ESLint

移除：

- `eslint`
- `eslint-plugin-oxlint`
- `eslint.config.js`
- `lint:eslint`

因为当前没有任何独立规则价值。

## 2. TypeScript 维护检查

保留：

```text
knip --strict
dependency-cruiser
jscpd
sherif
dupehound
typos
```

建议组合：

```text
pnpm quality
```

只聚合上述确定性检查。

不把 Nx / publint / attw 加进来，因为当前没有对应真实用途。

## 3. Rust 日常检查

迭代时运行最窄：

```text
cargo fmt --all -- --check
cargo check -p <touched-crate>
cargo clippy -p <touched-crate> --all-features -- -D warnings
cargo test -p <touched-crate> <relevant-test-filter>
```

完成较大施工后：

```text
just verify
```

## 4. production maintainability lint

删除 pedantic/nursery 全开式 `lint-strict`。

新增：

```text
just lint-maintainability
```

作用于 lib/bin production target，不作用 tests。

检查：

```text
clippy::unwrap_used
clippy::expect_used
clippy::panic
clippy::todo
clippy::dbg_macro
clippy::print_stdout
clippy::print_stderr
clippy::allow_attributes_without_reason
```

如果某处真的需要 suppression：

-局部；
- `#[expect(..., reason = "...")]`；
- reason说明为什么此处比重构成本更低。

## 5. 大文件

`too_many_lines` 是提示重看职责，不是“超过 120 行必须拆”。

规则：

- production function因多职责变长：拆；
-一个完整 transaction/state machine天然较长：可以 `#[expect]`，写真实原因；
-测试函数/fixture长：不因为行数机械拆。

## 6. duplicate tools

### jscpd

继续 TS/JS token duplicate。

### dupehound

继续跨语言/结构性 duplicate。

### cargo-dupes

当前先人工审查 existing 10 groups。

审查后的真实 baseline写入 `dupes.toml`。

如果没有可合理合并的 group，最多：

```toml
max_exact_duplicates = 10
max_exact_percent = 6.5
```

若减少到更低，用更低数字。

目的：以后不允许继续恶化。

不得用 exclude production directory逃避。

## 7. nextest

添加：

```toml
[profile.default]
leak-timeout = "2s"
```

路径：

```text
.config/nextest.toml
```

如果 PostgreSQL测试仍 leaky，再修 helper shutdown。

不关闭 leak detection。

## 8. OSV

`just osv`：

```text
osv-scanner scan -r .
```

不写死 Windows路径。

Agent检查：

```text
cargo tree -i lru
cargo tree -i paste
```

有上游可升级版本则升级。

没有则在完成报告中记录 exact dependency chain。

不 fork依赖，不加大范围 ignore。

## 9. coverage

保留：

```text
just coverage
```

不设全局 fail-under。

看 coverage的目的是发现：

“刚改了一段复杂逻辑，但没有任何能证明它的测试。”

不是追求百分比。

## 10. mutation

不提供默认全仓 `just mutants` 要求。

需要时：

```text
cargo mutants --file <path>
```

或其他窄范围 filter。

只针对 bug-prone/nontrivial逻辑。

## 11. 完成一次较大施工时建议运行

```text
corepack pnpm check
just verify
pnpm quality
just lint-maintainability
just nextest
just feature-check
just dupes
just dupehound
just typos
just osv
```

其中：

- `corepack pnpm check` / `just verify` 应当 PASS；
-其余工具发现问题时看内容处理；
-已知无上游修复的 advisory、经过审查的机械 duplicate等允许记录，不为了“全绿”扭曲代码。

## 12. 维护原则

工具新增的长期命令、配置、ignore本身也要接受 COST 判断。

同类工具如果长期输出完全重叠，删掉价值更低的那个。
