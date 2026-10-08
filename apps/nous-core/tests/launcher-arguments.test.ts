// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, it } from "vitest";
import { launcherCommandOffset } from "../src/launcher-arguments.js";

it("routes an actual Portable runtime command after private consumer options", () => {
  const args = [
    "--instance-root",
    "private-agent",
    "--consumer",
    "consumer:portable",
    "runtime",
    "list",
  ];
  expect(args.slice(launcherCommandOffset(args))).toEqual(["runtime", "list"]);
  expect(launcherCommandOffset(["runtime", "list", ...args.slice(0, 4)])).toBe(
    0,
  );
  expect(
    launcherCommandOffset([
      "--json",
      "--consumer=consumer:portable",
      "config",
      "check",
    ]),
  ).toBe(2);
});
