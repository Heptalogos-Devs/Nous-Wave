import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { parseArgs, parseEnv } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { connectNous } from "@nous-wave/client/node";
import { createCore } from "../../apps/nous-core/src/server.js";
import { KernelClient } from "../../apps/nous-core/src/kernel-client.js";
import {
  parseConfiguration,
  parseEffectiveConfiguration,
} from "../../apps/nous-core/src/config.js";
import { ModelRuntime } from "../../apps/nous-core/src/model/runtime.js";
import { ExecutionBudget } from "./execution-budget.js";
import {
  budgetOptions,
  executionLimits,
  formationPlan,
  loadFunctionalCorpus,
} from "./functional-plan.js";
import {
  launchFunctionalRuntime,
  type RuntimeInspection,
  type RuntimeMetrics,
} from "./functional-host.js";
import { startResearchGateway } from "./gateway.js";
const { values } = parseArgs({
  options: {
    ...budgetOptions,
    plan: { type: "boolean" },
    corpus: { type: "string", default: "docs/research/corpus/functional" },
    root: {
      type: "string",
      default: "data/research/runs/functional-cognition-v1",
    },
    config: { type: "string" },
    "env-file": { type: "string" },
    "query-ids": { type: "string" },
    "scenario-ids": { type: "string" },
    "review-need-ids": { type: "string" },
    "operation-timeout-seconds": { type: "string", default: "90" },
    "execution-id": { type: "string", default: "formation-1" },
  },
});
const root = resolve(values.root!);
if (!root.startsWith(resolve("data/research") + "/"))
  throw new Error("Run root must be under data/research");
const limits = executionLimits(values);
const operationTimeoutSeconds = Number(values["operation-timeout-seconds"]);
if (
  !Number.isSafeInteger(operationTimeoutSeconds) ||
  operationTimeoutSeconds < 1 ||
  operationTimeoutSeconds > 300
)
  throw new Error("Operation timeout must be 1..300 seconds");
const corpus = await loadFunctionalCorpus(
  values.corpus!,
  values["query-ids"]?.split(","),
);
const selected =
  values["scenario-ids"]?.split(",") ?? corpus.scenarios.map((s) => s.key);
if (selected.some((key) => !corpus.scenarios.some((s) => s.key === key)))
  throw new Error("Unknown scenario selection");
const scenarios = corpus.scenarios.filter((s) => selected.includes(s.key));
const plan = {
  ...(await formationPlan(
    values.corpus!,
    limits,
    values["query-ids"]?.split(","),
  )),
  formationSchedule:
    "deterministic-experience-then-bounded-five-minute-opportunities-v2",
  subjects: scenarios.length,
  events: scenarios.reduce((n, s) => n + s.events.length, 0),
  queries: corpus.queries.filter((query) =>
    selected.includes(query.scenario_key),
  ).length,
  uniqueTexts: new Set(
    scenarios.flatMap((s) => s.events.map((event) => event.text)),
  ).size,
  scenarioIds: selected,
  operationTimeoutSeconds,
  reviewNeedIds: values["review-need-ids"]?.split(",") ?? [],
  newServingGenerations: scenarios.reduce(
    (total, scenario) => total + scenario.events.length + 1,
    0,
  ),
  servingExplanation:
    "Formation needs lexical context lookup for existing Memory/Schema reuse. Each accepted mutation may refresh that shared lexical catalog; estimate one refresh per source event plus startup. Dense and topology readout stay disabled until validation.",
  startupArtifactAllowance: 128 * 1024 * 1024,
  perMutationArtifactAllowance: 32 * 1024 * 1024,
};
const identity = createHash("sha256")
  .update(
    JSON.stringify({
      plan,
      config: values.config ? await readFile(values.config, "utf8") : null,
    }),
  )
  .digest("hex");
if (values.plan) {
  await mkdir(root, { recursive: true });
  await writeFile(
    resolve(root, "formation-plan.json"),
    JSON.stringify({ identity, plan }, null, 2) + "\n",
    { mode: 0o600 },
  );
  console.log(JSON.stringify(plan, null, 2));
} else await run();
function operation(key: string) {
  const hex = createHash("sha256")
    .update(corpus.sourceDigest + ":" + key)
    .digest("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-4${hex.slice(13, 16)}-a${hex.slice(17, 20)}-${hex.slice(20, 32)}`;
}
interface SubjectState {
  id: string;
  workContext?: string;
  cognitiveInstant?: string;
  sessions: Record<string, string>;
  events: Record<string, unknown>;
}
interface State {
  digest: string;
  subjects: Record<string, SubjectState>;
  grants: unknown[];
}
async function run() {
  const approved = JSON.parse(
    await readFile(resolve(root, "formation-plan.json"), "utf8"),
  ) as { identity: string };
  if (approved.identity !== identity)
    throw new Error(
      "Run parameters changed; run --plan with these exact inputs and limits",
    );
  const budget = await ExecutionBudget.open(
    resolve(root, `${values["execution-id"]}.budget.json`),
    identity,
    limits,
  );
  let state: State = { digest: corpus.sourceDigest, subjects: {}, grants: [] };
  try {
    state = JSON.parse(
      await readFile(resolve(root, "formation-state.json"), "utf8"),
    ) as State;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  if (state.digest !== corpus.sourceDigest)
    throw new Error("Run corpus differs from its durable receipts");
  const diskBefore = await researchBytes();
  const proxies: Awaited<ReturnType<typeof startResearchGateway>>[] = [];
  let host: Awaited<ReturnType<typeof launchFunctionalRuntime>> | undefined;
  let app: Awaited<ReturnType<typeof createCore>> | undefined;
  let initial: RuntimeMetrics | undefined, last: RuntimeMetrics | undefined;
  const flush = () =>
    writeFile(
      resolve(root, "formation-state.json"),
      JSON.stringify(state, null, 2) + "\n",
      { mode: 0o600 },
    );
  try {
    await stat(resolve("target/debug/examples/functional-runtime"));
    await budget.reserve({ newArtifactBytes: plan.startupArtifactAllowance });
    const token = randomUUID() + randomUUID();
    host = await launchFunctionalRuntime(root, token, budget.signal);
    initial = host.metrics;
    last = initial;
    await budget.observeArtifactBytes(
      Math.max(0, last.runBytes - host!.baselineBytes),
    );
    await budget.observeServingGenerations(
      Math.max(0, initial.generations - host.baselineGenerations),
    );
    budget.signal.throwIfAborted();
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
      await flush();
      budget.signal.throwIfAborted();
    };
    const mutation = async <T>(perform: () => Promise<T>): Promise<T> => {
      await budget.reserve({
        newArtifactBytes: plan.perMutationArtifactAllowance,
      });
      const result = await perform();
      await checkpoint();
      return result;
    };
    let models = new ModelRuntime();
    if (values.config) {
      const { document } = parseConfiguration(
        await readFile(values.config, "utf8"),
        true,
      );
      const configuration = parseEffectiveConfiguration(document).models;
      if (values["env-file"]) {
        const secrets = parseEnv(await readFile(values["env-file"], "utf8"));
        for (const profile of Object.values(configuration.gateway_profiles)) {
          if (
            process.env[profile.credential_env] === undefined &&
            secrets[profile.credential_env] !== undefined
          )
            process.env[profile.credential_env] =
              secrets[profile.credential_env];
        }
      }
      for (const role of Object.keys(configuration.roles)) {
        if (
          ![
            "episode_segmentation",
            "journal_synthesis",
            "memory_consolidation",
            "topology_maintenance",
          ].includes(role)
        )
          delete configuration.roles[role as keyof typeof configuration.roles];
      }
      let ports: Record<string, number> = {};
      const portsPath = resolve(root, "gateway-ports.json");
      try {
        ports = JSON.parse(await readFile(portsPath, "utf8")) as Record<
          string,
          number
        >;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
      }
      for (const [key, profile] of Object.entries(
        configuration.gateway_profiles,
      )) {
        const destination = createHash("sha256")
          .update(profile.base_url)
          .digest("hex");
        const proxy = await startResearchGateway({
          upstream: profile.base_url,
          ledger: resolve(root, `${values["execution-id"]}.${key}.wire`),
          maxCalls: 0,
          port: ports[destination] ?? 0,
          executionBudget: budget,
          traceRoot: resolve(root, "traces", values["execution-id"]!, key),
        });
        proxies.push(proxy);
        ports[destination] = Number(new URL(proxy.endpoint).port);
        await writeFile(portsPath, JSON.stringify(ports, null, 2) + "\n", {
          mode: 0o600,
        });
        profile.base_url = proxy.endpoint;
      }
      models = await ModelRuntime.fromConfig(
        configuration,
        resolve("prompts"),
        undefined,
        resolve(root, "temp"),
      );
    }
    app = await createCore({
      kernel: KernelClient.connect(host.endpoint, token),
      token,
      consumers: [],
      models,
    });
    const client = connectNous(
      await app.listen({ host: "127.0.0.1", port: 0 }),
      token,
    );
    const options = {
      signal: budget.signal,
      timeoutMs: (operationTimeoutSeconds + 15) * 1000,
    };
    for (const scenario of scenarios) {
      let saved = state.subjects[scenario.key];
      if (!saved) {
        saved = {
          id: operation(scenario.key + ":subject"),
          sessions: {},
          events: {},
        };
        state.subjects[scenario.key] = saved;
      }
      if (
        !(await client.subjects.list({}, options)).items.some(
          (subject) => subject.subjectId === saved.id,
        )
      ) {
        await mutation(() =>
          client.subjects.create(
            {
              subjectId: saved.id,
              operationId: operation(scenario.key + ":create"),
              cognitiveSeed: {
                text: "schema_version = 1",
                format:
                  "application/vnd.nous-wave.cognitive-seed+toml;version=1",
                provenance: { corpusDigest: corpus.sourceDigest },
              },
            },
            options,
          ),
        );
      }
      for (const entity of scenario.entities) {
        await mutation(() =>
          client.identity.bind(
            {
              subjectId: saved.id,
              canonical: {
                kind: "entity",
                value: `entity:functional:${scenario.key}:${entity.key}`,
              },
              displayName: entity.display_name,
              aliases: entity.aliases,
            },
            options,
          ),
        );
      }
      if (!saved.workContext) {
        const response = await mutation(() =>
          client.cognition.createWorkContext(
            {
              operationId: operation(scenario.key + ":work"),
              subjectId: saved.id,
              purpose: scenario.work_context.purpose,
              unresolvedQuestions: scenario.work_context.open_questions,
            },
            options,
          ),
        );
        saved.workContext = response.workContext!.workContextId;
        await flush();
      }
      for (const event of scenario.events) {
        await host.control({
          command: "advance",
          subject: saved.id,
          instant: event.observed_at,
        });
        if (saved.events[event.key]) continue;
        let sessionId = saved.sessions[event.session_key];
        if (!sessionId) {
          const response = await mutation(() =>
            client.cognition.openSession({ subjectId: saved.id }, options),
          );
          sessionId = response.sessionId;
          saved.sessions[event.session_key] = sessionId;
          await flush();
        }
        const response = await mutation(() =>
          client.cognition.observe(
            {
              subjectId: saved.id,
              sessionId,
              requestId: operation(scenario.key + ":" + event.key),
              sourceClass: "message",
              externalObjectRef: `object:functional:${scenario.key}:${event.key}`,
              material: {
                case: "inlineText",
                value: { text: event.text, mediaType: "text/plain" },
              },
              observedAt: timestampFromDate(new Date(event.observed_at)),
              occurredTime: {
                value: {
                  case: "instant",
                  value: timestampFromDate(new Date(event.occurred_at)),
                },
              },
              context: {
                fixtureEventKey: event.key,
                entityRefs: event.entity_keys.map(
                  (key) => `entity:functional:${scenario.key}:${key}`,
                ),
              },
            },
            options,
          ),
        );
        saved.events[event.key] = response;
        await flush();
      }
      await host.control({
        command: "advance",
        subject: saved.id,
        instant: saved.cognitiveInstant ?? "2026-09-16T12:30:00Z",
      });
      // Close deterministic experience formation before spending model budget.
      // Episode acceptance creates delayed journal/consolidation needs; expose
      // those due opportunities at the corpus query time before Tag maintenance.
      const organized = await mutation(() =>
        client.cognition.grantMaintenance(
          {
            subjectId: saved.id,
            maxOperations: 1,
            maxModelCalls: 0,
            maxElapsedMs: operationTimeoutSeconds * 1000,
          },
          options,
        ),
      );
      state.grants.push(organized);
      await flush();
      await host.control({
        command: "advance",
        subject: saved.id,
        instant: saved.cognitiveInstant ?? "2026-09-16T18:00:00Z",
      });
      if (plan.reviewNeedIds.length && scenarios.length !== 1)
        throw new Error("Explicit review requires one selected Subject");
      if (plan.reviewNeedIds.length)
        await mutation(() =>
          host!.control({
            command: "review",
            subject: saved.id,
            need_ids: plan.reviewNeedIds,
          }),
        );
      const queryHorizon = Math.min(
        ...corpus.queries
          .filter((query) => query.scenario_key === scenario.key)
          .map((query) => Date.parse(query.prepared_query.as_of)),
      );
      for (let pass = 0; pass < 40; pass++) {
        const instant = Math.max(
          Date.parse("2026-09-16T18:00:00Z"),
          Date.parse(saved.cognitiveInstant ?? "2026-09-16T18:00:00Z"),
        );
        if (instant > queryHorizon) break;
        await host.control({
          command: "advance",
          subject: saved.id,
          instant: new Date(instant).toISOString(),
        });
        saved.cognitiveInstant = new Date(instant + 5 * 60_000).toISOString();
        await flush();
        const grant = await mutation(() =>
          client.cognition.grantMaintenance(
            {
              subjectId: saved.id,
              maxOperations: 1,
              maxModelCalls: 1,
              maxElapsedMs: operationTimeoutSeconds * 1000,
            },
            options,
          ),
        );
        state.grants.push(grant);
        await flush();
        if (
          grant.results.some(
            (result) =>
              result.status === "retry" || result.status === "rejected_invalid",
          )
        ) {
          await budget.stop(
            "maintenance proposal failed; inspect the retained workflow/trace before further calls",
          );
          break;
        }
        if (!grant.results.length) {
          const inspection = (await host.control({
            command: "inspect",
          })) as unknown as RuntimeInspection;
          const due = (
            inspection.needs as {
              subject_id: string;
              state: string;
              due_at: string;
            }[]
          )
            .filter(
              (need) =>
                need.subject_id === saved.id && need.state === "pending",
            )
            .map((need) => Date.parse(need.due_at))
            .sort((a, b) => a - b)[0];
          if (due !== undefined && due > instant && due <= queryHorizon) {
            saved.cognitiveInstant = new Date(
              Math.max(instant, due + 1000),
            ).toISOString();
            await flush();
            continue;
          }
          break;
        }
      }
    }
    const inspection = (await host.control({
      command: "inspect",
    })) as unknown as RuntimeInspection;
    await writeFile(
      resolve(root, "formation-inspection.json"),
      JSON.stringify(inspection, null, 2) + "\n",
      { mode: 0o600 },
    );
    last = inspection.metrics;
  } catch (error) {
    await budget.stop(
      budget.snapshot().stopReason ??
        (error instanceof Error ? error.message : "formation failed"),
    );
  } finally {
    if (host) {
      try {
        const inspection = (await host.control({
          command: "inspect",
        })) as unknown as RuntimeInspection;
        last = inspection.metrics;
        await writeFile(
          resolve(root, "formation-inspection.json"),
          JSON.stringify(inspection, null, 2) + "\n",
          { mode: 0o600 },
        );
      } catch {
        /* startup failures can close the host before a snapshot exists */
      }
    }
    await flush();
    await app?.close();
    for (const proxy of proxies) await proxy.close();
    await host?.close();
    await budget.close();
    await writeFile(
      resolve(root, `${values["execution-id"]}.result.json`),
      JSON.stringify(
        {
          budget: budget.snapshot(),
          initial,
          last,
          researchBytesBefore: diskBefore,
          researchBytesAfter: await researchBytes(),
          qualityAcceptance:
            "pending semantic inspection; no retrieval result claimed",
        },
        null,
        2,
      ) + "\n",
      { mode: 0o600 },
    );
  }
  console.log(
    JSON.stringify({
      suite: corpus.manifest.suite,
      subjects: Object.keys(state.subjects).length,
      observations: Object.values(state.subjects).reduce(
        (n, s) => n + Object.keys(s.events).length,
        0,
      ),
      budget: budget.snapshot().used,
      stopReason: budget.snapshot().stopReason,
    }),
  );
}

async function researchBytes() {
  const { stdout } = await promisify(execFile)(
    "du",
    ["-sb", resolve("data/research")],
    { timeout: 10000 },
  );
  const bytes = Number(stdout.split("\t")[0]);
  if (!Number.isSafeInteger(bytes) || bytes < 0)
    throw new Error("Research disk inventory unavailable");
  return bytes;
}
