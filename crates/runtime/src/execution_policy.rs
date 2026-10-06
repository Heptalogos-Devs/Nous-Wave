//! Infrastructure execution budgets owned by Cognitive Runtime.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use nous_configuration::*;
use nous_core::{Error, Result};
pub const MODEL_WORKFLOW_LEASE: ConfigKey<u64> = ConfigKey::new("model_workflow.lease_seconds");
pub const QUERY_LEASE: ConfigKey<u64> = ConfigKey::new("runtime.query_lease_seconds");
pub const QUERY_SLOTS: ConfigKey<usize> = ConfigKey::new("runtime.query_lease_slots");
pub fn register_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    for (key, default) in [(MODEL_WORKFLOW_LEASE, 360), (QUERY_LEASE, 360)] {
        registry.register(
            key,
            "cognitive-runtime",
            "Execution lease duration.",
            default,
            ConfigExposure::Developer,
            ConfigScopePolicy::SystemOnly,
            ConfigApplyMode::Live,
            ConfigSemanticEffect::Operational,
            |value| {
                if (1..=3600).contains(value) {
                    Ok(())
                } else {
                    Err(Error::Invalid(
                        "execution lease must be 1..3600 seconds".into(),
                    ))
                }
            },
        )?;
        registry.describe(key.path(), |d| {
            d.unit = Some("seconds".into());
            d.json_schema["minimum"] = serde_json::json!(1);
            d.json_schema["maximum"] = serde_json::json!(3600);
        })?;
    }
    registry.register(
        QUERY_SLOTS,
        "cognitive-runtime",
        "Concurrent retained query slots.",
        16,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::Operational,
        |value| {
            if (1..=64).contains(value) {
                Ok(())
            } else {
                Err(Error::Invalid("query lease slots must be 1..64".into()))
            }
        },
    )?;
    registry.describe(QUERY_SLOTS.path(), |d| {
        d.unit = Some("items".into());
        d.json_schema["minimum"] = serde_json::json!(1);
        d.json_schema["maximum"] = serde_json::json!(64);
    })
}
