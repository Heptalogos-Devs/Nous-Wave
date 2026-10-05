import { parseArgs } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { ExecutionBudget } from "./execution-budget.js";
import { budgetOptions, executionLimits } from "./functional-plan.js";
import {
  launchFunctionalRuntime,
  type RuntimeMetrics,
} from "./functional-host.js";
interface Input {
  key: string;
  as_of: string;
  query: {
    subject: string;
    capabilities: { text_embedding: string; rerank: string };
    [key: string]: unknown;
  };
}
const { values } = parseArgs({
  options: {
    ...budgetOptions,
    plan: { type: "boolean" },
    input: { type: "string" },
    root: { type: "string" },
    "query-ids": { type: "string" },
    profiles: {
      type: "string",
      default: "baseline-rrf,nous-node-potential-v1",
    },
    "execution-id": { type: "string", default: "prepared-1" },
  },
});
if (!values.input || !values.root)
  throw new Error("--input and existing --root are required");
const root = resolve(values.root);
if (!root.startsWith(resolve("data/research") + "/"))
  throw new Error("Research root required");
if ((await stat(values.input)).size > 256 * 1024)
  throw new Error("Prepared input exceeds 256 KiB");
const inputBytes = await readFile(values.input, "utf8");
const all = (JSON.parse(inputBytes) as { queries: Input[] }).queries;
const ids = values["query-ids"]?.split(",");
if (ids?.some((key) => !all.some((q) => q.key === key)))
  throw new Error("Unknown query ID");
const queries = ids ? all.filter((q) => ids.includes(q.key)) : all;
if (
  !queries.length ||
  queries.length > 40 ||
  new Set(queries.map((q) => q.key)).size !== queries.length
)
  throw new Error("Invalid Prepared query set");
const profiles = values.profiles!.split(",");
if (
  profiles.some(
    (profile) => !["baseline-rrf", "nous-node-potential-v1"].includes(profile),
  )
)
  throw new Error(
    "VCP arms require the shared embedding-cache stage; this provider-free arm only supports baseline/native",
  );
if (
  queries.some(
    (q) =>
      q.query.capabilities.text_embedding !== "forbidden" ||
      q.query.capabilities.rerank !== "forbidden" ||
      !Number.isFinite(Date.parse(q.as_of)),
  )
)
  throw new Error(
    "Provider-free Prepared arm requires embedding/rerank forbidden and exact as_of",
  );
const limits = executionLimits(values);
const plan = {
  suite: "prepared-owner-native",
  queries: queries.length,
  subjects: new Set(queries.map((q) => q.query.subject)).size,
  profiles,
  uniqueTexts: new Set(queries.map((q) => JSON.stringify(q.query))).size,
  existingEmbeddings: 0,
  embeddingMisses: 0,
  estimatedEmbeddingRequests: 0,
  rerankRequests: 0,
  sharedServingAssetsReused: "existing compatible families reused",
  newAlgorithmAssets: queries.length ? 1 : 0,
  newServingGenerations: new Set(queries.map((q) => q.query.subject)).size * 3,
  estimatedNewArtifactBytes: { upperBound: limits.newArtifactBytes },
  externalDownloads: 0,
  estimatedRuntimeClass: `bounded by ${limits.runtimeSeconds} seconds`,
  limits,
  scope:
    "Provider-free structural Prepared readout; VCP semantic comparison follows the shared embedding cache preflight.",
};
const identity = createHash("sha256")
  .update(JSON.stringify({ inputBytes, plan }))
  .digest("hex");
const planPath = resolve(root, `${values["execution-id"]}.plan.json`);
if (values.plan) {
  await mkdir(root, { recursive: true });
  await writeFile(
    planPath,
    JSON.stringify({ identity, plan }, null, 2) + "\n",
    { mode: 0o600 },
  );
  console.log(JSON.stringify(plan, null, 2));
} else await run();
async function run() {
  const saved = JSON.parse(await readFile(planPath, "utf8")) as {
    identity: string;
  };
  if (saved.identity !== identity)
    throw new Error(
      "Run --plan with the exact Prepared inputs/profiles/budgets",
    );
  const budget = await ExecutionBudget.open(
    resolve(root, `${values["execution-id"]}.budget.json`),
    identity,
    limits,
  );
  const results: unknown[] = [];
  let host: Awaited<ReturnType<typeof launchFunctionalRuntime>> | undefined;
  let initial: RuntimeMetrics | undefined, last: RuntimeMetrics | undefined;
  try {
    await budget.reserve({ newArtifactBytes: 16 * 1024 * 1024 });
    host = await launchFunctionalRuntime(
      root,
      randomUUID() + randomUUID(),
      budget.signal,
      true,
    );
    initial = host.metrics;
    const checkpoint = async () => {
      last = (await host!.control({
        command: "metrics",
      })) as unknown as RuntimeMetrics;
      await budget.observeArtifactBytes(
        Math.max(0, last.runBytes - host!.baselineBytes),
      );
      await budget.observeServingGenerations(
        Math.max(0, last.generations - host!.baselineGenerations),
      );
      budget.signal.throwIfAborted();
    };
    // Freeze every representation and context before executing any algorithm arm.
    for (const query of queries) {
      await host.control({
        command: "advance",
        subject: query.query.subject,
        instant: query.as_of,
      });
      results.push({
        preparation: await host.control({
          command: "prepare",
          key: query.key,
          query: query.query,
        }),
      });
    }
    await checkpoint();
    for (const query of queries) {
      for (const profile of profiles) {
        await budget.reserve({ newArtifactBytes: 32 * 1024 * 1024 });
        if (
          last!.generations - host!.baselineGenerations >=
          limits.newServingGenerations
        )
          throw new Error(
            "Serving generation allowance exhausted before readout",
          );
        const result = await host.control({
          command: "execute",
          key: query.key,
          profile,
        });
        results.push(result);
        await checkpoint();
        await writeFile(
          resolve(root, `${values["execution-id"]}.rows.json`),
          JSON.stringify(results, null, 2) + "\n",
          { mode: 0o600 },
        );
      }
    }
  } catch (error) {
    await budget.stop(
      budget.snapshot().stopReason ??
        (error instanceof Error ? error.message : "Prepared run failed"),
    );
  } finally {
    if (host) {
      try {
        last = (await host.control({
          command: "metrics",
        })) as unknown as RuntimeMetrics;
      } catch {
        /* startup/transport failures can close the host */
      }
      await host.close();
    }
    await budget.close();
    await writeFile(
      resolve(root, `${values["execution-id"]}.result.json`),
      JSON.stringify(
        { plan, budget: budget.snapshot(), initial, last, rows: results },
        null,
        2,
      ) + "\n",
      { mode: 0o600 },
    );
  }
  console.log(
    JSON.stringify({
      queries: queries.length,
      completedArms: results.filter(
        (row) => row && typeof row === "object" && "profile" in row,
      ).length,
      budget: budget.snapshot().used,
      stopReason: budget.snapshot().stopReason,
    }),
  );
}
