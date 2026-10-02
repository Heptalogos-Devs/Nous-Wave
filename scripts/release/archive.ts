import { createHash } from "node:crypto";
import { readFile, copyFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";
const current = resolve("dist/portable/windows-x64/current.zip");
const digest = createHash("sha256")
  .update(await readFile(current))
  .digest("hex");
const release = JSON.parse(
  await readFile(
    resolve("dist/portable/windows-x64/current/manifest/release.json"),
    "utf8",
  ),
) as { source_head: string };
const archive = resolve("dist/releases");
await mkdir(archive, { recursive: true });
const path = join(archive, `${release.source_head}-${digest}.zip`);
await copyFile(current, path, 1);
console.log(path);
