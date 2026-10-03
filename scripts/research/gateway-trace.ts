import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const sensitiveKey =
  /^(?:authorization|proxy-authorization|cookie|set-cookie|.*api[_-]?key|.*credential.*|.*secret.*|(?:access|refresh)[_-]?token|x[-_].*(?:token|key|auth)|token)$/i;

export function traceSecrets(
  headers: Record<string, string | string[] | undefined>,
) {
  const values: string[] = [];
  for (const [key, value] of Object.entries(headers)) {
    if (!sensitiveKey.test(key) || !value) continue;
    for (const entry of Array.isArray(value) ? value : [value]) {
      values.push(entry);
      if (/^Bearer /i.test(entry)) values.push(entry.slice(7));
      if (/cookie/i.test(key))
        for (const cookie of entry.split(";")) {
          const at = cookie.indexOf("=");
          if (at >= 0) values.push(cookie.slice(at + 1).trim());
        }
    }
  }
  return values.filter(Boolean).sort((a, b) => b.length - a.length);
}

function redact(text: string, secrets: readonly string[]) {
  for (const secret of secrets) text = text.split(secret).join("[REDACTED]");
  return text
    .replace(/Bearer\s+[^\s"\\<>]+/gi, "Bearer [REDACTED]")
    .replace(/\bsk-[A-Za-z0-9_-]{8,}\b/g, "[REDACTED]")
    .replace(
      /(?:authorization|proxy-authorization|set-cookie|cookie)\s*:\s*[^\r\n]+/gi,
      "[REDACTED HEADER]",
    );
}

function mediaDescriptor(data: string, mediaType: string) {
  const decoded = Buffer.from(data, "base64");
  return {
    media_type: mediaType,
    encoded_bytes: Buffer.byteLength(data),
    decoded_bytes: decoded.length,
    sha256: createHash("sha256").update(decoded).digest("hex"),
  };
}

function sanitize(
  value: unknown,
  secrets: readonly string[],
  context = "",
): unknown {
  if (typeof value === "string") {
    const data = /^data:([^;,]+)(?:;[^,]*)?;base64,([\s\S]*)$/.exec(value);
    if (data) return mediaDescriptor(data[2]!, data[1]!);
    if (context.startsWith("input_audio:"))
      return mediaDescriptor(value, `audio/${context.slice(12)}`);
    if (value.length > 32768 && /^[A-Za-z0-9+/=\r\n]+$/.test(value))
      return mediaDescriptor(value, "application/octet-stream");
    return redact(value, secrets);
  }
  if (Array.isArray(value))
    return value.map((entry) => sanitize(entry, secrets));
  if (value && typeof value === "object") {
    const object = value as Record<string, unknown>;
    return Object.fromEntries(
      Object.entries(object)
        .filter(([key]) => !sensitiveKey.test(key))
        .map(([key, entry]) => [
          key,
          sanitize(
            entry,
            secrets,
            key === "data" && context === "input_audio"
              ? `input_audio:${typeof object.format === "string" ? object.format : "unknown"}`
              : key,
          ),
        ]),
    );
  }
  return value;
}

/** Hash every wire byte while retaining at most a bounded prefix for local inspection. */
export class TraceBody {
  private readonly hash = createHash("sha256");
  private chunks: Buffer[] = [];
  size = 0;
  constructor(private readonly limit: number) {}
  add(chunk: Buffer) {
    this.hash.update(chunk);
    const remaining = Math.max(0, this.limit - this.size);
    if (remaining) this.chunks.push(chunk.subarray(0, remaining));
    this.size += chunk.length;
  }
  get truncated() {
    return this.size > this.limit;
  }
  bytes() {
    return Buffer.concat(this.chunks);
  }
  digest() {
    return this.hash.copy().digest("hex");
  }
  async save(root: string, name: string, secrets: readonly string[]) {
    let body: unknown;
    let format = "txt";
    if (!this.truncated) {
      try {
        body = sanitize(JSON.parse(this.bytes().toString("utf8")), secrets);
        format = "json";
      } catch {
        /* Preserve non-JSON responses as sanitized text. */
      }
    }
    // Incomplete JSON can contain a cut-off credential value. Preserve only its wire identity.
    const text = this.truncated
      ? JSON.stringify({
          captured: false,
          truncated: true,
          bytes: this.size,
          sha256: this.digest(),
        }) + "\n"
      : format === "json"
        ? JSON.stringify(body, null, 2) + "\n"
        : redact(this.bytes().toString("utf8"), secrets);
    const path = resolve(root, `${name}.${format}`);
    await writeFile(path, text, { flag: "wx", mode: 0o600 });
    this.chunks = [];
    return {
      file: `${name}.${format}`,
      bytes: this.size,
      sha256: this.digest(),
      captured: !this.truncated,
      truncated: this.truncated,
    };
  }
}

export async function writeGatewayTrace(
  root: string,
  attempt: number,
  meta: Record<string, unknown>,
  request: TraceBody,
  response: TraceBody,
  secrets: readonly string[],
) {
  const directory = resolve(root, String(attempt));
  await mkdir(directory, { recursive: true, mode: 0o700 });
  const requestInfo = await request.save(directory, "request", secrets);
  const responseInfo = await response.save(directory, "response", secrets);
  await writeFile(
    resolve(directory, "meta.json"),
    JSON.stringify(
      { attempt, ...meta, request: requestInfo, response: responseInfo },
      null,
      2,
    ) + "\n",
    { flag: "wx", mode: 0o600 },
  );
}
