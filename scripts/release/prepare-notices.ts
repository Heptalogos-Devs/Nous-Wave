// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { bundleApplication } from "./bundle.js";
import { prepareNotices } from "./notices.js";
import { parseArgs } from "node:util";
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const { values } = parseArgs({
  options: { "runtime-root": { type: "string" } },
});
const { packages } = await bundleApplication(repo);
await prepareNotices(
  repo,
  packages,
  values["runtime-root"] ? resolve(values["runtime-root"]) : undefined,
);
console.log("Release notices prepared");
