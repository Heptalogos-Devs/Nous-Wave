# 开发工具

通过根 `package.json` 和 `justfile` 调用脚本。目录按运行责任组织，源码入口见 [INDEX.md](INDEX.md)。

开发配置位于 ignored `data/dev/config/nous.toml`。Core 在文件缺失时创建最小配置，已有手写内容保留。发布组装不读取开发配置；portable 的 init/首次 serve 在用户实例创建配置。

运行时构建使用固定来源和工具链；准备好的 pack 可以复用。发布流程将网络 notice preparation 与离线 assembly 分开，默认替换 `dist/portable/windows-x64/current`。`release:archive` 用于保存正式发布物。

真实 retrieval/media 实验的方法与观测见 [Research](../docs/research/README.md)。更换电脑时需要携带的本地内容见 [开发环境迁移](../docs/reference/DEVELOPMENT.md)。
