// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { parseArgs } from "node:util";
import { repositoryRoot, workspacePaths } from "../workspace.js";

const execute = promisify(execFile);
const { values } = parseArgs({
  options: {
    component: { type: "string" },
    version: { type: "string" },
    source: { type: "string" },
    license: { type: "string" },
    root: { type: "string" },
    output: { type: "string", default: workspacePaths.runtime },
  },
});
if (
  !values.component ||
  !["node", "postgresql", "ffmpeg"].includes(values.component) ||
  !values.version ||
  !/^[0-9A-Za-z._-]+$/.test(values.version) ||
  !values.source ||
  !values.license ||
  !values.root
)
  throw new Error("Provide component/version/source/license/root");
if (process.platform !== "win32" || process.arch !== "x64")
  throw new Error("Runtime packing supports Windows x64 ZIP packs");
const root = resolve(values.root),
  program = resolve(values.output);
if (
  ![program, root].every((path) =>
    path.startsWith(join(repositoryRoot, "data") + sep),
  )
)
  throw new Error(
    "Runtime directory and pack output must be under repository data/",
  );
const required =
  values.component === "node"
    ? ["node.exe"]
    : values.component === "postgresql"
      ? [
          "bin/postgres.exe",
          "bin/initdb.exe",
          "bin/pg_ctl.exe",
          "bin/pg_isready.exe",
        ]
      : ["bin/ffmpeg.exe", "bin/ffprobe.exe"];
async function hash(path: string) {
  const result = createHash("sha256");
  for await (const chunk of createReadStream(path))
    result.update(chunk as Buffer);
  return result.digest("hex");
}
const files: { path: string; sha256: string }[] = [];
async function walk(directory: string) {
  for (const file of await readdir(directory, { withFileTypes: true })) {
    if (file.isSymbolicLink())
      throw new Error("Runtime pack cannot contain symlinks");
    const path = join(directory, file.name);
    if (file.isDirectory()) await walk(path);
    else if (file.isFile() && path !== join(root, "manifest.json")) {
      if (
        /^(\.env.*|gateway\.env|postgres\.password|.*\.pgpass)$/i.test(
          file.name,
        )
      )
        throw new Error("Secret material cannot enter a runtime pack");
      files.push({
        path: path
          .slice(root.length + 1)
          .split(sep)
          .join("/"),
        sha256: await hash(path),
      });
    }
  }
}
await walk(root);
files.sort((a, b) => a.path.localeCompare(b.path, "en"));
for (const executable of required)
  if (!files.some((file) => file.path === executable))
    throw new Error("Runtime source is missing a required executable");
if (!files.some((file) => /license|copying/i.test(file.path)))
  throw new Error("Runtime source is missing license notices");
const manifest = {
  component: values.component,
  version: values.version,
  platform: process.platform,
  arch: process.arch,
  license: values.license,
  source: values.source,
  required_executables: required,
  files,
};
await writeFile(
  join(root, "manifest.json"),
  JSON.stringify(manifest, null, 2) + "\n",
);
const manifestHash = await hash(join(root, "manifest.json"));
await mkdir(join(program, "packs"), { recursive: true });
await mkdir(join(program, "manifest"), { recursive: true });
const archive = `nous-runtime-${values.component}-${values.version}-windows-x64.zip`;
const systemRoot = process.env.SystemRoot;
if (!systemRoot) throw new Error("Windows SystemRoot missing");
const script = join(
  dirname(fileURLToPath(import.meta.url)),
  "../release/zip.ps1",
);
await execute(
  join(systemRoot, "System32", "WindowsPowerShell", "v1.0", "powershell.exe"),
  [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    script,
    "-SourceRoot",
    root,
    "-ArchivePath",
    join(program, "packs", archive),
  ],
  { windowsHide: true, maxBuffer: 65536 },
);
const entry = {
  component: values.component,
  version: values.version,
  platform: process.platform,
  arch: process.arch,
  sha256: await hash(join(program, "packs", archive)),
  manifest_sha256: manifestHash,
  archive,
};
const catalogPath = join(program, "manifest", "runtimes.json");
const catalog = JSON.parse(
  await readFile(catalogPath, "utf8").catch((error) => {
    if ((error as NodeJS.ErrnoException).code === "ENOENT")
      return '{"packs":[]}';
    throw error;
  }),
) as { packs: (typeof entry)[] };
catalog.packs = catalog.packs.filter(
  (pack) =>
    pack.component !== entry.component ||
    pack.platform !== entry.platform ||
    pack.arch !== entry.arch,
);
catalog.packs.push(entry);
await writeFile(catalogPath, JSON.stringify(catalog, null, 2) + "\n");
console.log(JSON.stringify(entry));
