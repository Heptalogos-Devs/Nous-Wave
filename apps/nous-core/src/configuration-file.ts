import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { stringify } from "smol-toml";
import type { RuntimeLocations } from "./locations.js";

/** Create operator-owned configuration once; schema defaults own runtime policy. */
export async function initializeConfiguration(locations: RuntimeLocations) {
  const path = join(locations.config, "nous.toml");
  await mkdir(locations.config, { recursive: true });
  try {
    await writeFile(
      path,
      stringify({
        deployment: "portable",
        consumers: [{ consumer_id: "default" }],
      }),
      { flag: "wx", mode: 0o600 },
    );
    return { path, created: true };
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
    return { path, created: false };
  }
}
