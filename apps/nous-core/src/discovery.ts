import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, open, readFile, unlink, writeFile } from "node:fs/promises";
import { join } from "node:path";

const execute = promisify(execFile);
function errorCode(error: unknown): string | undefined {
  if (
    error instanceof Error &&
    "code" in error &&
    typeof error.code === "string"
  )
    return error.code;
  return undefined;
}

function parseLock(value: unknown): { pid: number } {
  if (
    value !== null &&
    typeof value === "object" &&
    "pid" in value &&
    typeof value.pid === "number" &&
    Number.isInteger(value.pid) &&
    value.pid > 0
  )
    return { pid: value.pid };
  throw new Error("Invalid instance lock");
}

export async function claimInstance(dataRoot: string) {
  const runtime = join(dataRoot, "runtime");
  await mkdir(runtime, { recursive: true, mode: 0o700 });
  if (process.platform === "win32") {
    const { stdout } = await execute("whoami", [], { windowsHide: true });
    const user = stdout.trim();
    if (!user || /[\r\n]/.test(user))
      throw new Error("Cannot identify local credential owner");
    await execute(
      "icacls",
      [runtime, "/inheritance:r", "/grant:r", `${user}:(OI)(CI)F`],
      { windowsHide: true },
    );
  }
  const lock = join(runtime, "instance.lock");
  try {
    const previous = parseLock(JSON.parse(await readFile(lock, "utf8")));
    let alive = true;
    try {
      process.kill(previous.pid, 0);
    } catch (error) {
      if (errorCode(error) === "ESRCH") alive = false;
      else throw error;
    }
    if (alive) throw new Error("A Core instance already owns this data root");
    await unlink(lock);
  } catch (error) {
    if (errorCode(error) !== "ENOENT") throw error;
  }
  const handle = await open(lock, "wx", 0o600);
  await handle.writeFile(JSON.stringify({ pid: process.pid }));
  await handle.close();
  const path = join(runtime, "core.json");
  return {
    path,
    publish: (endpoint: string, token: string) =>
      writeFile(path, JSON.stringify({ endpoint, token, pid: process.pid }), {
        mode: 0o600,
      }),
    release: async () => {
      await unlink(path).catch((error: unknown) => {
        if (errorCode(error) !== "ENOENT") throw error;
      });
      await unlink(lock);
    },
  };
}
