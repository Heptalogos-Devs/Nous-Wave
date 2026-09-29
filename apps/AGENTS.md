# Application Process Instructions

`apps/` owns process composition, not domain Authority.

TypeScript Core is the public host and official Client boundary. Rust Kernel is a private authenticated loopback child process. Public capability proof starts through Core/official Client; direct Kernel calls are internal evidence only.

Keep Kernel endpoint discovery parent-owned and ephemeral. Remove bootstrap flags or transport routes that do not control runtime behavior. Update the public run path when process composition changes.
