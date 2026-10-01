import { describe, expect, it } from "vitest";
import { createServer } from "node:http";
import { RagflowAdapter } from "../src/resources/providers/ragflow.js";
import { resourceProfilesSchema } from "../src/resources/configuration.js";
import { ResourceRegistry } from "../src/resources/registry.js";

describe("RAGFlow wire and stable material boundary", () => {
  it("normalizes the pinned API and rejects changed, withdrawn, foreign and oversized material", async () => {
    let content = "source fact";
    let state: "normal" | "foreign" | "denied" | "missing" = "normal";
    const requests: { method: string; path: string; body: unknown }[] = [];
    const provider = createServer((request, response) => {
      void (async () => {
        const buffers: Buffer[] = [];
        for await (const chunk of request) {
          const bytes: unknown = chunk;
          if (!(bytes instanceof Uint8Array)) throw new Error("request bytes");
          buffers.push(Buffer.from(bytes));
        }
        requests.push({
          method: request.method!,
          path: request.url!,
          body: buffers.length
            ? (JSON.parse(Buffer.concat(buffers).toString()) as unknown)
            : undefined,
        });
        response.setHeader("Content-Type", "application/json");
        if (state === "denied") {
          response.writeHead(403);
          response.end('{"error":"fixture-credential-should-never-escape"}');
          return;
        }
        if (state === "missing") {
          response.end('{"code":100,"message":"entry deleted"}');
          return;
        }
        const data =
          request.method === "POST"
            ? {
                chunks: [
                  {
                    id: "chunk1",
                    dataset_id: state === "foreign" ? "foreign" : "dataset1",
                    document_id: "document1",
                    content,
                    document_keyword: "source.txt",
                    similarity: 0.8,
                  },
                ],
              }
            : request.url?.startsWith("/api/v1/datasets?")
              ? [{ id: "dataset1" }]
              : {
                  id: "chunk1",
                  doc_id: "document1",
                  content_with_weight: content,
                  available_int: 1,
                };
        response.end(JSON.stringify({ code: 0, data }));
      })().catch(() => response.destroy());
    });
    await new Promise<void>((done) => provider.listen(0, "127.0.0.1", done));
    const address = provider.address();
    if (!address || typeof address === "string")
      throw new Error("fixture port");
    const profiles = resourceProfilesSchema.parse({
      local: {
        adapter_kind: "ragflow",
        base_url: `http://127.0.0.1:${address.port}/api/v1`,
        credential_env: "NOUS_RESOURCE_QUALIFICATION",
        max_material_bytes: 32,
      },
    });
    const adapter = new RagflowAdapter(
      "local",
      profiles.local!,
      "fixture-credential-should-never-escape",
    );
    const binding = {
      resourceRef: "resource:kb",
      adapterKind: "ragflow",
      providerProfile: "local",
      providerLocator: JSON.stringify({ dataset_ids: ["dataset1"] }),
    };
    try {
      expect((await adapter.describe(binding)).accessible).toBe(true);
      const [record] = await adapter.search(binding, "source paraphrase", 2);
      expect(requests[1]).toEqual({
        method: "POST",
        path: "/api/v1/retrieval",
        body: {
          question: "source paraphrase",
          dataset_ids: ["dataset1"],
          page: 1,
          page_size: 2,
          highlight: false,
        },
      });
      expect(record?.providerRank).toBe(1);
      expect(record?.providerScore).toBe(0.8);
      expect(record?.reference.entryVersion).toBeNull();
      expect(record?.reference.contentDigest).toMatch(/^[a-f0-9]{64}$/);
      expect((await adapter.materialize(record!.reference)).content).toBe(
        content,
      );
      expect(requests[2]?.path).toBe(
        "/api/v1/datasets/dataset1/documents/document1/chunks/chunk1",
      );
      expect(
        (
          await adapter.checkVersion({
            ...record!.reference,
            profileDigest: "0".repeat(64),
          })
        ).status,
      ).toBe("stale");
      content = "changed fact";
      expect((await adapter.checkVersion(record!.reference)).status).toBe(
        "stale",
      );
      await expect(adapter.materialize(record!.reference)).rejects.toThrow(
        "content changed",
      );
      state = "denied";
      expect((await adapter.checkAccess(record!.reference)).status).toBe(
        "denied",
      );
      try {
        await adapter.search(binding, "question", 1);
        throw new Error("expected denial");
      } catch (error) {
        expect(String(error)).toContain("HTTP 403");
        expect(String(error)).not.toContain("fixture-credential");
      }
      state = "missing";
      expect((await adapter.checkVersion(record!.reference)).status).toBe(
        "missing",
      );
      state = "foreign";
      await expect(adapter.search(binding, "question", 1)).rejects.toThrow(
        "outside its selector",
      );
      state = "normal";
      content = "x".repeat(33);
      await expect(adapter.search(binding, "question", 1)).rejects.toThrow(
        "byte bound",
      );
      const abort = new AbortController();
      abort.abort(new Error("caller cancelled"));
      await expect(
        adapter.search(binding, "question", 1, abort.signal),
      ).rejects.toThrow("caller cancelled");
      expect(new ResourceRegistry({}).resolve(binding)).toBeUndefined();
      const previous = process.env.NOUS_RESOURCE_QUALIFICATION;
      process.env.NOUS_RESOURCE_QUALIFICATION = "controlled-fixture";
      try {
        expect(
          new ResourceRegistry(profiles).resolve(binding)?.profileDigest,
        ).toBe(adapter.profileDigest);
      } finally {
        if (previous === undefined)
          delete process.env.NOUS_RESOURCE_QUALIFICATION;
        else process.env.NOUS_RESOURCE_QUALIFICATION = previous;
      }
    } finally {
      await new Promise<void>((done) => provider.close(() => done()));
    }
  });
});
