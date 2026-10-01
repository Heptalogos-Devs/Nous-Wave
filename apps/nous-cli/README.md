# Nous CLI

第一方 reference consumer，只使用 `@nous-wave/client` 和 Node 标准库。Core 的本地 discovery 文件提供连接信息，CLI 只保存当前 Subject/Session/WorkContext 的选择。

先按[根 README](../../README.md)安装依赖、构建 Kernel，并显式运行 `cargo run -p nous-kernel --example qualification_postgres` 准备开发数据库；再运行 `corepack pnpm dev`。另一个终端：

```powershell
corepack pnpm nous status
corepack pnpm nous subject create
corepack pnpm nous session open
corepack pnpm nous observe text --text "实际来源中的有界原文" --source "https://example.com/source"
corepack pnpm nous observe file ./sample.png
corepack pnpm nous derive <source-region-id> --strategy describe_then_structure
corepack pnpm nous form <occurrence-id>
corepack pnpm nous embeddings prepare --max-calls 16
corepack pnpm nous query '"检索线索" $memory $limit(5)'
corepack pnpm nous trace memory:<memory-id>
corepack pnpm nous use memory_revision:<revision-id>
```

用 launcher 的 `--home <path>` / `--locator <bootstrap.toml>` 选择实例，用 `--json` 输出机器可读 JSON；int64 用十进制字符串。Consumer 接收独立 RunRoot/InstanceRoot，不推测 DataRoot。`subject use <id>` 和 `session show/close` 管理当前选择；`context create --text <purpose>`、`context foreground/show/end` 操作有界 WorkContext。

文件上传使用有界 stream，不整份读取到内存。`--media-type` 可以明确 MIME；`--source` 保存可追踪外部来源。Formation/embedding 需要 Core 的模型角色；凭据仅从配置指定的环境变量读取。当前功能和实际验收状态见 [本轮 Qualification](../../docs/qualification/2026-09-30-real-usage-retrieval.md)。
