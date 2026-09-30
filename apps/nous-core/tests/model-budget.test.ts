import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { expect, it } from "vitest";
import { ModelBudget } from "../src/model/budget.js";
it("serializes concurrent reservations and retains the hard cap after restart", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-budget-"));
  try {
    const path = join(root, "budget.json"),
      budget = new ModelBudget(path, 3);
    const requests = await Promise.allSettled(
      Array.from({ length: 8 }, () => budget.reserve()),
    );
    expect(
      requests.filter((request) => request.status === "fulfilled"),
    ).toHaveLength(3);
    expect(JSON.parse(await readFile(path, "utf8"))).toEqual({ count: 3 });
    await expect(new ModelBudget(path, 3).reserve()).rejects.toThrow(
      "exhausted",
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
