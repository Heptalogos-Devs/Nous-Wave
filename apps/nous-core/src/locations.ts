import { readFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { parse } from "smol-toml";
import { z } from "zod";

const rootNames = [
  "program",
  "runtime",
  "instance",
  "config",
  "data",
  "blob",
  "cache",
  "secret",
  "log",
  "run",
  "temp",
  "backup",
] as const;
export type RuntimeLocations = Readonly<
  Record<(typeof rootNames)[number], string>
>;
const path = z.string().min(1);
const locatorSchema = z.strictObject({
  paths: z.strictObject({
    program: path.optional(),
    runtime: path.optional(),
    instance: path.optional(),
    config: path.optional(),
    data: path.optional(),
    blob: path.optional(),
    cache: path.optional(),
    secret: path.optional(),
    log: path.optional(),
    run: path.optional(),
    temp: path.optional(),
    backup: path.optional(),
  }),
});
const folders: Record<(typeof rootNames)[number], string> = {
  program: "program",
  runtime: "runtime",
  instance: "instance",
  config: "config",
  data: "data",
  blob: "blobs",
  cache: "cache",
  secret: "secrets",
  log: "logs",
  run: "run",
  temp: "temp",
  backup: "backups",
};

/** Only bootstrap maps a profile; consumers receive independent absolute roots. */
export async function resolveLocations(input: {
  home?: string;
  locator?: string;
  installationHome: string;
  programRoot?: string;
  runtimeRoot?: string;
}): Promise<RuntimeLocations> {
  if (input.home && input.locator)
    throw new Error("Choose --home or --locator");
  if (!isAbsolute(input.installationHome))
    throw new Error("Installation home must be absolute");
  const base = input.locator
    ? dirname(resolve(input.locator))
    : resolve(input.home ?? input.installationHome);
  const overrides = input.locator
    ? locatorSchema.parse(parse(await readFile(resolve(input.locator), "utf8")))
        .paths
    : {};
  const result = {} as Record<(typeof rootNames)[number], string>;
  for (const name of rootNames) {
    const override = overrides[name];
    const installationDefault =
      name === "program"
        ? (input.programRoot ?? join(input.installationHome, "program"))
        : name === "runtime"
          ? (input.runtimeRoot ?? join(input.installationHome, "runtime"))
          : undefined;
    result[name] =
      override !== undefined
        ? resolve(base, override)
        : installationDefault
          ? resolve(installationDefault)
          : join(base, folders[name]);
  }
  return Object.freeze(result);
}
