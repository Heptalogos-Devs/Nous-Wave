# Script Owners

`dev/start.ts` owns development launch and shutdown, exposed by `pnpm dev`. `runtime/` builds and packs third-party runtimes; `release/` bundles the product. Release assembly consumes prepared packs/notices offline and replaces a single current staging.

`research/` owns live provider experiments and run budgets. `smoke/` uses normal Core hosting and the official Client.
