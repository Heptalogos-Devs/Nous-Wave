# Applications

`apps/` contains the process composition layer:

- `nous-core` is the public TypeScript host and official Client boundary;
- `nous-kernel` is the private Rust child process and composes current Rust owners.

Run the platform-neutral development path from the repository root with `corepack pnpm dev`. See the [current implementation architecture](../docs/architecture/current-implementation.md) for ownership and the local `AGENTS.md` for process-boundary rules.
