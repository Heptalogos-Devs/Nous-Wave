# Nous Core

Nous Core is the TypeScript host process. It loads local configuration, starts the Rust Kernel, exposes the Core Connect/HTTP surface, and composes WorkContext, Projection, Context, NousQL, and model operations.

- [Current implementation architecture](../../docs/architecture/current-implementation.md)
- [NousQL reference](../../docs/reference/NOUSQL.md)
- [Package manifest](package.json)

The Core does not own canonical PostgreSQL state or the internal semantics of Rust domain owners.

Use `corepack pnpm dev` from the repository root for the platform-neutral development path. Public capability proof uses the official Client, not direct Kernel RPC.
