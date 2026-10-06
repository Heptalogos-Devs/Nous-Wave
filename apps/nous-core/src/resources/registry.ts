// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

import type { ResourceProfiles } from "./configuration.js";
import type { ExternalResourceAdapter, ResourceBinding } from "./adapter.js";
import { RagflowAdapter } from "./providers/ragflow.js";

export class ResourceRegistry {
  private readonly adapters = new Map<string, ExternalResourceAdapter>();
  constructor(profiles: ResourceProfiles) {
    for (const [name, profile] of Object.entries(profiles)) {
      const credential = process.env[profile.credential_env];
      if (profile.enabled && credential)
        this.adapters.set(
          `${profile.adapter_kind}:${name}`,
          new RagflowAdapter(name, profile, credential),
        );
    }
  }
  resolve(binding: ResourceBinding): ExternalResourceAdapter | undefined {
    return this.adapters.get(
      `${binding.adapterKind}:${binding.providerProfile}`,
    );
  }
}
