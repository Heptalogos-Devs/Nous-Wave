# Nous Wave Repository Contract

Nous Wave is PRE_PRODUCTION with no released external consumers. Architecture-Vault owns long-term cognition semantics and accepted decisions; this repository owns current implementation and executable Specs.

Read the current task contract and affected Specs before changing behavior. Internal API, schema, protocol, module and fixture replacements migrate current producers and consumers together and delete the superseded route.

TypeScript Core owns public hosting, model/resource calls and process composition. Rust semantic owners retain mutation, lifecycle and Authority meaning. `persistence` supplies database mechanics; `retrieval` supplies rebuildable Serving. `runtime` remains independent of concrete retrieval/provider implementation.

Edit canonical `proto/` and run `corepack pnpm generate`. Fresh database schema is `crates/persistence/migrations/0001_foundation.sql` through `0004_indexes.sql`; project-owned dev/test databases may be reset during schema rebases. Keep Cargo `target/` for incremental builds. Avoid traversing `node_modules/`.

Credentials stay in configured environment/SecretRoot, outside outputs and tracked files. Third-party runtimes, build cache and instance data remain ignored. Ordinary serve uses installed packs; acquisition is explicit runtime installation.

Use a semantic branch, PR to the integration branch, squash merge and branch deletion. External pushes, merges, deployment and production mutations require task authorization. Never push directly to the default branch.

The installed agency-execution skill owns general execution guidance. `just check` is the ordinary PR check; public smoke, dependency audit, research and release verification are separate task-dependent commands. Keep current architecture/state and navigation synchronized when owners or contracts change.
