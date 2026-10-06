// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";
import { workspacePaths } from "../workspace.js";

const execute = promisify(execFile);
const version = "2.13.9";
const digest =
  "a2c9ae7b770da34860050c309f903221c67830c86e4a7e760692b803df95143a";

/** PostgreSQL's Linux archive needs the libxml2.so.2 ABI on Ubuntu 26. */
export async function preparePostgresXmlLibrary() {
  const root = join(workspacePaths.tools, "libxml2-dev");
  const installed = join(root, "installed");
  const library = join(installed, "lib/libxml2.so.2");
  if (!(await stat(library).catch(() => undefined))) {
    await mkdir(root, { recursive: true });
    const archive = join(root, `libxml2-${version}.tar.xz`);
    let bytes = await readFile(archive).catch(() => undefined);
    if (!bytes) {
      const response = await fetch(
        `https://download.gnome.org/sources/libxml2/2.13/libxml2-${version}.tar.xz`,
        { signal: AbortSignal.timeout(60000) },
      );
      if (!response.ok)
        throw new Error(`libxml2 download: HTTP ${response.status}`);
      bytes = Buffer.from(await response.arrayBuffer());
    }
    if (createHash("sha256").update(bytes).digest("hex") !== digest)
      throw new Error("Developer libxml2 source checksum mismatch");
    await writeFile(archive, bytes);
    await execute("tar", ["-xJf", archive, "-C", root]);
    const cwd = join(root, `libxml2-${version}`);
    const options = { cwd, maxBuffer: 8 * 1024 * 1024, timeout: 300000 };
    await execute(
      "./configure",
      [
        "--without-python",
        "--without-icu",
        "--without-lzma",
        "--disable-static",
        `--prefix=${installed}`,
      ],
      options,
    );
    await execute("make", ["-j4"], options);
    await execute("make", ["install"], options);
  }
  return join(installed, "lib");
}

export async function installPostgresXmlLibrary(
  installation: string,
  libraryRoot: string,
) {
  await mkdir(join(installation, "lib"), { recursive: true });
  await copyFile(
    join(libraryRoot, "libxml2.so.2"),
    join(installation, "lib/libxml2.so.2"),
  );
}
