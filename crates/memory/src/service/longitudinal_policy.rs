use nous_configuration::*;
use nous_core::{Error, Result};
pub const JOURNAL_MAX_EPISODES: ConfigKey<u64> = ConfigKey::new("journal.max_episode_count");
pub const JOURNAL_MAX_SPAN: ConfigKey<u64> = ConfigKey::new("journal.max_span");
pub const CONSOLIDATION_DELAY: ConfigKey<u64> = ConfigKey::new("consolidation.settle_delay");

pub const CONSOLIDATION_MAX_ACTIONS: ConfigKey<u64> = ConfigKey::new("consolidation.max_actions");

pub fn register_longitudinal_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        JOURNAL_MAX_EPISODES,
        "memory",
        "Maximum Episodes in a Journal synthesis scope.",
        12,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |value| {
            if (1..=64).contains(value) {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "Journal scope must contain 1..64 Episodes".into(),
                ))
            }
        },
    )?;
    registry.register(
        JOURNAL_MAX_SPAN,
        "memory",
        "Maximum Journal experience span in cognitive seconds.",
        86400,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |value| {
            if (1..=604800).contains(value) {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "Journal span must be 1..604800 seconds".into(),
                ))
            }
        },
    )?;
    registry.register(
        CONSOLIDATION_DELAY,
        "memory",
        "Settling delay before longitudinal consolidation in cognitive seconds.",
        300,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |value| {
            if (1..=86400).contains(value) {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "Consolidation delay must be 1..86400 seconds".into(),
                ))
            }
        },
    )?;
    registry.register(
        CONSOLIDATION_MAX_ACTIONS,
        "memory",
        "Maximum actions in one atomic longitudinal consolidation proposal.",
        8,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |value| {
            if (1..=16).contains(value) {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "Consolidation must contain 1..16 actions".into(),
                ))
            }
        },
    )?;
    Ok(())
}
