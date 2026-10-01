import { z } from "zod";

export const remoteEndpointSchema = z.string().transform((value, ctx) => {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    ctx.addIssue({ code: "custom", message: "Invalid remote endpoint URL" });
    return z.NEVER;
  }
  const loopback = url.hostname === "127.0.0.1" || url.hostname === "[::1]";
  if (
    url.username ||
    url.password ||
    url.search ||
    url.hash ||
    (url.protocol !== "https:" && !(url.protocol === "http:" && loopback))
  ) {
    ctx.addIssue({
      code: "custom",
      message:
        "Endpoint requires credential-free HTTPS or literal loopback HTTP",
    });
    return z.NEVER;
  }
  return url.toString().replace(/\/$/, "");
});
