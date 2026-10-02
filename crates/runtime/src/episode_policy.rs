//! Cheap typed boundary signals; Episode identity belongs to Memory.

use chrono::{DateTime, Duration, Utc};
use nous_core::{Result, SessionId};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceContext {
    pub session: SessionId,
    pub work_context: Option<Uuid>,
    pub conversation: Option<String>,
    pub actor: Option<String>,
    pub source_class: String,
}

#[derive(Debug, Clone)]
pub struct EpisodePolicy {
    pub soft_idle: Duration,
    pub hard_idle: Duration,
}

impl EpisodePolicy {
    pub fn from_snapshot(snapshot: &nous_configuration::ConfigSnapshot) -> Result<Self> {
        Ok(Self {
            soft_idle: Duration::seconds(snapshot.get(SOFT_IDLE_KEY)? as i64),
            hard_idle: Duration::seconds(snapshot.get(HARD_IDLE_KEY)? as i64),
        })
    }

    pub fn boundary(
        &self,
        previous: &ExperienceContext,
        next: &ExperienceContext,
        previous_at: DateTime<Utc>,
        next_at: DateTime<Utc>,
        work_context_ended: bool,
    ) -> Option<&'static str> {
        if work_context_ended {
            return Some("work_context_ended");
        }
        let gap = next_at - previous_at;
        if gap >= self.hard_idle {
            return Some("hard_idle");
        }
        let switches = usize::from(previous.work_context != next.work_context)
            + usize::from(previous.conversation != next.conversation)
            + usize::from(previous.actor != next.actor)
            + usize::from(previous.source_class != next.source_class);
        if switches >= 2 {
            Some("context_switch")
        } else if gap >= self.soft_idle && (switches > 0 || previous.session != next.session) {
            Some("soft_idle_context_switch")
        } else {
            None
        }
    }
}

pub const SOFT_IDLE_KEY: nous_configuration::ConfigKey<u64> =
    nous_configuration::ConfigKey::new("episode.soft_idle");
pub const HARD_IDLE_KEY: nous_configuration::ConfigKey<u64> =
    nous_configuration::ConfigKey::new("episode.hard_idle");
pub const SETTLE_DELAY_KEY: nous_configuration::ConfigKey<u64> =
    nous_configuration::ConfigKey::new("episode.settle_delay");

pub fn register_episode_configuration(
    registry: &mut nous_configuration::ConfigRegistryBuilder,
) -> Result<()> {
    use nous_configuration::*;
    for (key, default, description) in [
        (
            SOFT_IDLE_KEY,
            300,
            "Soft Episode idle threshold in cognitive seconds.",
        ),
        (
            HARD_IDLE_KEY,
            1800,
            "Hard Episode idle closure in cognitive seconds.",
        ),
        (
            SETTLE_DELAY_KEY,
            300,
            "Cognitive settling delay before semantic Episode review.",
        ),
    ] {
        registry.register(
            key,
            "cognitive-runtime",
            description,
            default,
            ConfigExposure::Advanced,
            ConfigScopePolicy::SubjectOverrideAllowed,
            ConfigApplyMode::Live,
            ConfigSemanticEffect::AuthorityFormation,
            |value| {
                if (1..=86400).contains(value) {
                    Ok(())
                } else {
                    Err(nous_core::Error::Invalid(
                        "Episode interval must be 1..86400 seconds".into(),
                    ))
                }
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuity_and_idle_context_boundaries() {
        let policy = EpisodePolicy {
            soft_idle: Duration::minutes(5),
            hard_idle: Duration::minutes(30),
        };
        let at = DateTime::<Utc>::UNIX_EPOCH;
        let previous = ExperienceContext {
            session: SessionId::new(),
            work_context: None,
            conversation: Some("chat:a".into()),
            actor: None,
            source_class: "message".into(),
        };
        let mut next = previous.clone();
        next.session = SessionId::new();
        assert_eq!(
            policy.boundary(&previous, &next, at, at + Duration::minutes(1), false),
            None
        );
        assert_eq!(
            policy.boundary(&previous, &next, at, at + Duration::minutes(5), false),
            Some("soft_idle_context_switch")
        );
        next = previous.clone();
        assert_eq!(
            policy.boundary(&previous, &next, at, at + Duration::minutes(30), false),
            Some("hard_idle")
        );
        next.actor = Some("person:b".into());
        next.conversation = Some("chat:b".into());
        assert_eq!(
            policy.boundary(&previous, &next, at, at, false),
            Some("context_switch")
        );
        assert_eq!(
            policy.boundary(&previous, &previous, at, at, true),
            Some("work_context_ended")
        );
    }
}
