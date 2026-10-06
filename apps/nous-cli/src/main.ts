import { runCli } from "./commands.js";
import { cliErrorPayload } from "./agent.js";
const compact = process.argv.includes("--json");
const json = (value: unknown) =>
  JSON.stringify(
    value,
    (_, v: unknown) => (typeof v === "bigint" ? v.toString() : v),
    compact ? undefined : 2,
  );
runCli(process.argv.slice(2))
  .then((result) => console.log(json(result)))
  .catch((error: unknown) => {
    console.error(json(cliErrorPayload(error)));
    process.exitCode = 1;
  });
