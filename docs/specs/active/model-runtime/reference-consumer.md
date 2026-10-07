# Reference Consumer

## Owner

`apps/nous-cli` is the first-party reference consumer. It uses the public `@nous-wave/client` package and its Node entry point; consumer behavior is documented in the [CLI README](../../../../apps/nous-cli/README.md).

## Boundary

The consumer connects through Core discovery in RunRoot. It stores the selected Subject, Session and WorkContext under InstanceRoot; these selections are local consumer state, not Nous Wave Authority.

Consumer operations use public Client APIs for Subject and Session lifecycle, Observation, Material derivation, Memory formation, embedding preparation, query, trace, UseEvent and WorkContext. File upload streams Artifact content through the authenticated Core endpoint.

The consumer uses citty 0.2.2 command families, Zod-validated semantic TOML inputs, default semantic text and opt-in versioned JSON. Raw Client DTO diagnostics require --raw --developer. Durable local state saves exact result:N references and operation receipts before RPC; retry replays frozen operation inputs and expected revisions. Context set/pin/unpin/clear/pause/resume/select/foreground use public WorkContext APIs. The consumer is built against the public Client surface. Provenance trace follows public Material and exact Memory/Schema/Episode/Journal revision reads from a revision through its producer and exact evidence references to the source Artifact.

[返回当前产品合同](../../INDEX.md)
