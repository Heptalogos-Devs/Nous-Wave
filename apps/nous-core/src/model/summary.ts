import type { ModelInvocationEvidence } from "./invocations.js";

export function invocationSummary(evidence: ModelInvocationEvidence) {
  return {
    role: evidence.role,
    protocol: evidence.protocol,
    model: evidence.model,
    profileDigest: evidence.profileDigest,
    promptDigest: evidence.promptDigest,
    latencyMs: evidence.latencyMs,
    requestCount: evidence.requestCount,
    status: "PASS",
    inputUsage:
      evidence.usage?.inputTokens === undefined
        ? undefined
        : BigInt(evidence.usage.inputTokens),
    outputUsage:
      evidence.usage?.outputTokens === undefined
        ? undefined
        : BigInt(evidence.usage.outputTokens),
    totalUsage:
      evidence.usage?.totalTokens === undefined
        ? undefined
        : BigInt(evidence.usage.totalTokens),
  };
}
