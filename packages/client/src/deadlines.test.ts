// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { create, type DescMessage } from "@bufbuild/protobuf";
import { type Transport } from "@connectrpc/connect";
import { expect, test, vi } from "vitest";
import { createNousClient } from "./index.js";

test("maintenance honors its opportunity budget and caller deadline overrides", async () => {
  const unary = vi.fn(
    async (
      method: { output: DescMessage },
      _signal: unknown,
      _timeout: unknown,
    ) => ({ message: create(method.output) }),
  );
  const client = createNousClient({ unary } as unknown as Transport);
  const request = {
    subjectId: "s",
    maxOperations: 3,
    maxModelCalls: 3,
    maxElapsedMs: 120000,
  };
  await client.cognition.grantMaintenance(request);
  expect(unary.mock.calls[0]?.[2]).toBe(125000);
  await client.cognition.grantMaintenance(request, { timeoutMs: 1000 });
  expect(unary.mock.calls[1]?.[2]).toBe(1000);
  await client.cognition.grantMaintenance({ ...request, maxElapsedMs: 900000 });
  expect(unary.mock.calls[2]?.[2]).toBe(905000);
});
