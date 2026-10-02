# Nous Wave TypeScript Client

`@nous-wave/client` is the official typed TypeScript Client for the public Core API. It wraps Connect transport and generated Protobuf types; it is not a separate domain Authority.

- [Package manifest](package.json)
- [Current API reference](../../docs/reference/NOUSQL.md)
- [Protobuf source](../../proto/README.md)

Node consumers use `connectNousInstance({ runRoot })` from `@nous-wave/client/node` for authenticated local discovery. The returned client includes `artifacts.uploadFile(subjectId, path, { mediaType })` and `artifacts.uploadBytes(subjectId, bytes, { mediaType })`. File upload streams with backpressure and an exact multipart length; credentials stay inside the transport. Request options support cancellation and an upload timeout (default 300 seconds).

[返回目录](../../INDEX.md)
