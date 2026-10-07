// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import {
  ConfigExposure,
  ConfigurationView,
  configurationValue,
} from "@nous-wave/client";
import type { CliEnvironment } from "./runtime.js";

export async function configCommands(
  env: CliEnvironment,
  action: string | undefined,
  argument?: string,
  positionals: string[] = [],
) {
  const { client, values, required } = env;
  const exposureCeiling = values.developer
    ? ConfigExposure.DEVELOPER
    : values.advanced
      ? ConfigExposure.ADVANCED
      : ConfigExposure.STANDARD;
  if (action === "list") return client.configuration.list({ exposureCeiling });
  const path = required(argument, "Configuration path");
  if (action === "describe") return client.configuration.describe(path);
  if (action === "get")
    return client.configuration.get({
      paths: [path],
      subjectId: values.subject,
      view: values.desired
        ? ConfigurationView.DESIRED
        : ConfigurationView.ACTIVE,
    });
  const operationId = values["operation-id"] ?? crypto.randomUUID();
  if (action === "set") {
    const value = configurationValue(required(positionals[3], "JSON value"));
    return values.subject
      ? client.configuration.setSubject({
          operationId,
          path,
          value,
          subjectId: values.subject,
        })
      : client.configuration.setSystem({ operationId, path, value });
  }
  if (action === "clear")
    return values.subject
      ? client.configuration.clearSubject({
          operationId,
          path,
          subjectId: values.subject,
        })
      : client.configuration.clearSystem({ operationId, path });
  throw new Error("Use config list|describe|get|set|clear");
}
