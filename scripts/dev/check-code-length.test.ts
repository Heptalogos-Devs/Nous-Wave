// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";
import { expect, test } from "vitest";
import { repositoryRoot, workspacePaths } from "../workspace.js";

const execute = promisify(execFile);

test("checks physical boundaries and the actual tracked/untracked worktree with strict configuration", async () => {
  const temporary = await mkdtemp(
    join(workspacePaths.temporary, "code-length-"),
  );
  const command = [
    join(repositoryRoot, "node_modules/tsx/dist/cli.mjs"),
    join(repositoryRoot, "scripts/dev/check-code-length.ts"),
  ];
  const options = { cwd: temporary, windowsHide: true };
  const run = async () => {
    try {
      const output = await execute(process.execPath, command, options);
      return { ...output, code: 0 };
    } catch (error) {
      return error as { stdout: string; stderr: string; code: number };
    }
  };
  try {
    await execute("git", ["init", "-q"], options);
    await mkdir(join(temporary, ".config/scripts"), { recursive: true });
    const configPath = join(temporary, ".config/scripts/code-length.toml");
    const config = await readFile(
      join(repositoryRoot, ".config/scripts/code-length.toml"),
    );
    await writeFile(configPath, config);
    await writeFile(
      join(temporary, "边界 tracked.ts"),
      "\ufeff" + "a\r\n".repeat(799),
    );
    await writeFile(join(temporary, "deleted.rs"), "a\n".repeat(1000));
    await writeFile(join(temporary, ".gitignore"), "ignored.ts\n");
    await execute("git", ["add", "."], options);
    await rm(join(temporary, "deleted.rs"));
    await writeFile(join(temporary, "new.cts"), "a\r".repeat(800));
    await writeFile(join(temporary, "warning.rs"), "a\n".repeat(599) + "a");
    await writeFile(join(temporary, "empty.mts"), "");
    await writeFile(join(temporary, "ignored.ts"), "a\n".repeat(1200));
    const generated = join(temporary, "crates/protocol/src/generated");
    await mkdir(generated, { recursive: true });
    await writeFile(join(generated, "wire.rs"), "a\n".repeat(1000));
    const warnings = await run();
    expect(warnings.code).toBe(0);
    expect(warnings.stdout).toContain("WARN new.cts 800 lines");
    expect(warnings.stdout).toContain("WARN warning.rs 600 lines");
    expect(warnings.stdout).toContain(
      "4 files, 2 warnings, 0 rejected, 0 read errors",
    );
    await writeFile(join(temporary, "边界 tracked.ts"), "a\n".repeat(1200));
    await writeFile(join(temporary, "new.rs"), "a\n".repeat(1000));
    const rejected = await run();
    expect(rejected.code).toBe(1);
    expect(rejected.stdout).toContain("ERROR 边界 tracked.ts 1200 lines");
    expect(rejected.stdout).toContain("ERROR new.rs 1000 lines");
    await writeFile(configPath, config.toString() + "\nunknown = true\n");
    const invalid = await run();
    expect(invalid.code).toBe(2);
    expect(invalid.stderr).toContain("unknown");
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
});
