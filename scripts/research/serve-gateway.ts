import { parseArgs } from "node:util";
import { relative, resolve, sep } from "node:path";
import { startResearchGateway } from "./gateway.js";

const { values } = parseArgs({
  options: {
    upstream: { type: "string", default: "http://127.0.0.1:3000/v1" },
    ledger: { type: "string" },
    "trace-root": { type: "string" },
    "max-calls": { type: "string" },
    port: { type: "string", default: "18000" },
  },
});
if (!values.ledger || !values["max-calls"])
  throw new Error("Research gateway requires --ledger and --max-calls");
const traceRoot = values["trace-root"]
  ? resolve(values["trace-root"])
  : undefined;
if (traceRoot) {
  const rel = relative(resolve("data/research"), traceRoot);
  if (rel === ".." || rel.startsWith(`..${sep}`) || rel.startsWith(sep))
    throw new Error("Trace root must be under ignored data/research");
}
const gateway = await startResearchGateway({
  upstream: values.upstream,
  traceRoot,
  ledger: resolve(values.ledger),
  maxCalls: Number(values["max-calls"]),
  port: Number(values.port),
});
console.log(
  JSON.stringify({
    endpoint: gateway.endpoint,
    traceRoot,
    ledger: resolve(values.ledger),
  }),
);
await new Promise<void>((done) => {
  process.once("SIGINT", done);
  process.once("SIGTERM", done);
});
await gateway.close();
