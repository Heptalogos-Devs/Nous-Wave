import { join, resolve } from "node:path";
import { stat } from "node:fs/promises";
import {
  CONFIG_REVISION,
  ConfigurationError,
  readConfiguration,
} from "./config.js";
import type { RuntimeLocations } from "./locations.js";
import { PromptRegistry } from "./model/prompts.js";
import { roleNames } from "./model/configuration.js";

/** Offline inspection: no environment mutation, runtime startup or provider calls. */
export async function checkConfiguration(
  locations: RuntimeLocations,
  development = false,
) {
  const configuration = join(locations.config, "nous.toml");
  const issues: { path: string; code: string; message: string }[] = [];
  try {
    const { config, models } = await readConfiguration(locations, development);
    const prompts = new PromptRegistry(
      join(locations.program, "prompts"),
      join(locations.config, "prompts"),
    );
    for (const role of roleNames) {
      const binding = models.roles[role];
      if (!binding) continue;
      try {
        await prompts.load(role, binding.prompt);
      } catch {
        issues.push({
          path: `roles.${role}.prompt`,
          code: "invalid_reference",
          message:
            "Prompt must exist inside its declared root, contain UTF-8 text and fit within 128 KiB",
        });
      }
    }
    if (models.roles.material_description) {
      try {
        await prompts.load("material_description", models.video.prompt);
      } catch {
        issues.push({
          path: "video.prompt",
          code: "invalid_reference",
          message:
            "Video prompt must be a valid UTF-8 asset inside its declared root",
        });
      }
    }
    for (const [key, path] of [
      [
        "video.ffmpeg_executable",
        config.video.ffmpeg_executable
          ? resolve(locations.config, config.video.ffmpeg_executable)
          : undefined,
      ],
      [
        "kernel_executable",
        config.kernel_executable
          ? resolve(locations.program, config.kernel_executable)
          : undefined,
      ],
    ]) {
      if (path && !(await stat(path).catch(() => undefined))?.isFile())
        issues.push({
          path: key!,
          code: "invalid_reference",
          message: "Configured executable must refer to an existing file",
        });
    }
  } catch (error) {
    if (error instanceof ConfigurationError) issues.push(...error.issues);
    else
      issues.push({
        path: "nous.toml",
        code: "unreadable_file",
        message: "Configuration file could not be read",
      });
  }
  return {
    valid: issues.length === 0,
    config_revision: CONFIG_REVISION,
    configuration,
    issues,
  };
}
