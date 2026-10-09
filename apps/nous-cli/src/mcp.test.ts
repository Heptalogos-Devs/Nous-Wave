// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from "vitest";
import { createMcpServer } from "./mcp.js";
import { Client } from "@modelcontextprotocol/client";
import { InMemoryTransport } from "@modelcontextprotocol/client";

test("MCP forwards exact argv, serializes consumer state, and preserves CLI errors", async () => {
  const seen: string[][] = [];
  let active = 0;
  let peak = 0;
  const server = createMcpServer(async (args) => {
    seen.push(args);
    peak = Math.max(peak, ++active);
    await new Promise((done) => setTimeout(done, 10));
    active--;
    return {
      content: [{ type: "text", text: args.join("\n") }],
      isError: args[0] === "bad",
    };
  });
  const client = new Client({ name: "dogfooding-regression", version: "1" });
  const [a, b] = InMemoryTransport.createLinkedPair();
  await server.connect(a);
  await client.connect(b);
  try {
    expect((await client.listTools()).tools.map((t) => t.name)).toEqual([
      "nous_help",
      "nous_command",
      "nous_query",
    ]);
    const query = "Nous MCP $return(memory) $limit(5); $(echo secret)";
    await Promise.all([
      client.callTool({ name: "nous_query", arguments: { query } }),
      client.callTool({
        name: "nous_command",
        arguments: { args: ["context", "set", "--text", "a & b"] },
      }),
    ]);
    expect(seen).toContainEqual(["query", query]);
    expect(seen).toContainEqual(["context", "set", "--text", "a & b"]);
    expect(peak).toBe(1);
    expect(
      (
        await client.callTool({
          name: "nous_command",
          arguments: { args: ["bad"] },
        })
      ).isError,
    ).toBe(true);
    expect(
      (
        await client.callTool({
          name: "nous_command",
          arguments: { args: ["query", "x", "--instance-root=other"] },
        })
      ).isError,
    ).toBe(true);
    expect(seen).not.toContainEqual(["query", "x", "--instance-root=other"]);
  } finally {
    await client.close();
    await server.close();
  }
});
