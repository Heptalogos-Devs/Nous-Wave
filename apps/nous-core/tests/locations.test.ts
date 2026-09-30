import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { expect, it } from "vitest";
import { resolveLocations } from "../src/locations.js";

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
