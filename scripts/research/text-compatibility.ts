import { parseArgs } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { connectNous } from "@nous-wave/client/node";
import { createCore } from "../../apps/nous-core/src/server.js";
import { KernelClient } from "../../apps/nous-core/src/kernel-client.js";
import { ExecutionBudget } from "./execution-budget.js";
import { budgetOptions, executionLimits } from "./functional-plan.js";
import { launchFunctionalRuntime } from "./functional-host.js";

interface Source {
  key: string;
  text: string;
  occurred_at: string;
}
interface Case {
  key: string;
  suite: string;
  text: string;
  as_of: string;
  sources: Source[];
}
interface Receipts {
  digest: string;
  subjects: Record<string, string>;
  sources: Record<
    string,
    { occurrence: string; memory: string; revision: string }
  >;
}
const { values } = parseArgs({
  options: {
    ...budgetOptions,
    plan: { type: "boolean" },
    input: { type: "string" },
    root: { type: "string" },
    "execution-id": { type: "string", default: "text-formation-1" },
  },
});
if (!values.input || !values.root)
  throw new Error("--input and --root required");
const root = resolve(values.root);
if (!root.startsWith(resolve("data/research") + "/"))
  throw new Error("Research-owned root required");
if ((await stat(values.input)).size > 256 * 1024)
  throw new Error("Selected text input exceeds 256 KiB");
const bytes = await readFile(values.input, "utf8");
const cases = (JSON.parse(bytes) as { cases: Case[] }).cases;
const sources = new Map<string, { suite: string; source: Source }>();
for (const item of cases) {
  for (const source of item.sources) {
    const previous = sources.get(source.key);
    if (
      previous &&
      (previous.suite !== item.suite ||
        JSON.stringify(previous.source) !== JSON.stringify(source))
    )
      throw new Error("Conflicting selected source");
    if (!source.text.trim() || !Number.isFinite(Date.parse(source.occurred_at)))
      throw new Error("Source text/time missing");
    sources.set(source.key, { suite: item.suite, source });
  }
}
const suites = [...new Set(cases.map((item) => item.suite))];
if (!cases.length || cases.length > 8 || sources.size > 20 || suites.length > 4)
  throw new Error("Selected compatibility envelope exceeded");
if (cases.some((item) => !Number.isFinite(Date.parse(item.as_of))))
  throw new Error("Query time missing");
const limits = executionLimits(values);
if (limits.providerCalls || limits.newEmbeddingItems || limits.rerankCalls)
  throw new Error("Controlled source formation is provider-free");
const digest = createHash("sha256").update(bytes).digest("hex");
const plan = {
  suite: "selected-text-compatibility-v1",
  phase: "controlled-source-memory",
  digest,
  subjects: suites.length,
  cases: cases.length,
  sources: sources.size,
  limits,
  sourceFormation:
    "Unmodified source text, direct occurrence support, no model",
  queryInput:
    "Original question only; no oracle, Entity, Tag or WorkContext cues",
  retrievalAsOf:
    "2026-09-16T19:00:00Z; source times preserved; benchmark dates are selection metadata",
  retrieval: "Separate shared-cache Prepared readout; no benchmark generation",
};
const identity = createHash("sha256")
  .update(JSON.stringify(plan))
  .digest("hex");
const planPath = resolve(root, `${values["execution-id"]}.plan.json`);
await mkdir(root, { recursive: true });
if (values.plan) {
  await writeFile(
    planPath,
    JSON.stringify({ identity, plan }, null, 2) + "\n",
    {
      mode: 0o600,
    },
  );
  console.log(JSON.stringify(plan, null, 2));
} else await run();

async function run() {
  const approved = JSON.parse(await readFile(planPath, "utf8")) as {
    identity: string;
  };
  if (approved.identity !== identity)
    throw new Error("Run --plan with exact inputs/limits");
  const budget = await ExecutionBudget.open(
    resolve(root, `${values["execution-id"]}.budget.json`),
    identity,
    limits,
  );
  let host: Awaited<ReturnType<typeof launchFunctionalRuntime>> | undefined;
  let app: Awaited<ReturnType<typeof createCore>> | undefined;
  let state: Receipts = { digest, subjects: {}, sources: {} };
  const statePath = resolve(root, "text-receipts.json");
  try {
    state = JSON.parse(await readFile(statePath, "utf8")) as Receipts;
    if (state.digest !== digest)
      throw new Error("Selected text receipts differ from input");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  const save = () =>
    writeFile(statePath, JSON.stringify(state, null, 2) + "\n", {
      mode: 0o600,
    });
  const operation = (key: string) => {
    const hex = createHash("sha256")
      .update(digest + ":" + key)
      .digest("hex");
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-4${hex.slice(13, 16)}-a${hex.slice(17, 20)}-${hex.slice(20, 32)}`;
  };
  try {
    await budget.reserve({ newArtifactBytes: 128 * 1024 * 1024 });
    const token = randomUUID() + randomUUID();
    host = await launchFunctionalRuntime(root, token, budget.signal);
    const checkpoint = async () => {
      const metrics = await host!.control({ command: "metrics" });
      await budget.observeArtifactBytes(
        Math.max(0, Number(metrics.runBytes) - host!.baselineBytes),
      );
      await budget.observeServingGenerations(
        Math.max(0, Number(metrics.generations) - host!.baselineGenerations),
      );
      budget.signal.throwIfAborted();
      await save();
    };
    app = await createCore({
      kernel: KernelClient.connect(host.endpoint, token),
      token,
      consumers: [],
    });
    const client = connectNous(
      await app.listen({ host: "127.0.0.1", port: 0 }),
      token,
    );
    const options = { signal: budget.signal, timeoutMs: 30_000 };
    const existingSubjects = new Set(
      (await client.subjects.list({}, options)).items.map(
        (item) => item.subjectId,
      ),
    );
    for (const suite of suites) {
      const subjectId = state.subjects[suite] ?? operation(suite + ":subject");
      if (existingSubjects.has(subjectId)) {
        state.subjects[suite] = subjectId;
        continue;
      }
      await budget.reserve({ newArtifactBytes: 32 * 1024 * 1024 });
      await client.subjects.create(
        {
          subjectId,
          operationId: operation(suite + ":create"),
          cognitiveSeed: {
            text: "schema_version = 1",
            format: "application/vnd.nous-wave.cognitive-seed+toml;version=1",
          },
        },
        options,
      );
      state.subjects[suite] = subjectId;
      await checkpoint();
    }
    for (const [key, { suite, source }] of sources) {
      if (state.sources[key]) continue;
      const subjectId = state.subjects[suite]!;
      await budget.reserve({ newArtifactBytes: 32 * 1024 * 1024 });
      const observed = await client.cognition.observe(
        {
          subjectId,
          requestId: operation(key + ":observe"),
          sourceClass: "message",
          observedAt: timestampFromDate(new Date(source.occurred_at)),
          occurredTime: {
            value: {
              case: "instant",
              value: timestampFromDate(new Date(source.occurred_at)),
            },
          },
          admit: true,
          material: {
            case: "inlineText",
            value: { text: source.text, mediaType: "text/plain" },
          },
        },
        options,
      );
      const memory = await client.memory.form(
        {
          subjectId,
          operationId: operation(key + ":form"),
          input: {
            cognitiveRole: "declarative",
            formationMode: "grounded",
            groundingOccurrenceId: observed.occurrenceId,
            semanticRole: "source_statement",
            text: source.text,
            epistemicClass: "reported",
            supports: [
              {
                support: {
                  case: "evidence",
                  value: {
                    occurrenceId: observed.occurrenceId,
                    locator: { case: "wholeOccurrence", value: true },
                    supportRole: "direct",
                  },
                },
              },
            ],
          },
        },
        options,
      );
      state.sources[key] = {
        occurrence: observed.occurrenceId,
        memory: memory.memoryId,
        revision: memory.revisionId,
      };
      await checkpoint();
    }
    const queries = cases.map((item) => ({
      key: item.key,
      as_of: "2026-09-16T19:00:00Z",
      query: {
        api_version: 1,
        subject: state.subjects[item.suite],
        session: null,
        work_context: null,
        situation: {},
        text_only_compatibility: true,
        expression: {
          operation: "atom",
          targets: [{ kind: "memory" }],
          cues: [{ kind: "text", text: item.text }],
          constraints: {},
        },
        exploration: "none",
        resources: {},
        result_need: { limit: 5 },
        effort: "normal",
        capabilities: {
          text_embedding: "preferred",
          rerank: "forbidden",
          residual_sensing: "forbidden",
          multimodal_interpretation: "forbidden",
        },
        diagnostics: "full",
      },
    }));
    await budget.reserve({ newArtifactBytes: 1024 * 1024 });
    await writeFile(
      resolve(root, "text-prepared-input.json"),
      JSON.stringify({ queries }, null, 2) + "\n",
      { mode: 0o600 },
    );
    await writeFile(
      resolve(root, "text-inspection.json"),
      JSON.stringify(await host.control({ command: "inspect" }), null, 2) +
        "\n",
      { mode: 0o600 },
    );
    await checkpoint();
    console.log(
      JSON.stringify({
        sources: Object.keys(state.sources).length,
        queries: queries.length,
        budget: budget.snapshot(),
      }),
    );
  } finally {
    await app?.close();
    await host?.close();
    await budget.close();
  }
}
