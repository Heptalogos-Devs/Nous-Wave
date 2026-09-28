//! Shared configuration registration, resolution, persistence, and snapshots.
//!
//! This crate owns configuration mechanics only. Domain meaning stays with the
//! registering owner, which resolves a snapshot into its typed policy.

mod key;
mod registry;
mod service;
mod snapshot;

pub use key::*;
pub use registry::{ConfigDescriptor, ConfigRegistry, ConfigRegistryBuilder, flatten_settings};
pub use service::{ConfigChangeOutcome, ConfigurationService};
pub use snapshot::{ConfigSnapshot, ResolvedConfigValue};

use nous_core::Result;

pub fn register_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    let bool_validator = |_: &bool| Ok(());
    registry.register(
        PROCESS_MEMORY,
        "configuration-service",
        "Whether the process provides Memory cognition.",
        true,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::RestartProcess,
        ConfigSemanticEffect::Operational,
        bool_validator,
    )?;
    registry.register(
        PROCESS_SELF,
        "configuration-service",
        "Whether the process provides Self cognition.",
        false,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::RestartProcess,
        ConfigSemanticEffect::Operational,
        bool_validator,
    )?;
    registry.register(
        PROCESS_SOCIAL,
        "configuration-service",
        "Whether the process provides Social cognition.",
        false,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::RestartProcess,
        ConfigSemanticEffect::Operational,
        bool_validator,
    )?;
    registry.register(
        SUBJECT_DEFAULT_MEMORY,
        "configuration-service",
        "Default Memory capability for newly created Subjects.",
        true,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::NewSubjectsOnly,
        ConfigSemanticEffect::SubjectProvisioning,
        bool_validator,
    )?;
    registry.register(
        SUBJECT_DEFAULT_SELF,
        "configuration-service",
        "Default Self capability for newly created Subjects.",
        false,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::NewSubjectsOnly,
        ConfigSemanticEffect::SubjectProvisioning,
        bool_validator,
    )?;
    registry.register(
        SUBJECT_DEFAULT_SOCIAL,
        "configuration-service",
        "Default Social capability for newly created Subjects.",
        false,
        ConfigExposure::Developer,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::NewSubjectsOnly,
        ConfigSemanticEffect::SubjectProvisioning,
        bool_validator,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TEST_KEY: ConfigKey<f64> = ConfigKey::new("test.policy.threshold");

    #[test]
    fn duplicate_keys_and_invalid_defaults_are_rejected() {
        let mut builder = ConfigRegistryBuilder::new();
        builder
            .register(
                TEST_KEY,
                "test",
                "threshold",
                1.0,
                ConfigExposure::Advanced,
                ConfigScopePolicy::SubjectOverrideAllowed,
                ConfigApplyMode::Live,
                ConfigSemanticEffect::QueryPolicy,
                |value| {
                    if *value >= 0.0 {
                        Ok(())
                    } else {
                        Err(nous_core::Error::Invalid("negative".into()))
                    }
                },
            )
            .unwrap();
        assert!(
            builder
                .register(
                    TEST_KEY,
                    "test",
                    "threshold",
                    1.0,
                    ConfigExposure::Advanced,
                    ConfigScopePolicy::SubjectOverrideAllowed,
                    ConfigApplyMode::Live,
                    ConfigSemanticEffect::QueryPolicy,
                    |_| Ok(()),
                )
                .is_err()
        );
    }

    #[test]
    fn settings_are_flattened_and_snapshot_digest_is_subset_scoped() {
        let mut builder = ConfigRegistryBuilder::new();
        builder
            .register(
                TEST_KEY,
                "test",
                "threshold",
                1.0,
                ConfigExposure::Advanced,
                ConfigScopePolicy::SubjectOverrideAllowed,
                ConfigApplyMode::Live,
                ConfigSemanticEffect::QueryPolicy,
                |_| Ok(()),
            )
            .unwrap();
        let registry = builder.finish().unwrap();
        let values = std::collections::BTreeMap::from([(
            TEST_KEY.path().to_owned(),
            ResolvedConfigValue {
                json: json!(1.0),
                source: ConfigSource::ReferenceDefault,
            },
        )]);
        let snapshot = ConfigSnapshot::new(None, 1, registry, values).unwrap();
        let first = snapshot.digest_for(&[TEST_KEY.path()]).unwrap();
        assert_eq!(snapshot.get(TEST_KEY).unwrap(), 1.0);
        assert_eq!(first, snapshot.digest_for(&[TEST_KEY.path()]).unwrap());
        assert_eq!(
            flatten_settings(&json!({"settings":{"test":{"policy":{"threshold":2.0}}}})).unwrap()
                [TEST_KEY.path()],
            json!(2.0)
        );
    }
}
