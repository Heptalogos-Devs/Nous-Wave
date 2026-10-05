import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { once } from "node:events";
import { resolve } from "node:path";
export interface RuntimeMetrics {
  runBytes: number;
  servingBytes: number;
  databaseBytes: number;
  generations: number;
  retired: number;
}
export interface RuntimeInspection {
  memories: unknown[];
  tags: unknown[];
  associations: unknown[];
  episodes: unknown[];
  journals: unknown[];
  needs: unknown[];
  metrics: RuntimeMetrics;
}
export async function launchFunctionalRuntime(
  root: string,
  token: string,
  signal: AbortSignal,
  queryMode = false,
  embeddingCache?: string,
) {
  const child = spawn(
    resolve("target/debug/examples/functional-runtime"),
    embeddingCache
      ? [root, queryMode ? "query" : "formation", resolve(embeddingCache)]
      : queryMode
        ? [root, "query"]
        : [root],
    {
      stdio: ["pipe", "pipe", "inherit"],
      env: { ...process.env, NOUS_RESEARCH_TOKEN: token },
    },
  );
  const exit = once(child, "exit");
  const lines = createInterface({ input: child.stdout });
  const replies = lines[Symbol.asyncIterator]();
  const read = async () => {
    const result = await replies.next();
    if (result.done) throw new Error("Functional runtime closed");
    const value = JSON.parse(result.value) as Record<string, unknown>;
    if (value.error)
      throw new Error(
        typeof value.error === "string"
          ? value.error
          : JSON.stringify(value.error),
      );
    return value;
  };
  const abort = () => child.kill("SIGINT");
  signal.addEventListener("abort", abort, { once: true });
  let queue: Promise<unknown> = Promise.resolve();
  const control = (command: Record<string, unknown>) => {
    const result = queue.then(async () => {
      child.stdin.write(JSON.stringify(command) + "\n");
      return read();
    });
    queue = result.catch(() => {});
    return result;
  };
  try {
    const ready = await read();
    if (!ready.ready || typeof ready.endpoint !== "string")
      throw new Error("Functional runtime did not become ready");
    signal.removeEventListener("abort", abort);
    return {
      endpoint: ready.endpoint,
      metrics: ready.metrics as unknown as RuntimeMetrics,
      baselineBytes: Number(ready.baselineBytes),
      baselineGenerations: Number(ready.baselineGenerations),
      control,
      close: async () => {
        signal.removeEventListener("abort", abort);
        if (child.exitCode === null && !child.killed) {
          try {
            await control({ command: "done" });
          } catch {
            child.kill("SIGINT");
          }
        }
        await exit;
        lines.close();
      },
    };
  } catch (error) {
    signal.removeEventListener("abort", abort);
    child.kill("SIGINT");
    await exit;
    lines.close();
    throw error;
  }
}
