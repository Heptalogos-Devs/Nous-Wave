# Repository Script Instructions

This scope owns repository checks and bounded maintenance scripts.

- Keep deterministic checks under `check/`; keep policy values under `check/config/`; keep narrowly scoped cleanup under `maintenance/`.
- Do not make scripts traverse `node_modules/`, delete Cargo `target/` by default, or touch repository data and `.codegraph` unless an explicit command scope says so.
- Cleanup scripts MUST identify exact artifact markers, support a preview mode when deletion is possible, and skip recent or active artifacts rather than using broad wildcard deletion.
- Keep verification ordering and public command names in the root `justfile`; update `scripts/README.md` when the script contract changes.
- Run the narrow script check after editing a checker and report any unrun broader gate explicitly.
