import { stringify } from "smol-toml";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function qualificationConfig(root: string, name: string) {
  const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  return stringify({
    bootstrap: {
      database: {
        mode: "managed_private",
        url: "",
        max_connections: 4,
        name,
        install_dir:
          process.env.NOUS_WAVE_POSTGRES_RUNTIME ??
          join(repo, "data/dev/runtime/postgresql"),
        data_dir: join(root, "data/postgres"),
        instance_dir: join(root, "instance"),
        secret_dir: join(root, "secrets"),
      },
      object_store: {
        backend: "fs",
        root: join(root, "blobs"),
        max_upload_bytes: 1048576,
      },
      serving: { root: join(root, "cache/serving") },
    },
    settings: {
      capabilities: {
        process: { memory: true },
        subject_defaults: { memory: true },
      },
    },
  });
}
