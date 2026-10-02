import { createServer, request as httpRequest } from "node:http";
import { request as httpsRequest } from "node:https";
import { once } from "node:events";
import { appendFile } from "node:fs/promises";
import { remoteEndpointSchema } from "../../apps/nous-core/src/remote-endpoint.js";
import { ResearchModelCallGuard } from "./model-call-guard.js";

/** Run-owned accounting at the wire boundary, including failed/retried requests. */
export async function startResearchGateway(options: {
  upstream: string;
  ledger: string;
  maxCalls: number;
  port: number;
}) {
  const upstream = new URL(remoteEndpointSchema.parse(options.upstream));
  const guard = new ResearchModelCallGuard(options.ledger, options.maxCalls);
  let telemetry: Promise<void> = Promise.resolve();
  const basePath = upstream.pathname.replace(/\/$/, "");
  const paths = new Set(
    [
      "/chat/completions",
      "/responses",
      "/embeddings",
      "/rerank",
      "/audio/transcriptions",
    ].map((path) => basePath + path),
  );
  const server = createServer((incoming, outgoing) => {
    void (async () => {
      const target = new URL(incoming.url ?? "/", upstream.origin);
      if (
        incoming.method !== "POST" ||
        target.origin !== upstream.origin ||
        !paths.has(target.pathname)
      ) {
        outgoing.writeHead(404).end();
        return;
      }
      let attempt: number;
      try {
        attempt = await guard.reserve();
      } catch {
        outgoing.writeHead(429, { "Content-Type": "application/json" });
        outgoing.end(
          '{"error":{"message":"Research call guard exhausted or unavailable"}}',
        );
        return;
      }
      const transport =
        upstream.protocol === "https:" ? httpsRequest : httpRequest;
      const started = performance.now();
      let recorded = false;
      const record = (
        status: number | null,
        usage: Record<string, number> = {},
      ) => {
        if (recorded) return;
        recorded = true;
        const latencyMs = performance.now() - started;
        telemetry = telemetry.then(() =>
          appendFile(
            options.ledger + ".telemetry.jsonl",
            JSON.stringify({
              attempt,
              endpoint: target.pathname.slice(basePath.length),
              status,
              latencyMs,
              usage,
              cost: "unknown",
            }) + "\n",
            { mode: 0o600 },
          ),
        );
        // close() propagates an unavailable research telemetry sink to the runner.
        void telemetry.catch(() => {});
      };
      const forwarded = transport(
        target,
        {
          method: incoming.method,
          headers: { ...incoming.headers, host: upstream.host },
          timeout: 300000,
        },
        (response) => {
          const chunks: Buffer[] = [];
          let size = 0;
          response.on("data", (chunk: Buffer) => {
            size += chunk.length;
            if (size <= 1048576) chunks.push(chunk);
            else chunks.length = 0;
          });
          response.on("end", () => {
            const usage: Record<string, number> = {};
            if (size <= 1048576) {
              try {
                const result: unknown = JSON.parse(
                  Buffer.concat(chunks).toString("utf8"),
                );
                if (
                  result &&
                  typeof result === "object" &&
                  "usage" in result &&
                  result.usage &&
                  typeof result.usage === "object"
                ) {
                  const raw = result.usage as Record<string, unknown>;
                  for (const key of [
                    "prompt_tokens",
                    "completion_tokens",
                    "input_tokens",
                    "output_tokens",
                    "total_tokens",
                  ]) {
                    const value = raw[key];
                    if (
                      typeof value === "number" &&
                      Number.isSafeInteger(value) &&
                      value >= 0
                    )
                      usage[key] = value;
                  }
                }
              } catch {
                /* Usage is unknown for non-JSON or truncated responses. */
              }
            }
            record(response.statusCode ?? null, usage);
          });
          response.on("aborted", () => record(response.statusCode ?? null));
          outgoing.writeHead(response.statusCode ?? 502, response.headers);
          response.pipe(outgoing);
          response.on("error", () => {
            record(response.statusCode ?? null);
            outgoing.destroy();
          });
        },
      );
      forwarded.on("timeout", () => forwarded.destroy());
      forwarded.on("close", () => record(null));
      forwarded.on("error", () => {
        record(null);
        if (outgoing.headersSent) outgoing.destroy();
        else outgoing.writeHead(502).end();
      });
      incoming.on("aborted", () => forwarded.destroy());
      outgoing.on("close", () => {
        if (!outgoing.writableFinished) forwarded.destroy();
      });
      incoming.pipe(forwarded);
    })().catch(() => outgoing.destroy());
  });
  server.listen(options.port, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  if (!address || typeof address === "string")
    throw new Error("Research gateway address unavailable");
  return {
    endpoint: `http://127.0.0.1:${address.port}${basePath}`,
    close: async () => {
      server.closeAllConnections();
      await new Promise<void>((done, reject) =>
        server.close((error) => (error ? reject(error) : done())),
      );
      await telemetry;
    },
  };
}
