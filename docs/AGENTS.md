# Current Repository Knowledge Instructions

Human-facing maintained docs use Chinese; AI-facing instructions use concise technical English. Architecture-Vault owns long-term Target Design, decisions, rationale, and research. This tree owns current implementation, current authorization, executable Specs, and executed Qualification evidence.

Keep each current fact owned once: `docs/INDEX.md` routes, `plans/active/` authorizes, `specs/active/` contracts, `architecture/` describes current code, `current-state/` summarizes verified capability, and `qualification/` records actual commands/results.

Remove superseded current documents when Git history is sufficient. Do not document planned or historical behavior as current. When paths or owners change, update the relevant README/INDEX and run the repository Markdown link check plus `git diff --check`.
