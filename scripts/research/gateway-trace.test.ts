import { expect, it } from "vitest";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { TraceBody, traceSecrets, writeGatewayTrace } from "./gateway-trace.js";

it("preserves text/schema and media identity while stripping echoed credentials", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-trace-"));
  try {
    const request = new TraceBody(1024 * 1024);
    const response = new TraceBody(1024 * 1024);
    const source = Buffer.from(
      JSON.stringify({
        model: "real-model",
        messages: [
          { role: "system", content: "real prompt" },
          {
            role: "user",
            content: [
              {
                type: "input_audio",
                input_audio: {
                  data: Buffer.from("media bytes").toString("base64"),
                  format: "mp3",
                },
              },
            ],
          },
        ],
        response_format: { json_schema: { schema: { type: "object" } } },
      }),
    );
    request.add(source);
    response.add(
      Buffer.from(
        JSON.stringify({
          error: {
            message: "echo Bearer private-token",
            headers: {
              authorization: "Bearer private-token",
              cookie: "session=private-cookie",
            },
            credential: "private-provider-secret",
          },
          usage: { total_tokens: 3 },
        }),
      ),
    );
    await writeGatewayTrace(
      root,
      1,
      { endpoint: "/chat/completions", status: 400 },
      request,
      response,
      traceSecrets({
        authorization: "Bearer private-token",
        cookie: "session=private-cookie",
      }),
    );
    const saved = await readFile(join(root, "1/request.json"), "utf8");
    expect(saved).toContain("real prompt");
    expect(saved).toContain("real-model");
    expect(saved).toContain('"media_type": "audio/mp3"');
    expect(saved).toContain('"decoded_bytes": 11');
    expect(saved).not.toContain(Buffer.from("media bytes").toString("base64"));
    const reply = await readFile(join(root, "1/response.json"), "utf8");
    for (const secret of [
      "private-token",
      "private-cookie",
      "private-provider-secret",
      "authorization",
      "cookie",
    ])
      expect(reply).not.toContain(secret);
    const meta = JSON.parse(
      await readFile(join(root, "1/meta.json"), "utf8"),
    ) as { request: { sha256: string; bytes: number } };
    expect(meta.request.sha256).toBe(
      createHash("sha256").update(source).digest("hex"),
    );
    expect(meta.request.bytes).toBe(source.length);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

it("hashes bytes beyond the capture bound without storing a partial credential", async () => {
  const root = await mkdtemp(join(tmpdir(), "nous-truncated-trace-"));
  try {
    const capture = new TraceBody(12);
    const source = Buffer.from('{"authorization":"Bearer private-token"}');
    capture.add(source.subarray(0, 8));
    capture.add(source.subarray(8));
    const info = await capture.save(root, "response", []);
    expect(info.truncated).toBe(true);
    expect(info.sha256).toBe(createHash("sha256").update(source).digest("hex"));
    expect(await readFile(join(root, info.file), "utf8")).not.toContain(
      "authorization",
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
