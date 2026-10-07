// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import type { CliEnvironment } from "./runtime.js";

export async function deriveCommands(
  env: CliEnvironment,
  action: string | undefined,
) {
  const { client, values, subjectId, required } = env;
  return client.model.deriveMaterial({
    subjectId,
    sourceRegionId: required(action, "Source region ID"),
    strategy: values.strategy,
    target: values.target,
    supersedes: values.supersedes,
  });
}
