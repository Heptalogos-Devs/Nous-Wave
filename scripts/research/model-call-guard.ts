// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { readFile, writeFile, rename, mkdir } from "node:fs/promises";
import { dirname } from "node:path";
import { randomUUID } from "node:crypto";

export class ResearchModelCallGuard {
  private queue: Promise<void> = Promise.resolve();
  constructor(
    private readonly path: string,
    private readonly limit: number,
  ) {
    if (!Number.isInteger(limit) || limit < 1 || limit > 100000)
      throw new Error("Live model budget must be 1..100000");
  }
  reserve(): Promise<number> {
    const reserved = this.queue.then(async () => {
      let count = 0;
      try {
        const ledger: unknown = JSON.parse(await readFile(this.path, "utf8"));
        if (
          !ledger ||
          typeof ledger !== "object" ||
          !("count" in ledger) ||
          Object.keys(ledger).length !== 1 ||
          typeof ledger.count !== "number" ||
          !Number.isSafeInteger(ledger.count) ||
          ledger.count < 0
        )
          throw new Error("Invalid research call ledger");
        count = ledger.count;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT")
          throw new Error("Model budget ledger is unavailable or invalid", {
            cause: error,
          });
      }
      if (count >= this.limit)
        throw new Error("Live model call budget exhausted");
      await mkdir(dirname(this.path), { recursive: true });
      const temporary = this.path + "." + randomUUID() + ".tmp";
      await writeFile(temporary, JSON.stringify({ count: count + 1 }), {
        flag: "wx",
        mode: 0o600,
      });
      await rename(temporary, this.path);
      return count + 1;
    });
    this.queue = reserved.then(() => {}).catch(() => {});
    return reserved;
  }
}
