// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { execFile } from "node:child_process";
import { promisify } from "node:util";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { rm, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  ConfigExposure,
  ConfigurationView,
  configurationValue,
} from "@nous-wave/client";
import { boot, prepare, stop, type Boot } from "./support.js";
import { workspaceTemp } from "../workspace.js";
import { resolveLocations } from "../../apps/nous-core/src/locations.js";

const root = await workspaceTemp("smoke", "configuration-");
let current: Boot | undefined;
try {
  const locator = await prepare(root);
  const locations = await resolveLocations({
    locator,
    installationHome: process.cwd(),
  });
  const configPath = join(locations.config, "nous.toml");
  await writeFile(
    configPath,
    (await readFile(configPath, "utf8")) + "\n[maintenance]\nenabled = false\n",
  );
  current = await boot(locator);
  let client = current.client;
  const cli = async (...args: string[]) => {
    const result = await promisify(execFile)(
      process.execPath,
      [
        join(locations.program, "node_modules/tsx/dist/cli.mjs"),
        join(locations.program, "apps/nous-core/src/launcher.ts"),
        "--development",
        "--locator",
        locator,
        "config",
        ...args,
      ],
      { maxBuffer: 2 * 1024 * 1024, windowsHide: true },
    );
    const parsed = JSON.parse(result.stdout) as {
      schemaVersion?: string;
      data?: unknown;
    };
    if (args[0] !== "check") assert.equal(parsed.schemaVersion, "nous.cli.v1");
    return (args[0] === "check" ? parsed : parsed.data) as {
      valid?: boolean;
      descriptors?: { path: string; exposure: number }[];
    };
  };
  assert.equal((await cli("check")).valid, true);
  const standardCli = (await cli("list", "--json")).descriptors!;
  assert(standardCli.length > 0);
  assert(standardCli.every((d) => d.exposure === ConfigExposure.STANDARD));
  const advancedCli = (await cli("list", "--advanced", "--json")).descriptors!;
  assert(advancedCli.some((d) => d.exposure === ConfigExposure.ADVANCED));
  assert(advancedCli.every((d) => d.exposure !== ConfigExposure.DEVELOPER));
  const developerCli = (await cli("list", "--developer", "--json"))
    .descriptors!;
  assert(developerCli.some((d) => d.exposure === ConfigExposure.DEVELOPER));
  const standard = await client.configuration.list({});
  assert(standard.descriptors.some((d) => d.path === "models"));
  assert(standard.descriptors.some((d) => d.path === "maintenance.enabled"));
  assert(
    standard.descriptors.every((d) => d.exposure === ConfigExposure.STANDARD),
  );
  const schema = (await client.configuration.describe("runtime.resident_limit"))
    .jsonSchema;
  assert(schema && typeof schema === "object" && !Array.isArray(schema));
  assert.equal(schema.$schema, "https://json-schema.org/draft/2020-12/schema");
  const subject = await client.subjects.create({
    operationId: randomUUID(),
    cognitiveSeed: {
      text: "schema_version = 1",
      format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
    },
  });
  const entry = async (path: string, subjectId?: string, desired = false) => {
    const result = await client.configuration.get({
      paths: [path],
      subjectId,
      view: desired ? ConfigurationView.DESIRED : ConfigurationView.ACTIVE,
    });
    const item = result.entries[0];
    assert(item);
    return item;
  };
  const defaults = (await entry("video")).value;
  const modelGraph = {
    gateway_profiles: {
      local: {
        base_url: "http://127.0.0.1:1/v1",
        credential_env: "NOUS_CONFIGURATION_SMOKE",
      },
    },
    model_profiles: {
      chat: {
        gateway: "local",
        protocol: "openai-chat",
        model: "declared",
        capabilities: ["text", "structured_output"],
      },
    },
    execution_profiles: { primary: { model: "chat" } },
    roles: { memory_formation: { routes: ["primary"] } },
  };
  const graphOperation = randomUUID();
  const sparseGraph = await client.configuration.setSystem({
    operationId: graphOperation,
    path: "models",
    value: modelGraph,
  });
  const frozenGraph = (await entry("models", undefined, true)).value;
  assert(
    frozenGraph &&
      typeof frozenGraph === "object" &&
      !Array.isArray(frozenGraph),
  );
  assert.deepEqual(
    (frozenGraph.execution_profiles as Record<string, unknown>).primary,
    {
      model: "chat",
      reasoning: "provider-default",
      provider_options: {},
      max_output_tokens: 4096,
      timeout_ms: 30000,
    },
  );
  const explicitGraph = await client.configuration.setSystem({
    operationId: graphOperation,
    path: "models",
    value: frozenGraph,
  });
  assert.equal(explicitGraph.revision, sparseGraph.revision);
  assert.equal(explicitGraph.desiredDigest, sparseGraph.desiredDigest);
  await assert.rejects(
    client.configuration.setSystem({
      operationId: randomUUID(),
      path: "models",
      value: {
        ...modelGraph,
        roles: { memory_formation: { routes: ["absent"] } },
      },
    }),
    /models.roles.memory_formation.routes.0/,
  );
  assert(defaults && typeof defaults === "object" && !Array.isArray(defaults));
  const normalizedOperation = randomUUID();
  const omitted = await client.configuration.setSystem({
    operationId: normalizedOperation,
    path: "video",
    value: { max_frames: 4 },
  });
  const explicit = await client.configuration.setSystem({
    operationId: normalizedOperation,
    path: "video",
    value: { ...defaults, max_frames: 4 },
  });
  assert.equal(omitted.desiredDigest, explicit.desiredDigest);
  assert.equal(omitted.revision, explicit.revision);
  assert.deepEqual((await entry("video", undefined, true)).value, {
    ...defaults,
    max_frames: 4,
  });
  assert.deepEqual((await entry("video")).value, defaults);
  assert.equal((await entry("maintenance.enabled")).source, "deployment_file");
  const operationId = randomUUID();
  const change = await client.configuration.setSystem({
    operationId,
    path: "maintenance.enabled",
    value: configurationValue("true"),
  });
  const replay = await client.configuration.setSystem({
    operationId,
    path: "maintenance.enabled",
    value: configurationValue("true"),
  });
  assert.equal(replay.revision, change.revision);
  assert.equal((await entry("maintenance.enabled")).value, true);
  await client.configuration.setSubject({
    operationId: randomUUID(),
    subjectId: subject.subjectId,
    path: "maintenance.enabled",
    value: configurationValue("false"),
  });
  assert.equal(
    (await entry("maintenance.enabled", subject.subjectId)).source,
    "persisted_subject",
  );
  assert.equal(
    (await entry("maintenance.enabled", subject.subjectId)).value,
    false,
  );
  const initial = (await entry("runtime.resident_limit")).value;
  const restart = await client.configuration.setSystem({
    operationId: randomUUID(),
    path: "runtime.resident_limit",
    value: configurationValue("512"),
  });
  assert.deepEqual(restart.effects, ["restart_required"]);
  assert.equal((await entry("runtime.resident_limit")).value, initial);
  assert.equal(
    (await entry("runtime.resident_limit", undefined, true)).value,
    512,
  );
  assert.equal(
    (await entry("runtime.resident_limit")).pendingEffect,
    "restart_required",
  );
  await assert.rejects(
    client.configuration.setSystem({
      operationId: randomUUID(),
      path: "host.port",
      value: configurationValue("0"),
    }),
  );
  await stop(current);
  current = await boot(locator);
  client = current.client;
  assert.equal((await entry("runtime.resident_limit")).value, 512);
  assert.deepEqual((await entry("video")).value, {
    ...defaults,
    max_frames: 4,
  });
  assert.equal(
    (await entry("runtime.resident_limit")).pendingEffect,
    undefined,
  );
  assert.equal(
    (await entry("maintenance.enabled", subject.subjectId)).value,
    false,
  );
  console.log(
    "CONFIGURATION_SMOKE catalog=true cli=true precedence=true replay=true restart=true normalized_identity=true",
  );
} finally {
  await stop(current);
  await rm(root, { recursive: true, force: true });
}
