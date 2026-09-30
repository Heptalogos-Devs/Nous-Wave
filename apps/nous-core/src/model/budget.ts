import { readFile, writeFile, rename, mkdir } from "node:fs/promises";
import { dirname } from "node:path";
import { randomUUID } from "node:crypto";
import { z } from "zod";

export class ModelBudget {
  private queue: Promise<void> = Promise.resolve();
  constructor(
    private readonly path: string,
    private readonly limit: number,
  ) {
    if (!Number.isInteger(limit) || limit < 1 || limit > 10000)
      throw new Error("Live model budget must be 1..10000");
  }
  reserve(): Promise<void> {
    const reserved = this.queue.then(async () => {
      let count = 0;
      try {
        count = z
          .strictObject({ count: z.number().int().nonnegative() })
          .parse(JSON.parse(await readFile(this.path, "utf8"))).count;
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
    });
    this.queue = reserved.catch(() => {});
    return reserved;
  }
}
