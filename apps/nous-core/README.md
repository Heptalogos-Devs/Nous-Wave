# Nous Core

Nous Core is the TypeScript host process. It loads local configuration, starts the Rust Kernel, exposes the Core Connect/HTTP surface, and composes WorkContext, Projection, Context, NousQL, and model operations.

- [Current implementation architecture](../../docs/architecture/current-implementation.md)
- [NousQL reference](../../docs/reference/NOUSQL.md)
- [Package manifest](package.json)

The Core does not own canonical PostgreSQL state or the internal semantics of Rust domain owners.

Use `corepack pnpm dev` from the repository root for the platform-neutral development path. Public capability proof uses the official Client, not direct Kernel RPC.

For an external foreground launcher, `corepack pnpm start --config <core.toml> --stop-on-stdin-close` requests graceful shutdown when the launcher's stdin pipe closes. Interactive deployments can continue using SIGINT/SIGTERM. Core model clients use explicit gateway/profile/role bindings and local Prompt assets; no upstream provider credentials are managed here.
