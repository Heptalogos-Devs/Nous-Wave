# Nous Wave

PRE_PRODUCTION; no released external consumers. Internal replacements update current producers/consumers together and delete superseded shapes. Architecture-Vault owns long-term cognition semantics and accepted decisions. Executable contracts are routed through [docs/INDEX.md](docs/INDEX.md).

TypeScript Core owns public hosting and model/resource execution. Rust semantic owners hold Authority, mutation and lifecycle meaning. `persistence` supplies database mechanics; `retrieval` supplies rebuildable Serving. `runtime` is independent of concrete retrieval/provider implementations.

Canonical wire contracts live in `proto/`; regenerate bindings with `corepack pnpm generate`. Fresh database schema is `crates/persistence/migrations/0001_foundation.sql` through `0004_indexes.sql`.

Development configuration is ignored `data/dev/config/nous.toml`; secrets use SecretRoot/environment. Portable payloads contain no operator configuration. Runtime acquisition is explicit; ordinary serve uses installed packs.

Default branches are integration-only: PR and squash merge, never direct push.
