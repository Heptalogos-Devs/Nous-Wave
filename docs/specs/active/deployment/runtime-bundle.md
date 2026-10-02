# Runtime Bundle、路径与第三方 Runtime

状态：IMPLEMENTATION-AUTHORIZING；2026-09-30 用户批准追加合同。

## 产品与 owner

一个 Nous Wave 产品按 OS/architecture 产生不同物理发布物。当前 shipping assembly 为 Windows x64 portable ZIP，其他平台采用同一逻辑合同。宿主以 sidecar 方式启动、等待 READY、通过 official Client 使用并关闭实例。

Core bootstrap 拥有路径解析、实例发现、应用启动和显式 runtime installation。Kernel 仍拥有 Authority/Serving，并管理 private PostgreSQL 的 cluster 生命周期；不增加领域、provider fleet、更新框架或第二套 persistence owner。

## RuntimeLocations

独立 logical roots：Program、Runtime、Instance、Configuration、Data、Blob、Cache、Secret、Log、Run、Temp、Backup。任何 root 都可以没有共同父目录，不能通过 CWD 或其他 root 的 `../` 推断。

`nous serve --home <home>` 将实例 roots 映射为 `instance/ config/ data/ blobs/ cache/ secrets/ logs/ run/ temp/ backups/`；Program/Runtime 默认从当前安装位置解析。无参数的 portable launcher 以安装 home 为默认。`nous serve --locator <bootstrap.toml>` 读取显式 `[paths]`；相对路径只相对 locator 文件。未知 key、重复 CLI locator/home、无有效路径必须拒绝。

ProgramRoot 是 immutable 应用 payload。RuntimeRoot 是 versioned 第三方 packs。InstanceRoot 保存稳定 instance UUID 和 private database bootstrap state。ConfigurationRoot/nous.toml 是唯一用户配置入口；bootstrap.toml 只保存路径，不保存 token 或普通认知配置。Kernel 接收解析后的非敏感配置和 paths，不读取第二份可编辑模型配置。

Portable 出厂不携带活动 nous.toml、bootstrap locator、开发配置或配置模板。`nous init` 或首次 `nous serve` 在解析后的 ConfigurationRoot 以 exclusive create 生成最小用户配置；已有配置内容保持原样。默认策略由 Core 配置 schema 拥有，初始配置仅声明 portable deployment 和 default consumer，模型/Resource 由用户配置。仓库示例属于文档，不参与 assembly。

源码开发命令以显式 development 启动 profile 选择 ProgramRoot 下的 debug Kernel 与源码 Client；该选择由命令承担，用户配置无需保存开发机 binary 路径。默认开发 ConfigurationRoot 为 ignored `data/dev/config`。

DataRoot/postgres 是 durable cluster；BlobRoot 是 Artifact/CAS；CacheRoot/serving 是可重建 Serving；RunRoot/core.json 是 endpoint/token/PID discovery；TempRoot 是短期 upload/media staging。CLI 选择状态属于 InstanceRoot/consumer，不属于 Authority。RunRoot 清理不删除 durable roots；同 instance/cluster 的启动必须互斥。SecretRoot/gateway.env 只向配置引用的 credential variables供值，OS environment 优先；Kernel/FFmpeg child 不继承 gateway token。

默认 Prompt 来自 ProgramRoot/prompts，用户 Prompt 来自 ConfigurationRoot/prompts；同一个 registry 执行 realpath/UTF-8/128 KiB/digest validation。用户 override 和默认资产的选择必须明确，缺失不静默换 prompt。ProgramRoot/runtime replacement 不删除实例配置、数据或 secret。

## 启动与生命周期

入口：`nous serve --home ...` 或 `--locator ...`；可用 stdin-close/SIGINT/SIGTERM请求 graceful shutdown。READY 只在 private database、Kernel、Core 和 discovery 均就绪后发布；stdout 只输出 redacted startup record，token 只进入受保护 RunRoot。

数据库模式为 `managed_private` 或 `external`。managed_private 只使用 RuntimeRoot 的已安装 PostgreSQL pack；首次初始化生成受保护凭据、选可用 loopback port并保存 instance bootstrap state。后续使用同一 port；冲突明确失败，不换端口。cluster/version 不一致明确失败，不自动升级、重建或丢弃数据。external 只连接明确 endpoint，不管理其 binary/cluster。

普通 serve 不联网下载 runtime。缺包明确 UNAVAILABLE，并提示 `nous runtime install <component>`。完整 bundle已包含 Node、PostgreSQL、FFmpeg。不注册系统服务、不改 global PATH、不占固定 5432。默认 ship profile 不使用 developer PATH。

FFmpeg 解析：operator explicit executable → installed verified FFmpeg pack → 明确允许的开发 PATH → unavailable。只用 argument array 执行 bounded preprocessing；temp、bytes、frames、time、cancel 均受控。

## Runtime Packs 与安装

命令：`nous runtime install/list/verify`。installer 接受 local pack 或 manifest 中明确的 URL；网络仅发生于显式 installation。catalog 与 pack manifest 固定 component/version/platform/arch/hash/license/source/build references、required executable及 closure inventory。拒绝未知/不匹配平台、checksum损坏、path traversal、archive symlink escape和超限解包。

安装到 RuntimeRoot 内独立 staging，校验后 atomic publication；只清理验证过的自有 staging路径。不能自动切换运行中的数据库或执行 major upgrade。Node、PostgreSQL、FFmpeg均 fixed version；application与runtime pack可独立组装，当前合同不授权自动更新。

FFmpeg pack由 release pipeline 从精确 source构建；禁用 GPL、nonfree、uncontrolled autodetect，携带对应 source archive/build flags/patches/license/native dependencies。之前的 Gyan GPL downloader不进入产品。PostgreSQL pack只包含server closure及必要工具/library/notices，不包含无关管理产品。

## Assembly

发布物包含 compiled Core/CLI/official Client、release Kernel、protocol、Prompt、migration closure、private runtimes。运行不需要源码仓库、pnpm、tsx或开发工具。第三方 binary不提交Git；构建/缓存/runtime data保持ignored。

当前 assembly 生成 release/component/checksum manifests、SPDX SBOM、THIRD_PARTY_NOTICES与每个runtime的license/source/build references。Nous Wave-owned code保持MIT，各third-party组件独立标注实际许可，不能把GPL/nonfree FFmpeg误报为LGPL。

Windows release verification在Git仓库外，使用任意CWD、无developer PATH和实际gateway/source，通过official Client完成 formation/embedding/rerank/NousQL/provenance/use/restart。验证共置、完全分离roots、搬移安装位置、missing-pack serve无acquisition、external override及cancel。检查结果对应实际平台与当前 artifact。

2026-10-02 用户批准 Windows shipping Kernel 改用 LLVM-MinGW UCRT 与 `--target x86_64-pc-windows-gnullvm`；assembler 只读取此 target 的 release binary 和私有 `libc++.dll`/`libunwind.dll`。`scripts/release/build-kernel.ps1` 设置独立 target C/C++/linker、C++17、source remap 与 post-link debug strip，复制精确 compiler runtime DLL。Rust MSVC source-tree 开发仍可运行，不再作为 shipping payload。以实际 PE import inventory 验证所有非系统依赖已归入 payload；系统 Win32/UCRT 不当作私有 pack。

Release compiler 为 LLVM-MinGW20260922 UCRT Windows x64，官方 archive SHA256 `e3ad77d117a4bea19a7a3b333341824d79a5a371004a10e25b8504e7b3047666`；PostgreSQL/FFmpeg 使用同发行的 Linux-host cross tools。保留 LLVM/MinGW runtime notices，逐项核查 Rust GNU self-contained/native inputs，当前 payload 保存实际 native closure。正常 serve 不获取 compiler。

Source remapping includes the checkout, Cargo registry/git sources and Rust toolchain roots. Audit the actual assembled binaries as well as source manifests; remapping only workspace paths leaves dependency panic locations tied to the development machine.

应用 bundle、runtime packs 和 notices 分别缓存。网络 license/source preparation 为显式 `release:notices`；assembly 离线读取。默认输出 `dist/portable/windows-x64/current` 与 `current.zip`，先写 sibling staging，再替换 current；只有显式 archive 固化带 source SHA/payload digest 的发布物。
