//! Memory Authority orchestration for the R1 reference profile.

mod accessibility;
mod accretion;
mod batch;
mod consolidation_context;
mod dependencies;
mod episode;
mod journal;
mod lane;
mod longitudinal_policy;
mod longitudinal_query;
mod maintenance_planning;
mod memory;
mod provenance;
mod query;
mod query_materialization;
mod query_support;
mod runtime;
pub mod schema;
mod schema_lane;
mod source_classes;
mod state;
mod tag;
mod topology;
mod topology_maintenance;

use crate::*;
use chrono::{DateTime, Utc};
use nous_core::*;
use nous_object_store::ObjectStore;
use nous_persistence::{
    AuthorityStore, MutationEnvelope, MutationStart, OwnerLock, ProjectionInvalidation,
    database_error as db,
};
pub use nous_runtime::{ResidentView, ResourceUpsert, ResourceView, SessionView};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::Row;
use state::{fence_epoch, require_transition};
use std::sync::Arc;
use uuid::Uuid;

pub use crate::{CognitiveRole, FormationMode};
pub use accessibility::{
    AccessibilityPolicy, eligible as accessibility_eligible, register_configuration,
    resolve_accessibility_policy,
};
pub use accretion::{ACCRETION, AccretionPolicy, AccretionSignal};
pub use episode::*;
pub use journal::*;
pub use longitudinal_policy::{
    CONSOLIDATION_CONTEXT, CONSOLIDATION_MAX_ACTIONS, ConsolidationContextPolicy, EPISODE_SYNOPSIS,
};
pub use maintenance_planning::MaintenanceScope;
pub use nous_core::TopologyRelation;
pub use nous_core::{CognitiveQuery, CognitiveQueryResult};
pub use tag::*;
pub use topology_maintenance::*;

#[derive(Clone)]
pub struct MemoryService {
    pub configuration: nous_configuration::ConfigurationService,
    pub store: AuthorityStore,
    pub objects: ObjectStore,
    pub cognition: nous_runtime::CognitiveRuntimeService,
    capabilities: Arc<Vec<CapabilityDescriptor>>,
}

impl MemoryService {
    async fn start_mutation(
        &self,
        subject: SubjectId,
        operation: OperationId,
        kind: &str,
        digest: &str,
    ) -> Result<MutationStart<'_>> {
        MutationEnvelope::begin(
            &self.store,
            subject,
            operation,
            kind,
            digest,
            Some(OwnerLock("memory-authority")),
        )
        .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub api_version: u32,
    pub ready: bool,
    pub authority: String,
    pub capabilities: Vec<CapabilityStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryView {
    pub object: MemoryObject,
    pub revision: MemoryRevision,
    pub supports: Vec<RevisionSupport>,
    pub aboutness: Vec<EntityRef>,
    pub tags: Vec<TagId>,
    pub relations: Vec<MemoryRevisionRelation>,
    pub relations_truncated: bool,
    pub accessibility_level: AccessibilityLevel,
    pub temporal_evidence: TemporalEvidence,
    #[serde(default)]
    pub source_classes: Vec<SourceClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseMemoryInput {
    #[serde(default)]
    pub producer: Option<ProducerSignature>,
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub memory_id: MemoryId,
    pub expected_object_epoch: i64,
    pub intent: RevisionIntent,
    pub formation_mode: FormationMode,
    pub grounding_occurrence_id: Option<OccurrenceId>,
    pub semantic_role: String,
    pub representation_text: String,
    pub title: Option<String>,
    pub supports: Vec<RevisionSupport>,
    pub aboutness: Vec<EntityRef>,
    pub valid_time: TemporalExtent,

    pub epistemic_class: EpistemicClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub operation_id: OperationId,
    pub label: String,
    pub description: Option<String>,
    pub kind_hint: Option<String>,
    #[serde(default = "default_explicit_origin")]
    pub origin: String,
}

fn default_explicit_origin() -> String {
    "explicit".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssociationRequest {
    pub operation_id: OperationId,
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub relation_kind: String,
    pub polarity: AssociationPolarity,
    pub support_class: AssociationSupportClass,
    pub supports: Vec<AssociationSupport>,
    pub producer_signature_id: Option<Uuid>,
    pub valid_time: TemporalExtent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebindEntityRequest {
    pub mention_id: Uuid,
    pub entity_ref: Option<EntityRef>,
    pub binding_state: String,
    pub host_resolution_ref: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub memory: Option<MemoryView>,
    pub topology_changes: usize,
}

fn parse_enum<T: DeserializeOwned>(value: String, name: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| Error::Infrastructure(format!("invalid {name}: {error}")))
}

fn temporal_from_columns(
    kind: String,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<TemporalExtent> {
    match kind.as_str() {
        "unknown" => Ok(TemporalExtent::Unknown),
        "instant" => Ok(TemporalExtent::Instant {
            at: start.ok_or_else(|| Error::Infrastructure("instant has no timestamp".into()))?,
        }),
        "interval" => Ok(TemporalExtent::Interval { start, end }),
        _ => Err(Error::Infrastructure("invalid temporal extent kind".into())),
    }
}

fn temporal_columns(
    value: &TemporalExtent,
) -> (&'static str, Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    match value {
        TemporalExtent::Unknown => ("unknown", None, None),
        TemporalExtent::Instant { at } => ("instant", Some(*at), None),
        TemporalExtent::Interval { start, end } => ("interval", *start, *end),
    }
}

fn operation_digest<T: Serialize>(kind: &str, subject: SubjectId, value: &T) -> Result<String> {
    canonical_request_digest(kind, subject, value)
}

impl MemoryService {
    async fn formation_time_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        operation: OperationId,
        started_at: DateTime<Utc>,
    ) -> Result<DateTime<Utc>> {
        let timestamp: Option<Option<String>> = sqlx::query_scalar(
            "SELECT snapshot->>'cognitive_formed_at' FROM model_workflow_operations WHERE subject_id=$1 AND owner='memory' AND operation_key=$2",
        ).bind(subject.0).bind(operation.0.to_string()).fetch_optional(&mut **tx).await.map_err(db)?;
        timestamp
            .flatten()
            .map(|value| {
                value.parse().map_err(|_| {
                    Error::Infrastructure("invalid workflow cognitive formation time".into())
                })
            })
            .transpose()
            .map(|value| value.unwrap_or(started_at))
    }

    pub fn new(
        store: AuthorityStore,
        objects: ObjectStore,
        cognition: nous_runtime::CognitiveRuntimeService,
        configuration: nous_configuration::ConfigurationService,
    ) -> Self {
        Self {
            configuration,
            store,
            objects,
            cognition,
            capabilities: Arc::new(Vec::new()),
        }
    }

    pub fn with_capabilities(mut self, capabilities: Vec<CapabilityDescriptor>) -> Self {
        self.capabilities = Arc::new(capabilities);
        self
    }

    pub async fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            api_version: API_VERSION,
            ready: true,
            authority: "postgresql".into(),
            capabilities: self
                .capabilities
                .iter()
                .map(|capability| CapabilityStatus {
                    capability_id: capability.operation.as_str().into(),
                    status: match capability.readiness {
                        CapabilityReadiness::Ready => Readiness::Ready,
                        CapabilityReadiness::Degraded => Readiness::Degraded,
                        CapabilityReadiness::Unavailable => Readiness::Unavailable,
                    },
                    reason: None,
                })
                .collect(),
        }
    }
}
