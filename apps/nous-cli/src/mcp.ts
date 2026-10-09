// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { McpServer, type CallToolResult } from "@modelcontextprotocol/server";
import { z } from "zod";

type Execute = (args: string[]) => Promise<CallToolResult>;

/** Honor an explicit opportunity through the process boundary; ordinary calls stay bounded. */
function cliCommandTimeoutMs(args: readonly string[]) {
  const command = args.indexOf("maintenance");
  if (command < 0 || args[command + 1] !== "grant") return 360000;
  const option = args.findIndex((arg) => /^--max-elapsed-ms(?:=|$)/.test(arg));
  if (option < 0) return 360000;
  const text = args[option]!.includes("=")
    ? args[option]!.slice(args[option]!.indexOf("=") + 1)
    : args[option + 1];
  const elapsed = text && /^\d+$/.test(text) ? Number(text) : NaN;
  return Number.isSafeInteger(elapsed) && elapsed > 0 && elapsed <= 900000
    ? Math.max(360000, elapsed + 15000)
    : 360000;
}

/** One connection owns one CLI working set; concurrent tool requests cannot race it. */
export function createMcpServer(execute: Execute) {
  const server = new McpServer({ name: "nous", version: "0.1.0" });
  let pending: Promise<unknown> = Promise.resolve();
  const invoke = (args: string[]) => {
    if (
      args[0] === "mcp" ||
      args.some((arg) =>
        /^--(?:run-root|instance-root|state-root|consumer)(?:=|$)/.test(arg),
      )
    )
      return Promise.resolve<CallToolResult>({
        content: [
          {
            type: "text",
            text: "MCP connection owns run-root, state-root and consumer; use --subject/--session for cognition selection.",
          },
        ],
        isError: true,
      });
    const result = pending.then(() => execute(args));
    pending = result.catch(() => undefined);
    return result;
  };
  server.registerTool(
    "nous_help",
    {
      description:
        "Read Nous CLI help without a daemon. Start here, then nous_help topic=nousql. All output is the native CLI text.",
      inputSchema: z.object({ topic: z.string().max(64).optional() }),
    },
    ({ topic }) => invoke(["help", ...(topic ? [topic] : [])]),
  );
  server.registerTool(
    "nous_command",
    {
      description:
        "Execute native Nous CLI argv (no shell). Discover commands with nous_help. This connection has isolated selection, exact result:N references and retry receipts. Default semantic text; use --json only when needed. Failed commands preserve the native error and retry receipt. Subject/Session selection is explicit or durable locally.",
      inputSchema: z.object({
        args: z.array(z.string().max(65536)).min(1).max(128),
      }),
    },
    ({ args }) => invoke(args),
  );
  server.registerTool(
    "nous_query",
    {
      description:
        "Query Nous with Unicode intent and optional NousQL syntax islands. Uses selected Subject/Session/foreground WorkContext and saves exact result:N for show, trace, pin and use. Read nous_help topic=nousql for syntax.",
      inputSchema: z.object({
        query: z.string().min(1).max(65536),
        subject: z.string().optional(),
        session: z.string().optional(),
      }),
    },
    ({ query, subject, session }) =>
      invoke([
        "query",
        query,
        ...(subject ? ["--subject", subject] : []),
        ...(session ? ["--session", session] : []),
      ]),
  );
  return server;
}

/** A process boundary keeps citty's help/error output out of the MCP protocol stream. */
export function cliExecutor(options: {
  entry: string;
  runRoot: string;
  stateRoot: string;
  consumer: string;
}): Execute {
  return async (args) => {
    try {
      const result = await promisify(execFile)(
        process.execPath,
        [
          ...process.execArgv,
          options.entry,
          "--run-root",
          options.runRoot,
          "--instance-root",
          options.stateRoot,
          "--consumer",
          options.consumer,
          ...args,
        ],
        {
          windowsHide: true,
          maxBuffer: 8 * 1024 * 1024,
          timeout: cliCommandTimeoutMs(args),
        },
      );
      return {
        content: [
          { type: "text", text: result.stdout.trim() || result.stderr.trim() },
        ],
      };
    } catch (error) {
      const failure = error as Error & { stdout?: string; stderr?: string };
      return {
        content: [
          {
            type: "text",
            text:
              failure.stderr?.trim() ||
              failure.stdout?.trim() ||
              failure.message,
          },
        ],
        isError: true,
      };
    }
  };
}
