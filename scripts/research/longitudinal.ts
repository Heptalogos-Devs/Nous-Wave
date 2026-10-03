import { parseArgs } from "node:util";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { fromJson, toJson, type JsonValue } from "@bufbuild/protobuf";
import { MaintenancePlanSchema } from "@nous-wave/protocol/nous/wave/kernel/v1alpha1/longitudinal_pb.js";
import { ProducerSignatureSchema } from "@nous-wave/protocol/nous/wave/v1alpha1/types_pb.js";
import { create } from "@bufbuild/protobuf";
import { parseConfiguration } from "../../apps/nous-core/src/config.js";
import { ModelRuntime } from "../../apps/nous-core/src/model/runtime.js";
import {
  episodePartitionSchema,
  journalSynthesisSchema,
  partitionIndices,
  journalSupportKeys,
} from "../../apps/nous-core/src/model/schemas/longitudinal.js";
import { consolidationSchema } from "../../apps/nous-core/src/model/schemas/consolidation.js";
import { consolidationRequest } from "../../apps/nous-core/src/maintenance/consolidation.js";

const { values } = parseArgs({
  options: {
    config: { type: "string" },
    input: { type: "string" },
    output: { type: "string" },
    role: { type: "string" },
    "prompt-root": { type: "string", default: "prompts" },
    "override-prompt-root": { type: "string" },
    help: { type: "boolean", default: false },
  },
});
if (values.help) {
  console.log(
    "pnpm research:longitudinal --config <nous.toml> --input <MaintenancePlan.json> --role episode_segmentation|journal_synthesis|memory_consolidation --output <local-result.json> [--prompt-root prompts] [--override-prompt-root directory]",
  );
} else {
  await run();
}
async function run() {
  const role = values.role;
  if (
    !values.config ||
    !values.input ||
    !values.output ||
    ![
      "episode_segmentation",
      "journal_synthesis",
      "memory_consolidation",
    ].includes(role ?? "")
  )
    throw new Error(
      "config, input, supported role and output are required; see --help",
    );
  const bytes = await readFile(resolve(values.input));
  if (bytes.length > 262144)
    throw new Error("MaintenancePlan input exceeds 256 KiB");
  const plan = fromJson(
    MaintenancePlanSchema,
    JSON.parse(bytes.toString("utf8")) as JsonValue,
  );
  if (
    plan.status !== "ready" ||
    !plan.subjectId ||
    plan.members.length > 2048 ||
    plan.supports.length > 512 ||
    plan.candidates.length > 32
  )
    throw new Error(
      "Input must be a ready bounded MaintenancePlan with an exact Subject/source catalog",
    );
  const { models: configuration } = parseConfiguration(
    await readFile(resolve(values.config), "utf8"),
    true,
  );
  const models = await ModelRuntime.fromConfig(
    configuration,
    resolve(values["prompt-root"]!),
    values["override-prompt-root"]
      ? resolve(values["override-prompt-root"])
      : undefined,
  );
  const signal = new AbortController();
  process.once("SIGINT", () => signal.abort(new Error("Research interrupted")));
  const input = JSON.stringify(toJson(MaintenancePlanSchema, plan));
  const snapshot = models.invocations.snapshot(
    role as
      "episode_segmentation" | "journal_synthesis" | "memory_consolidation",
  );
  let result;
  if (role === "episode_segmentation") {
    result = await models.segmentEpisode(input, signal.signal, snapshot);
    partitionIndices(
      episodePartitionSchema.parse(result.value),
      plan.members.map((member) => member.key),
    );
  } else if (role === "journal_synthesis") {
    result = await models.synthesizeJournal(input, signal.signal, snapshot);
    journalSupportKeys(
      journalSynthesisSchema.parse(result.value),
      new Set(plan.supports.map((support) => support.key)),
    );
  } else {
    result = await models.consolidate(input, signal.signal, snapshot);
    // Check supplied catalog keys using the production mapper; commit remains an owner operation.
    consolidationRequest(
      plan,
      consolidationSchema.parse(result.value),
      randomUUID(),
      create(ProducerSignatureSchema),
    );
  }
  await mkdir(dirname(resolve(values.output)), { recursive: true });
  await writeFile(
    resolve(values.output),
    JSON.stringify(
      {
        role,
        subjectId: plan.subjectId,
        sourcePlan: toJson(MaintenancePlanSchema, plan),
        proposal: result.value,
        producer: result.producerMetadata,
        review: {
          status: "human_review_required",
          checks:
            role === "episode_segmentation"
              ? [
                  "human boundary annotations",
                  "over/under segmentation",
                  "boundary displacement",
                ]
              : role === "journal_synthesis"
                ? [
                    "support per point",
                    "unsupported points",
                    "salient fact omissions",
                    "temporal scope",
                  ]
                : [
                    "create/revise identity",
                    "unsupported claims",
                    "contradiction handling",
                    "Schema generalization",
                    "later recall usefulness",
                  ],
        },
      },
      null,
      2,
    ) + "\n",
    { flag: "wx", mode: 0o600 },
  );
  console.log(`Proposal saved for human review: ${resolve(values.output)}`);
}
