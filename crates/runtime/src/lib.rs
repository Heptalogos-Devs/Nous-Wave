//! Model-independent Session continuity, consumer context and query orchestration.

mod query;
mod resources;
mod sessions;
mod types;
mod use_feedback;
mod work_contexts;
mod working_set;
pub use query::{
    BoundQuery, CognitiveContributor, CognitiveContributors, LaneCandidate, LaneOutput, LaneStatus,
    QueryPlan, SharedLaneProvider, WorkCycle, register_retrieval_configuration,
};
pub use work_contexts::*;
pub use working_set::*;

pub use resources::*;
pub use types::*;

use chrono::{DateTime, Utc};
use nous_core::*;
use nous_persistence::AuthorityStore;
use std::collections::HashSet;

pub const RESIDENT_LIMIT_KEY: nous_configuration::ConfigKey<usize> =
    nous_configuration::ConfigKey::new("runtime.resident_limit");

pub fn register_configuration(
    registry: &mut nous_configuration::ConfigRegistryBuilder,
) -> Result<()> {
    registry.register(
        RESIDENT_LIMIT_KEY,
        "cognitive-runtime",
        "Maximum resident references per session.",
        256,
        nous_configuration::ConfigExposure::Developer,
        nous_configuration::ConfigScopePolicy::SystemOnly,
        nous_configuration::ConfigApplyMode::RestartProcess,
        nous_configuration::ConfigSemanticEffect::Operational,
        |value| {
            if *value > 0 {
                Ok(())
            } else {
                Err(Error::Invalid("resident_limit must be positive".into()))
            }
        },
    )?;
    register_retrieval_configuration(registry)
}

#[derive(Clone)]
pub struct CognitiveRuntimeService {
    pub store: AuthorityStore,
    pub resident_limit: usize,
    pub configuration: nous_configuration::ConfigurationService,
}

impl CognitiveRuntimeService {
    pub fn new(
        store: AuthorityStore,
        resident_limit: usize,
        configuration: nous_configuration::ConfigurationService,
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
