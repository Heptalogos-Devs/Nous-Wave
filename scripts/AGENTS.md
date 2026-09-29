# Repository Script Instructions

Scripts under this scope are maintenance or development entrypoints. Keep destructive cleanup narrowly scoped to named project-owned artifacts, provide a preview path when deletion is possible, and never traverse `node_modules/`, delete Cargo `target/` by default, or touch repository data outside the command's explicit scope.

The canonical development entrypoint is `scripts/dev.ts`, exposed as `corepack pnpm dev`. Keep platform detection and child-process shutdown in that owner; do not duplicate it in README examples or alternate scripts.
