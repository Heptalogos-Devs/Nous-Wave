import { readFile, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { parse } from "smol-toml";
import { z } from "zod";
import { parseEnv } from "node:util";
import {
  modelConfigurationShape,
  modelConfigurationSchema,
} from "./model/configuration.js";
import type { RuntimeLocations } from "./locations.js";
import { resolvedEmbedding } from "./model/embedding-profile.js";

const requirement = z.enum(["REQUIRED", "PREFERRED", "OPTIONAL", "FORBIDDEN"]);
const schema = z.strictObject({
  deployment: z.enum(["portable", "development"]).default("portable"),
  port: z.number().int().min(0).max(65535).default(9470),
  dotenv_file: z.string().min(1).default("gateway.env"),
  kernel_executable: z.string().min(1).optional(),
  model_budget: z
    .strictObject({
      max_calls: z.number().int().min(1).max(10000).default(10000),
    })
    .prefault({}),
  database: z
    .strictObject({
      mode: z.enum(["managed_private", "external"]).default("managed_private"),
      url_env: z
        .string()
        .regex(/^[A-Za-z_][A-Za-z0-9_]*$/)
        .optional(),
      max_connections: z.number().int().min(1).max(256).default(8),
      name: z
        .string()
        .regex(/^[a-z][a-z0-9_]{0,62}$/)
        .default("nous_wave"),
    })
    .prefault({}),
  object_store: z
    .strictObject({
      max_upload_bytes: z
        .number()
        .int()
        .positive()
        .max(Number.MAX_SAFE_INTEGER)
        .default(8589934592),
    })
    .prefault({}),
  settings: z.record(z.string(), z.unknown()).default({}),
  consumers: z
    .array(
      z.strictObject({
        consumer_id: z.string().min(1).max(128),
        revision: z.string().min(1).default("1"),
        memory: requirement.default("PREFERRED"),
        runtime: requirement.default("OPTIONAL"),
        resource: requirement.default("OPTIONAL"),
        max_items: z.number().int().min(1).max(2048).default(32),
        max_text_bytes: z.number().int().min(1).max(1048576).default(32768),
        materialize: z.boolean().default(true),
      }),
    )
    .min(1)
    .max(64),
  ...modelConfigurationShape,
});

export async function loadConfig(locations: RuntimeLocations) {
  const config = schema.parse(
    parse(await readFile(join(locations.config, "nous.toml"), "utf8")),
  );
  const credentialNames = [
    ...Object.values(config.gateway_profiles).map((g) => g.credential_env),
    ...(config.database.url_env ? [config.database.url_env] : []),
  ];
  try {
    const credentials = parseEnv(
      await readFile(resolve(locations.secret, config.dotenv_file), "utf8"),
    );
    for (const name of credentialNames)
      if (process.env[name] === undefined && credentials[name] !== undefined)
        process.env[name] = credentials[name];
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT")
      // oxlint-disable-next-line preserve-caught-error -- Raw dotenv errors can include credential content.
      throw new Error("Credential dotenv file could not be loaded");
  }
  if (
    new Set(config.consumers.map((c) => c.consumer_id)).size !==
    config.consumers.length
  )
    throw new Error("Duplicate consumer policy ID");
  if (config.kernel_executable && config.deployment !== "development")
    throw new Error("Kernel executable override is development-only");
  const models = modelConfigurationSchema.parse({
    material_strategy: config.material_strategy,
    video: config.video,
    audio: config.audio,
    gateway_profiles: config.gateway_profiles,
    model_profiles: config.model_profiles,
    roles: config.roles,
  });
  if (models.video.ffmpeg_executable)
    models.video.ffmpeg_executable = resolve(
      locations.config,
      models.video.ffmpeg_executable,
    );
  else {
    const ffmpeg = join(
      locations.runtime,
      "ffmpeg",
      "bin",
      process.platform === "win32" ? "ffmpeg.exe" : "ffmpeg",
    );
    if ((await stat(ffmpeg).catch(() => undefined))?.isFile())
      models.video.ffmpeg_executable = ffmpeg;
  }
  const kernelExecutable = config.kernel_executable
    ? resolve(locations.program, config.kernel_executable)
    : join(
        locations.program,
        "kernel",
        process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
      );
  const databaseUrl = config.database.url_env
    ? process.env[config.database.url_env]
    : undefined;
  if (config.database.mode === "external" && !databaseUrl)
    throw new Error("External database credential environment is unavailable");
  return {
    deployment: config.deployment,
    modelBudget: config.model_budget.max_calls,
    externalFfmpeg: Boolean(config.video.ffmpeg_executable),
    kernelExecutable,
    locations,
    port: config.port,
    models,
    kernelBootstrap: {
      bootstrap: {
        database: {
          mode: config.database.mode,
          url: databaseUrl ?? "",
          max_connections: config.database.max_connections,
          name: config.database.name,
          install_dir: join(locations.runtime, "postgresql"),
          data_dir: join(locations.data, "postgres"),
          instance_dir: locations.instance,
          secret_dir: locations.secret,
        },
        object_store: {
          backend: "fs",
          root: locations.blob,
          max_upload_bytes: config.object_store.max_upload_bytes,
        },
        serving: {
          root: join(locations.cache, "serving"),
          resolved_embedding: resolvedEmbedding(models),
        },
      },
      settings: config.settings,
    },
    consumers: config.consumers.map((c) => ({
      consumerId: c.consumer_id,
      revision: c.revision,
      memory: c.memory,
      runtime: c.runtime,
      resource: c.resource,
      maxItems: c.max_items,
      maxTextBytes: c.max_text_bytes,
      materialize: c.materialize,
    })),
  };
}
