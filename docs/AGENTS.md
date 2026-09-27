# Documentation Instructions

This scope contains implementation-facing Nous Wave documentation. Keep the human corpus in Chinese; keep this file and all AI-facing instructions in concise technical English.

- Architecture-Vault owns long-term target design, decisions, rationale, and research. Do not maintain a second copy of that design here.
- Verify current implementation statements against current source, protocol, manifests, and tests. Mark target or planned capabilities explicitly.
- Current status and implementation gaps belong under `current-state/`; execution authorization belongs under `plans/active/`; current code architecture belongs under `architecture/`.
- `README.md` explains this documentation system; `INDEX.md` is its catalog. Do not duplicate the catalog or make a topic document a competing entry point.
- Update `INDEX.md` and the affected scope README when paths or owners change. Keep one canonical owner for each current fact and link to it from summaries.
- Preserve direct active Spec text and its revision identity. Change implementation docs or source when the current implementation changes; do not silently rewrite a Spec to fit code.
- Treat direct Spec files as contract artifacts rather than localization targets; preserve their source language and exact content while active.
- Run the repository link/format checks relevant to the changed documentation and report unexecuted checks explicitly.
