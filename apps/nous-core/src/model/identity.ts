// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { canonicalDigest } from "../digest.js";
import type { ModelProfile, ExecutionProfile } from "./configuration.js";
import type { PromptAsset } from "./prompts.js";

export function modelRoleIdentity(
  profile: ModelProfile,
  gateway: { base_url: string; request_timeout_ms: number },
  binding: ExecutionProfile,
  prompt?: PromptAsset,
) {
  const profileDigest = canonicalDigest({
    profile,
    destination: gateway.base_url,
  });
  const configDigest = canonicalDigest({
    binding: {
      ...binding,
      max_output_tokens: binding.max_output_tokens,
      timeout_ms: binding.timeout_ms ?? gateway.request_timeout_ms,
    },
    adapter: "ai-sdk@7.0.102/openai@4.0.67",
    profileDigest,
    prompt: prompt ? { id: prompt.id, digest: prompt.digest } : undefined,
  });
  return { profileDigest, configDigest };
}

export function invocationConfigDigest(
  binding: string,
  prompt?: PromptAsset,
  adapter?: string,
) {
  let result = prompt
    ? canonicalDigest({
        binding,
        promptId: prompt.id,
        promptDigest: prompt.digest,
      })
    : binding;
  if (adapter) result = canonicalDigest({ binding: result, adapter });
  return result;
}
