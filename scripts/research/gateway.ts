import { createServer, request as httpRequest } from "node:http";
import { request as httpsRequest } from "node:https";
import { once } from "node:events";
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
      try {
        await guard.reserve();
      } catch {
        outgoing.writeHead(429, { "Content-Type": "application/json" });
        outgoing.end(
          '{"error":{"message":"Research call guard exhausted or unavailable"}}',
        );
        return;
      }
      const transport =
        upstream.protocol === "https:" ? httpsRequest : httpRequest;
      const forwarded = transport(
        target,
        {
          method: incoming.method,
          headers: { ...incoming.headers, host: upstream.host },
          timeout: 300000,
        },
        (response) => {
          outgoing.writeHead(response.statusCode ?? 502, response.headers);
          response.pipe(outgoing);
          response.on("error", () => outgoing.destroy());
        },
      );
      forwarded.on("timeout", () => forwarded.destroy());
      forwarded.on("error", () => {
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
    },
  };
}
