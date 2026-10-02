---
name: preproduction-rebase
description: Replace internal Nous Wave API, schema, protocol, module or durable shapes in PRE_PRODUCTION.
---

# Internal Replacement

Nous Wave has no released external consumers. Migrate current producers and consumers together, regenerate canonical Protobuf bindings and delete the old shape. Update the fresh schema and reset project-owned dev/test databases when required. Remove superseded fixtures and current documents; Git retains history.
