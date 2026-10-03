import { parseArgs } from "node:util";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { relative, resolve, sep } from "node:path";
import {
  parseConfiguration,
  parseEffectiveConfiguration,
} from "../../apps/nous-core/src/config.js";
import {
  roleNames,
  resolveRoleBinding,
  type ModelRole,
} from "../../apps/nous-core/src/model/configuration.js";
import { PromptRegistry } from "../../apps/nous-core/src/model/prompts.js";
import { providerContractForRole } from "../../apps/nous-core/src/model/schemas/contracts.js";
import { modelRoleIdentity } from "../../apps/nous-core/src/model/identity.js";

const { values } = parseArgs({
  options: {
    all: { type: "boolean" },
    role: { type: "string" },
    config: { type: "string" },
    output: { type: "string" },
    "prompt-root": { type: "string", default: "prompts" },
    "override-prompt-root": { type: "string" },
  },
});
if (values.role && !roleNames.includes(values.role as ModelRole))
  throw new Error("Unknown model role");
if (!values.all && !values.role)
  throw new Error("Select --all or --role <role>");
let configuration;
if (values.config) {
  const { document } = parseConfiguration(
    await readFile(resolve(values.config), "utf8"),
    true,
  );
  configuration = parseEffectiveConfiguration({
    ...document,
    "material.strategy": (
      document.material as Record<string, unknown> | undefined
    )?.strategy,
  }).models;
}
const prompts = new PromptRegistry(
  resolve(values["prompt-root"]!),
  values["override-prompt-root"]
    ? resolve(values["override-prompt-root"])
    : undefined,
);
const output = resolve(
  values.output ?? "data/research/inspection/model-contracts",
);
const rel = relative(resolve("data/research"), output);
if (rel === ".." || rel.startsWith(`..${sep}`) || rel.startsWith(sep))
  throw new Error("Inspection output must be under ignored data/research");
for (const role of values.role ? [values.role as ModelRole] : roleNames) {
  const contract = providerContractForRole(role);
  const configured = configuration?.roles[role];
  const profile = configured
    ? configuration?.model_profiles[configured.model]
    : undefined;
  const gateway = profile
    ? configuration?.gateway_profiles[profile.gateway]
    : undefined;
  const binding = configured
    ? resolveRoleBinding(configured, profile?.protocol ?? "")
    : undefined;
  const prompt = await prompts.load(role, binding?.prompt);
  const identity =
    profile && gateway && binding
      ? modelRoleIdentity(profile, gateway, binding, prompt)
      : undefined;
  const metadata = {
    role,
    contract_id: contract?.id ?? null,
    provider_name: contract?.providerName ?? null,
    schema_digest: contract?.digest ?? null,
    prompt_id: prompt?.id ?? null,
    prompt_digest: prompt?.digest ?? null,
    ...(profile
      ? {
          model_profile: configured!.model,
          model: profile.model,
          protocol: profile.protocol,
          model_revision: profile.model_revision ?? null,
        }
      : {}),
    ...(identity
      ? {
          profile_digest: identity.profileDigest,
          config_digest: identity.configDigest,
        }
      : {}),
  };
  const root = values.role ? output : resolve(output, role);
  await mkdir(root, { recursive: true, mode: 0o700 });
  await writeFile(
    resolve(root, "contract.json"),
    JSON.stringify(metadata, null, 2) + "\n",
    { mode: 0o600 },
  );
  if (contract)
    await writeFile(
      resolve(root, "schema.json"),
      JSON.stringify(contract.providerSchema, null, 2) + "\n",
      { mode: 0o600 },
    );
  if (prompt)
    await writeFile(resolve(root, "prompt.md"), prompt.text, { mode: 0o600 });
  console.log(JSON.stringify({ ...metadata, output: root }));
}
