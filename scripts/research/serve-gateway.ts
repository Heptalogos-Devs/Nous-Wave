import { parseArgs } from "node:util";
import { resolve } from "node:path";
import { startResearchGateway } from "./gateway.js";

const { values } = parseArgs({
  options: {
    upstream: { type: "string", default: "http://127.0.0.1:3000/v1" },
    ledger: { type: "string" },
    "max-calls": { type: "string" },
    port: { type: "string", default: "18000" },
  },
});
if (!values.ledger || !values["max-calls"])
  throw new Error("Research gateway requires --ledger and --max-calls");
const gateway = await startResearchGateway({
  upstream: values.upstream,
  ledger: resolve(values.ledger),
  maxCalls: Number(values["max-calls"]),
  port: Number(values.port),
});
console.log(
  JSON.stringify({
    endpoint: gateway.endpoint,
    ledger: resolve(values.ledger),
  }),
);
await new Promise<void>((done) => {
  process.once("SIGINT", done);
  process.once("SIGTERM", done);
});
await gateway.close();
