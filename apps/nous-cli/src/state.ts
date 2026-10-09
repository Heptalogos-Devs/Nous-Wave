// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import {
  readFile,
  mkdir,
  writeFile,
  rename,
  readdir,
  unlink,
} from "node:fs/promises";
import { dirname, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { z } from "zod";
import { CliError } from "./agent.js";
const reference = z
  .object({ kind: z.string().max(64), value: z.string().max(2048) })
  .strict();
const selectionSchema = z
  .object({
    schemaVersion: z.literal(1).default(1),
    subjectId: z.string().optional(),
    sessionId: z.string().optional(),
    workContextId: z.string().optional(),
    lastQuery: z
      .object({
        subjectId: z.string(),
        queryId: z.string(),
        results: z.array(reference).max(2048),
      })
      .strict()
      .optional(),
  })
  .strict();
export type Selection = z.infer<typeof selectionSchema>;
const receiptSchema = z
  .object({
    schemaVersion: z.literal(1),
    name: z.string().max(128),
    subjectId: z.string().optional(),
    request: z.unknown(),
    status: z.enum(["pending", "complete", "rejected"]),
    result: z.unknown().optional(),
    rejection: z
      .object({ code: z.union([z.string(), z.number()]), message: z.string() })
      .strict()
      .optional(),
  })
  .strict();
async function atomicJson(path: string, value: unknown) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${crypto.randomUUID()}.tmp`;
  const encoded = JSON.stringify(value, (_, v: unknown) =>
    typeof v === "bigint" ? { $bigint: v.toString() } : v,
  );
  if (Buffer.byteLength(encoded) > 1048576)
    throw new CliError("RESOURCE_EXHAUSTED", "CLI state exceeds 1 MiB");
  await writeFile(temporary, encoded, { mode: 0o600 });
  try {
    for (let attempt = 0; ; attempt++) {
      try {
        await rename(temporary, path);
        break;
      } catch (error) {
        // Windows scanners can briefly hold the destination; retain the old atomic state.
        if (
          process.platform !== "win32" ||
          attempt >= 5 ||
          !["EPERM", "EACCES"].includes(
            (error as NodeJS.ErrnoException).code ?? "",
          )
        )
          throw error;
        await delay(20 * (attempt + 1));
      }
    }
  } finally {
    await unlink(temporary).catch(() => {});
  }
}
async function readJson(path: string): Promise<unknown> {
  const text = await readFile(path, "utf8");
  if (Buffer.byteLength(text) > 1048576)
    throw new CliError("INVALID_ARGUMENT", "CLI state exceeds bounds");
  return JSON.parse(text, (_, v: unknown) =>
    v &&
    typeof v === "object" &&
    "$bigint" in v &&
    typeof v.$bigint === "string" &&
    /^-?\d{1,30}$/.test(v.$bigint)
      ? BigInt(v.$bigint)
      : v,
  );
}
export function cliState(root?: string) {
  const requiredRoot = () => {
    if (!root)
      throw new CliError(
        "INVALID_ARGUMENT",
        "Use the nous launcher or provide --instance-root for durable selection and receipts",
      );
    return root;
  };
  const selectionPath = () => join(requiredRoot(), "consumers/nous-cli.json");
  const receiptPath = (id: string) => {
    if (!/^[a-zA-Z0-9_-]{1,128}$/.test(id))
      throw new CliError("INVALID_ARGUMENT", "Invalid receipt ID");
    return join(requiredRoot(), "consumers/nous-cli/operations", `${id}.json`);
  };
  return {
    async selection(): Promise<Selection> {
      if (!root) return { schemaVersion: 1 };
      try {
        return selectionSchema.parse(await readJson(selectionPath()));
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT")
          return { schemaVersion: 1 };
        throw error;
      }
    },
    async save(this: void, value: Selection) {
      await atomicJson(selectionPath(), selectionSchema.parse(value));
    },
    async receipt(id: string) {
      return receiptSchema.parse(await readJson(receiptPath(id)));
    },
    async writeReceipt(id: string, value: z.infer<typeof receiptSchema>) {
      const path = receiptPath(id);
      await mkdir(dirname(path), { recursive: true });
      const files = (await readdir(dirname(path))).filter((name) =>
        /^[a-zA-Z0-9_-]+\.json$/.test(name),
      );
      if (!files.includes(`${id}.json`) && files.length >= 256) {
        let remaining = files.length;
        for (const file of files) {
          const old = receiptSchema.parse(
            await readJson(join(dirname(path), file)),
          );
          if (old.status !== "pending") {
            await unlink(join(dirname(path), file));
            if (--remaining < 256) break;
          }
        }
        if (remaining >= 256)
          throw new CliError(
            "RESOURCE_EXHAUSTED",
            "256 unresolved operation receipts retained; recover them before another mutation",
          );
      }
      await atomicJson(path, receiptSchema.parse(value));
    },
  };
}
