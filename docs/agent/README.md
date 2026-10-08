# Agent guide

Start with the self-contained [NousQL manual](NOUSQL.md). It defines the objects,
query language, time views, optional exploration and a complete Agent workflow.

Strongly prefer [explicit known referents](NOUSQL.md#prefer-explicit-referents-for-retrieval)
and searchable keywords. Uncertain referents remain valid original queries.
Create and foreground a WorkContext once, then reuse its text and real anchors.

`nous help nousql` provides a daemon-free text quick reference; use `--json` when
a program needs the structured CLI projection.
For implementation details, use the Chinese [developer reference](../reference/NOUSQL.md).

[Documentation index](../INDEX.md)
