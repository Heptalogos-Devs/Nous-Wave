# 配置示例

- [nous.toml](nous.toml)：可编辑项和模型角色的完整说明。
- [bootstrap.toml](bootstrap.toml)：独立 logical roots 的 locator 示例。
- [gateway.env.example](gateway.env.example)：仅含空值的 credential variable 示例；使用时保存到实例 SecretRoot。

这些文件供阅读和手工配置，不进入 portable payload。`nous init --home <instance>` 或首次 `nous serve --home <instance>` 在该实例 ConfigurationRoot 创建最小 `nous.toml`。已有文件保持原样；模型与外部 Resource 需要用户自行配置。
