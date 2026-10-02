use nous_configuration::*;
use nous_core::{Error, Result};
pub const MAINTENANCE_ENABLED: ConfigKey<bool> = ConfigKey::new("maintenance.enabled");
pub const POLL_INTERVAL: ConfigKey<u64> = ConfigKey::new("maintenance.poll_interval");
pub const MAX_OPERATIONS: ConfigKey<u64> = ConfigKey::new("maintenance.max_operations_per_grant");
pub const WORKER_LEASE: ConfigKey<u64> = ConfigKey::new("maintenance.worker_lease_seconds");

pub fn register_maintenance_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        MAINTENANCE_ENABLED,
        "cognitive-runtime",
        "Enable host-granted maintenance opportunities.",
        true,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::Operational,
        |_| Ok(()),
    )?;
    registry.register(
        POLL_INTERVAL,
        "cognitive-runtime",
        "Standalone maintenance opportunity interval in infrastructure seconds.",
        30,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::Operational,
        interval,
    )?;
    registry.register(
        WORKER_LEASE,
        "cognitive-runtime",
        "Worker lease expiry in infrastructure seconds.",
        120,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::Operational,
        interval,
    )?;
    registry.register(
        MAX_OPERATIONS,
        "cognitive-runtime",
        "Maximum operations in one host-granted opportunity.",
        4,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::Operational,
        |value| {
            if (1..=32).contains(value) {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "Maintenance grant must contain 1..32 operations".into(),
                ))
            }
        },
    )?;
    Ok(())
}
fn interval(value: &u64) -> Result<()> {
    if (1..=3600).contains(value) {
        Ok(())
    } else {
        Err(Error::Invalid(
            "Maintenance interval must be 1..3600 seconds".into(),
        ))
    }
}
