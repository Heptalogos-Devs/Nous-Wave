// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { runCli } from "./commands.js";
import { cliErrorPayload } from "./agent.js";
import { renderText } from "./output.js";
const json = process.argv.includes("--json") || process.argv.includes("--raw");
const render = (value: unknown) =>
  json
    ? JSON.stringify(value, (_, v: unknown) =>
        typeof v === "bigint" ? v.toString() : v,
      )
    : renderText(value);
runCli(process.argv.slice(2))
  .then((result) => console.log(render(result)))
  .catch((error: unknown) => {
    console.error(render(cliErrorPayload(error)));
    process.exitCode = 1;
  });
