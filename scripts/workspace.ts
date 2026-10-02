import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { stringify } from "smol-toml";

export const repositoryRoot = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
);
export const workspacePaths = Object.freeze({
  configuration: join(repositoryRoot, "data/config/apps"),
  secrets: join(repositoryRoot, "data/config/secrets"),
  development: join(repositoryRoot, "data/instances/dev"),
  runtime: join(repositoryRoot, "data/runtime"),
  installedRuntime: join(repositoryRoot, "data/runtime/installed"),
  cache: join(repositoryRoot, "data/cache"),
  releases: join(repositoryRoot, "data/releases"),
  tools: join(repositoryRoot, "data/tools"),
  research: join(repositoryRoot, "data/research"),
  temporary: join(repositoryRoot, "data/temp"),
});

async function instanceLocator(
  profile: "dev" | "portable",
  program: string,
  runtime: string,
) {
  const instance = join(repositoryRoot, "data/instances", profile);
  await mkdir(instance, { recursive: true });
  const locator = join(instance, "bootstrap.toml");
  await writeFile(
    locator,
    stringify({
      paths: {
        program,
        runtime,
        config: workspacePaths.configuration,
        secret: workspacePaths.secrets,
        instance: join(instance, "identity"),
        data: join(instance, "authority"),
        blob: join(instance, "objects"),
        cache: join(workspacePaths.cache, "instances", profile),
        log: join(instance, "logs"),
        run: join(instance, "run"),
        temp: join(workspacePaths.temporary, profile),
        backup: join(instance, "backups"),
      },
    }),
  );
  return locator;
}

export function developmentLocator() {
  return instanceLocator(
    "dev",
    repositoryRoot,
    workspacePaths.installedRuntime,
  );
}

export function portableLocator(installation: string) {
  return instanceLocator(
    "portable",
    join(installation, "program"),
    join(installation, "runtime"),
  );
}

export async function workspaceTemp(scope: string, prefix: string) {
  const parent = join(workspacePaths.temporary, scope);
  await mkdir(parent, { recursive: true });
  return mkdtemp(join(parent, prefix));
}
