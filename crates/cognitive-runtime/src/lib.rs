//! Model-independent Session continuity, consumer context and query orchestration.

mod checkpoints;
mod query;
pub use checkpoints::*;
mod resources;
mod sessions;
mod types;
mod use_feedback;
mod working_set;
pub use query::{
    BoundQuery, CognitiveContributor, CognitiveContributors, LaneCandidate, LaneOutput, LaneStatus,
    QueryPlan, WorkCycle,
};
pub use working_set::*;

pub use resources::*;
pub use types::*;

use chrono::{DateTime, Utc};
use nous_authority_store::AuthorityStore;
use nous_core::*;
use std::collections::HashSet;

pub const RESIDENT_LIMIT_KEY: nous_configuration_service::ConfigKey<usize> =
    nous_configuration_service::ConfigKey::new("runtime.resident_limit");

pub fn register_configuration(
    registry: &mut nous_configuration_service::ConfigRegistryBuilder,
) -> Result<()> {
    registry.register(
        RESIDENT_LIMIT_KEY,
        "cognitive-runtime",
        "Maximum resident references per session.",
        256,
        nous_configuration_service::ConfigExposure::Developer,
        nous_configuration_service::ConfigScopePolicy::SystemOnly,
        nous_configuration_service::ConfigApplyMode::RestartProcess,
        nous_configuration_service::ConfigSemanticEffect::Operational,
        |value| {
            if *value > 0 {
                Ok(())
            } else {
                Err(Error::Invalid("resident_limit must be positive".into()))
            }
        },
    )
}

#[derive(Clone)]
pub struct CognitiveRuntimeService {
    pub store: AuthorityStore,
    pub resident_limit: usize,
    pub configuration: nous_configuration_service::ConfigurationService,
}

impl CognitiveRuntimeService {
    pub fn new(
        store: AuthorityStore,
        resident_limit: usize,
        configuration: nous_configuration_service::ConfigurationService,
    ) -> Result<Self> {
        if resident_limit == 0 {
            return Err(Error::Invalid("resident_limit must be positive".into()));
        }
        Ok(Self {
            store,
            resident_limit,
            configuration,
        })
    }

    pub async fn require_subject(&self, subject: SubjectId) -> Result<()> {
        self.store.require_subject(subject).await
    }

    pub async fn reference_in_subject(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<bool> {
        self.store.reference_in_subject(subject, reference).await
    }
}
