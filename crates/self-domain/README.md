# Self Domain

`self-domain` owns the value objects and validation rules for the Self Authority semantic slice:

- Self Facet and Narrative Identity identities and immutable revisions;
- Cognitive Seed adoption input types;
- Self lifecycle vocabulary, temporal fields, supports, and exact narrative references.

It does not own PostgreSQL mutation, query orchestration, Serving projection, or Kernel transport. Those boundaries are owned by [`self-service`](../self-service/README.md), `authority-store`, `cognitive-runtime`, and `apps/nous-kernel`.

Current interface and gap status: [`docs/reference/SELF.md`](../../docs/reference/SELF.md).
