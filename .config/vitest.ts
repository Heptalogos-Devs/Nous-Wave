import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { defineConfig } from "vitest/config";
import { workspacePaths } from "../scripts/workspace.js";

const temporary = join(workspacePaths.temporary, "tests");
mkdirSync(temporary, { recursive: true });
process.env.TMP = temporary;
process.env.TEMP = temporary;
process.env.TMPDIR = temporary;

export default defineConfig({
  test: {
    include: [
      "apps/**/*.test.ts",
      "packages/**/*.test.ts",
      "scripts/**/*.test.ts",
    ],
  },
  cacheDir: join(workspacePaths.cache, "vitest"),
});
