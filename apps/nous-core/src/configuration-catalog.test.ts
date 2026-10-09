// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import { expect, it } from "vitest";
import {
  CONFIG_REVISION,
  configurationBundle,
} from "./configuration-catalog.js";
import { modelConfigurationShape } from "./model/configuration.js";

it("freezes owner-normalized policies before the Kernel computes configuration identity", () => {
  const omitted = configurationBundle({
    config_revision: CONFIG_REVISION,
    video: { max_frames: 4 },
  });
  const explicit = configurationBundle({
    config_revision: CONFIG_REVISION,
    video: { ...modelConfigurationShape.video.parse(undefined), max_frames: 4 },
  });
  expect(omitted.deployment_document).toEqual(explicit.deployment_document);
});
