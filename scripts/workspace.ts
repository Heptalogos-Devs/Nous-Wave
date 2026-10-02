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

export async function developmentLocator() {
  await mkdir(workspacePaths.development, { recursive: true });
  const locator = join(workspacePaths.development, "bootstrap.toml");
  await writeFile(
    locator,
    stringify({
      paths: {
        program: repositoryRoot,
        runtime: workspacePaths.installedRuntime,
        config: workspacePaths.configuration,
        secret: workspacePaths.secrets,
        instance: join(workspacePaths.development, "identity"),
        data: join(workspacePaths.development, "authority"),
        blob: join(workspacePaths.development, "objects"),
        cache: join(workspacePaths.cache, "instances/dev"),
        log: join(workspacePaths.development, "logs"),
        run: join(workspacePaths.development, "run"),
        temp: join(workspacePaths.temporary, "dev"),
        backup: join(workspacePaths.development, "backups"),
      },
    }),
  );
  return locator;
}

export async function workspaceTemp(scope: string, prefix: string) {
  const parent = join(workspacePaths.temporary, scope);
  await mkdir(parent, { recursive: true });
  return mkdtemp(join(parent, prefix));
}
