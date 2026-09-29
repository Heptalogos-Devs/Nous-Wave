import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const binary = join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "nous-kernel.exe" : "nous-kernel",
);

if (!existsSync(binary)) {
  console.error(`Kernel binary not found: ${binary}`);
  console.error("Build it first with: cargo build -p nous-kernel");
  process.exit(1);
}

const dataRoot = resolve(root, "data", "dev");
await mkdir(dataRoot, { recursive: true });
const kernelConfig = join(dataRoot, "kernel.toml");
const coreConfig = join(dataRoot, "core.toml");

await writeFile(
  kernelConfig,
  `[bootstrap.database]\nmode = "managed"\nurl = ""\nmax_connections = 8\nname = "nous_wave_dev"\ninstall_dir = ${JSON.stringify(join(dataRoot, "postgres-install"))}\ndata_dir = ${JSON.stringify(join(dataRoot, "postgres"))}\n\n[bootstrap.object_store]\nbackend = "fs"\nroot = ${JSON.stringify(join(dataRoot, "objects"))}\nmax_upload_bytes = 8589934592\n\n[bootstrap.serving]\nroot = ${JSON.stringify(join(dataRoot, "serving"))}\n\n[settings.capabilities.process]\nmemory = true\n\n[settings.capabilities.subject_defaults]\nmemory = true\n`,
);
await writeFile(
  coreConfig,
  `kernel_executable = ${JSON.stringify(binary)}\nkernel_config = ${JSON.stringify(kernelConfig)}\ndata_root = ${JSON.stringify(dataRoot)}\nport = 9470\nmax_upload_bytes = 8589934592\n\n[models]\n\n[[consumers]]\nconsumer_id = "default"\nrevision = "1"\nmemory = "PREFERRED"\nruntime = "OPTIONAL"\nresource = "OPTIONAL"\nmax_items = 32\nmax_text_bytes = 32768\nmaterialize = true\n`,
);

const child = spawn(
  process.execPath,
  [
    join(root, "node_modules", "tsx", "dist", "cli.mjs"),
    "apps/nous-core/src/main.ts",
    "--config",
    coreConfig,
  ],
  {
    cwd: root,
    stdio: "inherit",
    windowsHide: true,
  },
);
child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exitCode = code ?? 1;
});
