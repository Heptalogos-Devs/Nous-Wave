import { blake3 } from "@noble/hashes/blake3.js";
import type { ModelConfiguration } from "./configuration.js";
import { canonicalDigest } from "../digest.js";

function digest(value: unknown) {
  return Buffer.from(
    blake3(new TextEncoder().encode(JSON.stringify(value))),
  ).toString("hex");
}
export function resolvedEmbedding(config: ModelConfiguration) {
  const binding = config.roles.query_embedding;
  if (!binding) return undefined;
  const profile = config.model_profiles[binding.model]!;
  if (!profile.model.trim() || !profile.embedding) return undefined;
  const gateway = config.gateway_profiles[profile.gateway]!;
  const profileDigest = canonicalDigest({
    profile,
    destination: gateway.base_url,
  });
  const configDigest = canonicalDigest({
    binding: {
      ...binding,
      max_output_tokens: 4096,
      timeout_ms: binding.timeout_ms ?? gateway.request_timeout_ms,
    },
    adapter: "ai-sdk@7.0.102/openai@4.0.67",
    profileDigest,
  });
  const space = {
    space_hash: "",
    model_identity: profile.model,
    weights_revision: profile.embedding.weights_revision,
    task: profile.embedding.task,
    input_representation: profile.embedding.input_representation,
    preprocessing_identity: profile.embedding.preprocessing_identity,
    preprocessing_revision: profile.embedding.preprocessing_revision,
    dimension: profile.embedding.dimension,
    normalization: profile.embedding.normalization,
    output_semantics: profile.embedding.output_semantics,
  };
  space.space_hash = digest(space);
  const producer = {
    signature_hash: "",
    provider_class: profile.protocol,
    operation: "text_embedding",
    implementation: "ai-sdk@7.0.102/openai@4.0.67",
    model_identity: profile.model,
    model_revision: profile.model_revision ?? null,
    output_schema_digest: null,
    preprocessing_identity: profile.embedding.preprocessing_identity,
    preprocessing_revision: profile.embedding.preprocessing_revision,
    config_digest: configDigest,
  };
  producer.signature_hash = digest(producer);
  return {
    space,
    producer: {
      ...producer,
      model_revision: profile.model_revision,
      output_schema_digest: undefined,
    },
  };
}
