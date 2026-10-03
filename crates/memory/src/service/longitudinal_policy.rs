use nous_configuration::*;
use nous_core::{Error, Result};
pub const JOURNAL_MAX_EPISODES: ConfigKey<u64> = ConfigKey::new("journal.max_episode_count");
pub const JOURNAL_MAX_SPAN: ConfigKey<u64> = ConfigKey::new("journal.max_span_seconds");
pub const CONSOLIDATION_DELAY: ConfigKey<u64> =
    ConfigKey::new("consolidation.settle_delay_seconds");

pub const CONSOLIDATION_MAX_ACTIONS: ConfigKey<u64> = ConfigKey::new("consolidation.max_actions");

pub fn register_longitudinal_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    let reference = ReferenceProfile::parse(include_str!(
        "../../../../config/reference/longitudinal-v1.json"
    ))?;
    registry.register(
        JOURNAL_MAX_EPISODES,
        "memory",
        "Maximum Episodes in a Journal synthesis scope.",
        reference.get(JOURNAL_MAX_EPISODES)?,
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
        reference.get(JOURNAL_MAX_SPAN)?,
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
        reference.get(CONSOLIDATION_DELAY)?,
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
        reference.get(CONSOLIDATION_MAX_ACTIONS)?,
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
    registry.bounds(JOURNAL_MAX_EPISODES, 1, 64, Some("items"))?;
    registry.bounds(JOURNAL_MAX_SPAN, 1, 604800, Some("cognitive_seconds"))?;
    registry.bounds(CONSOLIDATION_DELAY, 1, 86400, Some("cognitive_seconds"))?;
    registry.bounds(CONSOLIDATION_MAX_ACTIONS, 1, 16, Some("items"))?;
    reference.tag(registry, JOURNAL_MAX_EPISODES.path())?;
    reference.tag(registry, JOURNAL_MAX_SPAN.path())?;
    reference.tag(registry, CONSOLIDATION_DELAY.path())?;
    reference.tag(registry, CONSOLIDATION_MAX_ACTIONS.path())?;
    Ok(())
}
