// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
import { runCli } from "./commands.js";
import { cliErrorPayload } from "./agent.js";
import { renderText } from "./output.js";
import { friendlyOutput } from "./friendly.js";
const json = process.argv.includes("--json") || process.argv.includes("--raw");
const render = (value: unknown) =>
  json
    ? JSON.stringify(value, (_, v: unknown) =>
        typeof v === "bigint" ? v.toString() : v,
      )
    : renderText(value);
runCli(process.argv.slice(2), undefined, friendlyOutput)
  .then((result) => console.log(render(result)))
  .catch((error: unknown) => {
    const payload = cliErrorPayload(error);
    if (!json && !process.argv.includes("--developer"))
      payload.candidates = payload.candidates.map((candidate: unknown) => {
        if (
          candidate &&
          typeof candidate === "object" &&
          "lexicalRef" in candidate
        ) {
          const { canonical: _canonical, ...address } = candidate as Record<
            string,
            unknown
          >;
          return address;
        }
        return candidate;
      });
    console.error(render(payload));
    process.exitCode = 1;
  });
