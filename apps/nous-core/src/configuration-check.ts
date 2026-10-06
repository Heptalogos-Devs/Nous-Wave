// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { configurationBundle } from "./configuration-catalog.js";
import { join, resolve } from "node:path";
import { stat, mkdir, writeFile, rm } from "node:fs/promises";
import {
  CONFIG_REVISION,
  ConfigurationError,
  readConfiguration,
  parseEffectiveConfiguration,
  kernelExecutable,
} from "./config.js";
import type { RuntimeLocations } from "./locations.js";
import { PromptRegistry } from "./model/prompts.js";
import { roleNames, modelRoleProblem } from "./model/configuration.js";

/** Offline inspection: no environment mutation, runtime startup or provider calls. */
export async function checkConfiguration(
  locations: RuntimeLocations,
  development = false,
) {
  const configuration = join(locations.config, "nous.toml");
  const issues: { path: string; code: string; message: string }[] = [];
  const diagnostics: { role: string; state: string; detail: string }[] = [];
  try {
    const { config, document } = await readConfiguration(
      locations,
      development,
    );
    const { models } = parseEffectiveConfiguration({
      ...document,
      "material.inputs": (
        document.material as Record<string, unknown> | undefined
      )?.inputs,
      "material.strategy": (
        document.material as Record<string, unknown> | undefined
      )?.strategy,
    });
    const prompts = new PromptRegistry(
      join(locations.program, "prompts"),
      join(locations.config, "prompts"),
    );
    for (const role of roleNames) {
      const binding = models.roles[role];
      if (!binding) continue;
      const profile = models.model_profiles[binding.model];
      const problem = profile
        ? modelRoleProblem(role, binding, profile)
        : "Model profile is absent";
      const gateway = profile
        ? models.gateway_profiles[profile.gateway]
        : undefined;
      if (problem || !gateway || !profile?.model.trim())
        diagnostics.push({
          role,
          state: problem && profile ? "UNAVAILABLE" : "NOT_CONFIGURED",
          detail:
            problem ??
            (!gateway
              ? "Gateway profile is absent"
              : "Model identifier is unset"),
        });
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
        models.video.ffmpeg_executable
          ? resolve(locations.config, models.video.ffmpeg_executable)
          : undefined,
      ],
      [
        "kernel_executable",
        config.host.kernel_executable
          ? resolve(locations.program, config.host.kernel_executable)
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
    const bundle = configurationBundle(document);
    await mkdir(locations.temp, { recursive: true });
    const bundlePath = join(
      locations.temp,
      `config-check-${randomUUID()}.json`,
    );
    try {
      await writeFile(bundlePath, JSON.stringify(bundle), { mode: 0o600 });
      await promisify(execFile)(
        kernelExecutable(locations, development, config.host.kernel_executable),
        ["--check-configuration", bundlePath],
      );
    } catch (error) {
      const unavailable =
        error instanceof Error && "code" in error && error.code === "ENOENT";
      issues.push({
        path: unavailable ? "host.kernel_executable" : "catalog",
        code: unavailable ? "validator_unavailable" : "invalid_configuration",
        message: unavailable
          ? "Kernel Catalog validator executable is unavailable"
          : "Configuration document failed Kernel Catalog validation",
      });
    } finally {
      await rm(bundlePath, { force: true });
    }
  } catch (error) {
    if (error instanceof ConfigurationError) issues.push(...error.issues);
    else
      issues.push({
        path: "nous.toml",
        code: "invalid_configuration",
        message: "Configuration file or Catalog validation failed",
      });
  }
  return {
    valid: issues.length === 0,
    config_revision: CONFIG_REVISION,
    configuration,
    issues,
    diagnostics,
  };
}
