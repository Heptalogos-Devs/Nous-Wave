import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { expect, it } from "vitest";
import { resolveLocations } from "../src/locations.js";
import { initializeConfiguration } from "../src/configuration-file.js";
import {
  CONFIG_REVISION,
  parseConfiguration,
  loadConfig,
} from "../src/config.js";
import { checkConfiguration } from "../src/configuration-check.js";

it("resolves split roots relative to the locator while installation and instance remain independent", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-locations-"));
  try {
    const locator = join(root, "bootstrap.toml");
    await writeFile(
      locator,
      '[paths]\ndata="authority"\nrun="process"\nconfig="settings"\nblob="media"\n',
    );
    const locations = await resolveLocations({
      locator,
      installationHome: join(root, "install"),
    });
    expect(locations.program).toBe(join(root, "install", "program"));
    expect(locations.data).toBe(join(root, "authority"));
    expect(locations.run).toBe(join(root, "process"));
    expect(locations.config).toBe(join(root, "settings"));
    expect(locations.blob).toBe(join(root, "media"));
    expect(Object.isFrozen(locations)).toBe(true);
    await expect(
      resolveLocations({ home: root, locator, installationHome: root }),
    ).rejects.toThrow("Choose");
    await writeFile(locator, '[paths]\nunknown="escape"\n');
    await expect(
      resolveLocations({ locator, installationHome: root }),
    ).rejects.toThrow();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

it("initializes a portable instance once and preserves operator edits byte for byte", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-initial-config-"));
  try {
    const locations = await resolveLocations({
      home: root,
      installationHome: join(root, "install"),
    });
    const results = await Promise.all([
      initializeConfiguration(locations),
      initializeConfiguration(locations),
    ]);
    expect(results.filter((result) => result.created)).toHaveLength(1);
    const config = await loadConfig(locations);
    expect(config.deployment).toBe("portable");
    expect(config.models.roles).toEqual({});
    expect((await checkConfiguration(locations)).valid).toBe(true);
    const development = await loadConfig(locations, true);
    expect(development.deployment).toBe("development");
    expect(development.kernelExecutable).toBe(
      join(
        locations.program,
        "target/debug",
        process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
      ),
    );
    const edited = `# operator configuration\r\nconfig_revision = ${CONFIG_REVISION}\r\nport = 12345\r\n[[consumers]]\r\nconsumer_id = "custom"\r\n`;
    await writeFile(results[0]!.path, edited);
    expect((await initializeConfiguration(locations)).created).toBe(false);
    expect(await readFile(results[0]!.path, "utf8")).toBe(edited);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

it("rejects stale configuration semantics and checks examples and explicit references offline", async () => {
  const example = await readFile(
    join(process.cwd(), "docs/reference/examples/nous.toml"),
    "utf8",
  );
  expect(parseConfiguration(example).config.config_revision).toBe(
    CONFIG_REVISION,
  );
  expect(() =>
    parseConfiguration(
      example.replace("config_revision = 1", "config_revision = 0"),
    ),
  ).toThrow("Expected 1");
  expect(() =>
    parseConfiguration(example.replace("config_revision = 1", "")),
  ).toThrow("missing");
  expect(() =>
    parseConfiguration(example + "\nobsolete_field = true\n"),
  ).toThrow();
  const root = await mkdtemp(join(tmpdir(), "nous-config-check-"));
  try {
    const locations = await resolveLocations({
      home: root,
      installationHome: process.cwd(),
      programRoot: process.cwd(),
    });
    const initialized = await initializeConfiguration(locations);
    const text = `config_revision = ${CONFIG_REVISION}\n[[consumers]]\nconsumer_id = "default"\n[gateway_profiles.local]\nbase_url = "http://127.0.0.1:3000/v1"\ncredential_env = "NOUS_OFFLINE_CHECK_TOKEN"\n[model_profiles.local]\ngateway = "local"\nprotocol = "openai-chat"\nmodel = "local"\ncapabilities = ["text"]\n[roles.memory_formation]\nmodel = "local"\nprompt = "config-prompts/missing.md"\n`;
    await writeFile(initialized.path, text);
    const result = await checkConfiguration(locations);
    expect(result.issues).toContainEqual(
      expect.objectContaining({
        path: "roles.memory_formation.prompt",
        code: "invalid_reference",
      }),
    );
    expect(await readFile(initialized.path, "utf8")).toBe(text);
    expect(process.env.NOUS_OFFLINE_CHECK_TOKEN).toBeUndefined();
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
