// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { workspacePaths } from "../workspace.js";
import { createHash } from "node:crypto";
import { readFile, copyFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
const current = join(workspacePaths.releases, "windows-x64/current.zip");
const digest = createHash("sha256")
  .update(await readFile(current))
  .digest("hex");
const release = JSON.parse(
  await readFile(
    join(workspacePaths.releases, "windows-x64/current/manifest/release.json"),
    "utf8",
  ),
) as { source_head: string };
const archive = join(workspacePaths.releases, "archive/windows-x64");
await mkdir(archive, { recursive: true });
const path = join(archive, `${release.source_head}-${digest}.zip`);
await copyFile(current, path, 1);
console.log(path);
