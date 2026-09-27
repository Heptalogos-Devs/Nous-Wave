# Execution Contract

## Authority

- Architecture-Vault owns long-term target semantics, accepted design decisions, rationale, and long-term research.
- This repository owns current implementation behavior, code-level contracts, implementation plans, and verification evidence.
- Current source, Protobuf definitions, manifests, and tests establish what this checkout implements. Do not infer implementation from target design or earlier documentation.
- `docs/plans/active/` contains current implementation authorization. Do not start code work from a completed, superseded, or research-only document.

## Documentation roles

- `README.md` is the human entry point and shortest verified path; `INDEX.md` is the repository map; `AGENTS.md` is AI operational context. Keep these roles separate.
- `docs/README.md` explains the documentation system; `docs/INDEX.md` is the maintained documentation catalog. Do not create competing catalogs or copy complete topic documents into entry points.
- When an important boundary or path changes, update the affected README and both repository/documentation indexes. Keep detailed current behavior in its canonical reference or machine-readable contract.

## Architecture boundaries

- Keep Subject Core, Cognitive Runtime, Memory, material/evidence, Authority Store, retrieval, and Serving behind their current owners.
- Memory is optional to the runtime composition. Do not move Subject Core or generic Cognitive Runtime ownership into Memory.
- Long-term target semantics for Memory classes, CognitiveSchema, Self, Social Cognition, Motivation, and cross-system behavior are owned by Architecture-Vault. This codebase does not maintain a second target ontology.
- Current code implements TypeScript Core + Rust Kernel. Do not claim Self/Social/Motivation, Desired Condition, Pursuit, or Heptalogos live cognition integration is implemented without current code, protocol, and test evidence.
- Current code contains EPA/Residual and bounded Wave implementations. They are not permanent design authority or production-default claims. VCP is research lineage; its source code is not copied.

## Implementation and verification

- Use the dependency and tool routes selected in workspace manifests and lockfiles. Prefer suitable mature libraries for generic mechanics, behind Nous-owned interfaces.
- Compatibility is required only for an explicit current obligation. TDD is optional; tests protect current contracts, observed risks, or meaningful uncertainty.
- During iteration, use the narrowest useful check. At an authorized acceptance boundary use `corepack pnpm check` and `just verify` as required by the active Plan.
- Do not hand-edit generated protocol output. Change `proto/` and generation inputs, then regenerate and verify the Rust/TypeScript consumers.
- Do not traverse or clean `node_modules/`. Keep Cargo `target/` for incremental compilation; `just clean-build` is an explicit space-recovery operation, not a routine verification step. Use `just clean-test-temp` only for its exact embedded PostgreSQL temporary-directory scope.
- Do not add speculative fallback paths, schedulers, validators, recovery layers, or verification processes for hypothetical future work.
- Report evidence as `PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED` and keep each claim within what actually ran.
