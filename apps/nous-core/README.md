# Nous Core

Nous Core is the TypeScript host process. It loads local configuration, starts the Rust Kernel, exposes the Core Connect/HTTP surface, and composes WorkContext, Projection, Context, NousQL, and model operations.

- [Current implementation architecture](../../docs/architecture/current-implementation.md)
- [NousQL reference](../../docs/reference/NOUSQL.md)
- [Package manifest](package.json)

The Core does not own canonical PostgreSQL state or the internal semantics of Rust domain owners.

Use `corepack pnpm dev` from the repository root for the platform-neutral development path. Public smoke uses normal Core startup and the official Client.

For an external foreground launcher, `nous serve --locator <bootstrap.toml> --stop-on-stdin-close` requests graceful shutdown when the launcher's stdin pipe closes. Interactive deployments can continue using SIGINT/SIGTERM. `ConfigurationRoot/nous.toml` owns user settings and `RunRoot/core.json` supplies official Client discovery. Core model clients use explicit gateway/profile/role bindings and local Prompt assets; no upstream provider credentials are managed here.

[返回目录](../../INDEX.md)
