// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, it, vi } from "vitest";
const observed = vi.hoisted(() => ({ inherited: true }));
vi.mock("node:child_process", async (original) => {
  const actual = await original<typeof import("node:child_process")>();
  return {
    ...actual,
    spawn: (
      _command: string,
      _args: string[],
      options: import("node:child_process").SpawnOptions,
    ) => {
      observed.inherited = Object.hasOwn(
        options.env ?? {},
        "NOUS_QUALIFICATION_GATEWAY_SECRET",
      );
      throw new Error("capture launch boundary");
    },
  };
});
import { startKernel } from "../src/process.js";
it("keeps the configured gateway credential in Core and excludes it from the Kernel environment", async () => {
  const name = "NOUS_QUALIFICATION_GATEWAY_SECRET";
  const old = process.env[name];
  process.env[name] = "fixture-bearer";
  try {
    await expect(startKernel("kernel", "config")).rejects.toThrow(
      "capture launch boundary",
    );
    expect(observed.inherited).toBe(false);
    expect(process.env[name]).toBe("fixture-bearer");
  } finally {
    if (old === undefined) delete process.env[name];
    else process.env[name] = old;
  }
});
