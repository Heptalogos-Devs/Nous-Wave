import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { expect, it } from "vitest";
import { resolveLocations } from "../src/locations.js";
import { initializeConfiguration } from "../src/configuration-file.js";
import { loadConfig } from "../src/config.js";

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
    const development = await loadConfig(locations, true);
    expect(development.deployment).toBe("development");
    expect(development.kernelExecutable).toBe(
      join(
        locations.program,
        "target/debug",
        process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
      ),
    );
    const edited =
      '# operator configuration\r\nport = 12345\r\n[[consumers]]\r\nconsumer_id = "custom"\r\n';
    await writeFile(results[0]!.path, edited);
    expect((await initializeConfiguration(locations)).created).toBe(false);
    expect(await readFile(results[0]!.path, "utf8")).toBe(edited);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
