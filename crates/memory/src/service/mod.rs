//! Memory Authority orchestration for the R1 reference profile.

mod accessibility;
mod batch;
mod episode;
mod lane;
mod lifecycle;
mod provenance;
mod query;
mod query_materialization;
mod query_support;
mod runtime;
pub mod schema;
mod schema_lane;
mod source_classes;
mod topology;

use crate::*;
use chrono::{DateTime, Utc};
use nous_core::*;
use nous_object_store::ObjectStore;
use nous_persistence::{
    AuthorityStore, ProjectionInvalidation, check_receipt, commit_receipt, database_error as db,
    lock_operation,
};
pub use nous_runtime::{ResidentView, ResourceUpsert, ResourceView, SessionView};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

pub use crate::{CognitiveRole, FormationMode};
pub use accessibility::{
    AccessibilityPolicy, eligible as accessibility_eligible, register_configuration,
    resolve_accessibility_policy,
};
pub use episode::*;
pub use nous_core::{CognitiveQuery, CognitiveQueryResult};

#[derive(Clone)]
pub struct MemoryService {
    pub configuration: nous_configuration::ConfigurationService,
    pub store: AuthorityStore,
    pub objects: ObjectStore,
    pub cognition: nous_runtime::CognitiveRuntimeService,
    capabilities: Arc<Vec<CapabilityDescriptor>>,
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
    pub formed_at: DateTime<Utc>,
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

    pub async fn revision(
        &self,
        subject: SubjectId,
        revision: MemoryRevisionId,
    ) -> Result<MemoryView> {
        let memory: Uuid = sqlx::query_scalar(
            "SELECT memory_id FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2",
        )
        .bind(subject.0)
        .bind(revision.0)
        .fetch_one(self.store.pool())
        .await
        .map_err(db)?;
        self.memory(subject, MemoryId(memory), Some(revision)).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "memory read materializes one coherent Authority view"
    )]
    pub async fn memory(
        &self,
        subject: SubjectId,
        memory_id: MemoryId,
        revision: Option<MemoryRevisionId>,
    ) -> Result<MemoryView> {
        let row = sqlx::query("SELECT o.memory_id,o.subject_id,o.cognitive_role,o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state,o.accessibility_mode,o.created_at,r.memory_revision_id,r.revision_no,r.parent_revision_id,r.revision_intent,r.formation_mode,r.grounding_occurrence_id,r.semantic_role,r.title,r.representation_text,r.epistemic_class,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id FROM memory_objects o JOIN memory_revisions r ON r.memory_id=o.memory_id AND r.memory_revision_id=COALESCE($3,o.current_revision_id) WHERE o.subject_id=$1 AND o.memory_id=$2")
            .bind(subject.0)
            .bind(memory_id.0)
            .bind(revision.map(|value| value.0))
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("memory not found".into()))?;
        let revision_id = MemoryRevisionId(row.try_get("memory_revision_id").map_err(db)?);
        let object = MemoryObject {
            memory_id: MemoryId(row.try_get("memory_id").map_err(db)?),
            subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
            cognitive_role: parse_enum(
                row.try_get("cognitive_role").map_err(db)?,
                "cognitive role",
            )?,
            current_revision_id: MemoryRevisionId(row.try_get("current_revision_id").map_err(db)?),
            object_epoch: row.try_get("object_epoch").map_err(db)?,
            acceptance_state: parse_enum(
                row.try_get("acceptance_state").map_err(db)?,
                "acceptance state",
            )?,
            integrity_state: parse_enum(
                row.try_get("integrity_state").map_err(db)?,
                "integrity state",
            )?,
            suppression_state: parse_enum(
                row.try_get("suppression_state").map_err(db)?,
                "suppression state",
            )?,
            purge_state: parse_enum(row.try_get("purge_state").map_err(db)?, "purge state")?,
            accessibility_mode: parse_enum(
                row.try_get("accessibility_mode").map_err(db)?,
                "accessibility mode",
            )?,
            created_at: row.try_get("created_at").map_err(db)?,
        };
        let revision_value = MemoryRevision {
            producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
            memory_revision_id: revision_id,
            memory_id,
            subject_id: subject,
            revision_no: row.try_get("revision_no").map_err(db)?,
            parent_revision_id: row
                .try_get::<Option<Uuid>, _>("parent_revision_id")
                .map_err(db)?
                .map(MemoryRevisionId),
            revision_intent: row
                .try_get::<Option<String>, _>("revision_intent")
                .map_err(db)?
                .map(|value| parse_enum(value, "revision intent"))
                .transpose()?,
            formation_mode: parse_enum(
                row.try_get("formation_mode").map_err(db)?,
                "formation mode",
            )?,
            grounding_occurrence_id: row
                .try_get::<Option<Uuid>, _>("grounding_occurrence_id")
                .map_err(db)?
                .map(OccurrenceId),
            semantic_role: row.try_get("semantic_role").map_err(db)?,
            title: row.try_get("title").map_err(db)?,
            representation_text: row.try_get("representation_text").map_err(db)?,
            epistemic_class: parse_enum(
                row.try_get("epistemic_class").map_err(db)?,
                "epistemic class",
            )?,
            valid_time: temporal_from_columns(
                row.try_get("valid_time_kind").map_err(db)?,
                row.try_get("valid_time_start").map_err(db)?,
                row.try_get("valid_time_end").map_err(db)?,
            )?,
            formed_at: row.try_get("formed_at").map_err(db)?,
            recorded_at: row.try_get("recorded_at").map_err(db)?,
        };
        let supports = self.load_supports(revision_id).await?;
        let aboutness = sqlx::query_scalar::<_, String>("SELECT entity_ref FROM memory_revision_aboutness WHERE memory_revision_id=$1 ORDER BY entity_ref")
            .bind(revision_id.0).fetch_all(self.store.pool()).await.map_err(db)?.into_iter().map(EntityRef::new).collect::<Result<Vec<_>>>()?;
        let tags = sqlx::query_scalar::<_, Uuid>(
            "SELECT tag_id FROM memory_revision_tags WHERE memory_revision_id=$1 ORDER BY tag_id",
        )
        .bind(revision_id.0)
        .fetch_all(self.store.pool())
        .await
        .map_err(db)?
        .into_iter()
        .map(TagId)
        .collect();
        let relation_rows = sqlx::query("SELECT from_revision_id,to_revision_id,relation,created_at FROM memory_revision_relations WHERE from_revision_id=$1 OR to_revision_id=$1 ORDER BY created_at,from_revision_id,to_revision_id LIMIT 257")
            .bind(revision_id.0).fetch_all(self.store.pool()).await.map_err(db)?;
        let relations_truncated = relation_rows.len() > 256;
        let relations = relation_rows
            .into_iter()
            .take(256)
            .map(|row| {
                Ok(MemoryRevisionRelation {
                    from_revision_id: MemoryRevisionId(
                        row.try_get("from_revision_id").map_err(db)?,
                    ),
                    to_revision_id: MemoryRevisionId(row.try_get("to_revision_id").map_err(db)?),
                    relation: parse_enum(row.try_get("relation").map_err(db)?, "memory relation")?,
                    created_at: row.try_get("created_at").map_err(db)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let accessibility_level = self
            .accessibility_level(subject, memory_id, Utc::now())
            .await?;
        let temporal_evidence = self.temporal_evidence(revision_id).await?;
        let source_classes = self.source_classes(revision_id).await?;
        Ok(MemoryView {
            object,
            revision: revision_value,
            supports,
            aboutness,
            tags,
            relations,
            relations_truncated,
            accessibility_level,
            temporal_evidence,
            source_classes,
        })
    }

    async fn load_supports(&self, revision: MemoryRevisionId) -> Result<Vec<RevisionSupport>> {
        let mut supports = Vec::new();
        for row in sqlx::query("SELECT occurrence_id,source_region_id,derived_representation_id,derived_region_id,support_role FROM memory_revision_evidence WHERE memory_revision_id=$1 ORDER BY evidence_no")
            .bind(revision.0).fetch_all(self.store.pool()).await.map_err(db)? {
            let locator = match (
                row.try_get::<Option<Uuid>, _>("source_region_id").map_err(db)?,
                row.try_get::<Option<Uuid>, _>("derived_representation_id").map_err(db)?,
                row.try_get::<Option<Uuid>, _>("derived_region_id").map_err(db)?,
            ) {
                (Some(id), None, None) => EvidenceLocator::SourceRegion(nous_core::SourceRegionId(id)),
                (None, Some(id), None) => EvidenceLocator::DerivedRepresentation(nous_core::DerivedRepresentationId(id)),
                (None, None, Some(id)) => EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(id)),
                (None, None, None) => EvidenceLocator::WholeOccurrence,
                _ => return Err(Error::Infrastructure("evidence locator union is invalid".into())),
            };
            supports.push(RevisionSupport::Evidence(EvidenceRef { occurrence_id: OccurrenceId(row.try_get("occurrence_id").map_err(db)?), locator, support_role: parse_enum(row.try_get("support_role").map_err(db)?, "support role")? }));
        }
        for row in sqlx::query("SELECT target_ref_kind,target_ref,support_role FROM memory_revision_dependencies WHERE memory_revision_id=$1 ORDER BY target_ref_kind,target_ref")
            .bind(revision.0).fetch_all(self.store.pool()).await.map_err(db)? {
            supports.push(RevisionSupport::CognitionDependency(CognitionDependency { target_revision: parse_reference(&row.try_get::<String, _>("target_ref_kind").map_err(db)?, &row.try_get::<String, _>("target_ref").map_err(db)?)?, support_role: parse_enum(row.try_get("support_role").map_err(db)?, "support role")? }));
        }
        Ok(supports)
    }

    async fn temporal_evidence(&self, revision: MemoryRevisionId) -> Result<TemporalEvidence> {
        let rows = sqlx::query("SELECT o.occurred_time_kind,o.occurred_time_start,o.occurred_time_end,o.observed_at FROM memory_revision_evidence e JOIN observation_occurrences o USING(occurrence_id) WHERE e.memory_revision_id=$1 ORDER BY o.observed_at")
            .bind(revision.0).fetch_all(self.store.pool()).await.map_err(db)?;
        let mut occurred = Vec::new();
        let mut observed_at = None;
        for row in rows {
            occurred.push(temporal_from_columns(
                row.try_get("occurred_time_kind").map_err(db)?,
                row.try_get("occurred_time_start").map_err(db)?,
                row.try_get("occurred_time_end").map_err(db)?,
            )?);
            observed_at = Some(row.try_get("observed_at").map_err(db)?);
        }
        Ok(TemporalEvidence {
            occurred,
            observed_at,
        })
    }

    pub async fn memory_history(
        &self,
        subject: SubjectId,
        memory_id: MemoryId,
    ) -> Result<Vec<MemoryRevision>> {
        self.memory(subject, memory_id, None).await?;
        let ids = sqlx::query_scalar::<_, Uuid>("SELECT memory_revision_id FROM memory_revisions WHERE subject_id=$1 AND memory_id=$2 ORDER BY revision_no")
            .bind(subject.0).bind(memory_id.0).fetch_all(self.store.pool()).await.map_err(db)?;
        let mut result = Vec::new();
        for id in ids {
            result.push(
                self.memory(subject, memory_id, Some(MemoryRevisionId(id)))
                    .await?
                    .revision,
            );
        }
        Ok(result)
    }

    pub async fn form_memory(&self, mut input: ExplicitMemoryInput) -> Result<MemoryView> {
        input.producer = input
            .producer
            .as_ref()
            .map(AuthorityStore::canonical_producer)
            .transpose()?;
        input.validate()?;
        self.store.require_subject(input.subject).await?;
        self.validate_supports_for_subject(input.subject, &input.supports)
            .await?;
        self.validate_formation_semantics(input.subject, input.formation_mode, &input.supports)
            .await?;
        if input.formed_at > Utc::now() + chrono::Duration::minutes(5) {
            return Err(Error::Invalid("formed_at is too far in the future".into()));
        }
        let digest = operation_digest(
            "form_memory",
            input.subject,
            &serde_json::json!({"cognitive_role":input.cognitive_role,"formation_mode":input.formation_mode,"grounding_occurrence_id":input.grounding_occurrence_id,"semantic_role":input.semantic_role,"representation_text":input.representation_text,"title":input.title,"supports":input.supports,"aboutness":input.aboutness,"tags":input.tags,"valid_time":input.valid_time,"formed_at":input.formed_at,"epistemic_class":input.epistemic_class,"producer":input.producer}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "form_memory",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self
                    .memory(
                        input.subject,
                        MemoryId(
                            receipt
                                .result_ref
                                .ok_or_else(|| {
                                    Error::Infrastructure("form receipt has no result".into())
                                })?
                                .parse()
                                .map_err(|_| {
                                    Error::Infrastructure("invalid form receipt".into())
                                })?,
                        ),
                        None,
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "form_memory operation is already in progress".into(),
            ));
        }
        let memory_id = MemoryId::new();
        let revision_id = MemoryRevisionId::new();
        let now = Utc::now();
        self.validate_supports_in_tx(&mut tx, input.subject, &input.supports)
            .await?;
        sqlx::query("INSERT INTO memory_objects(memory_id,subject_id,cognitive_role,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,accessibility_mode,created_at) VALUES($1,$2,$3,$4,1,'accepted','valid','normal','normal','auto',$5)")
            .bind(memory_id.0).bind(input.subject.0).bind(input.cognitive_role.as_str()).bind(revision_id.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        self.insert_revision_in_tx(&mut tx, &input, memory_id, revision_id, None, None, 1, now)
            .await?;
        ProjectionInvalidation::all();
        AuthorityStore::invalidate_in(&mut tx, input.subject, ProjectionInvalidation::all())
            .await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "memory",
            Some(&memory_id.0.to_string()),
            Some(revision_id.0),
            Some(1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.memory(input.subject, memory_id, None).await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "revision insertion keeps immutable content and support commit together"
    )]
    async fn insert_revision_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        input: &ExplicitMemoryInput,
        memory_id: MemoryId,
        revision_id: MemoryRevisionId,
        parent: Option<MemoryRevisionId>,
        revision_intent: Option<RevisionIntent>,
        revision_no: i32,
        recorded_at: DateTime<Utc>,
    ) -> Result<()> {
        let producer_id = if let Some(p) = &input.producer {
            if !matches!(
                p.operation,
                CapabilityOperation::MemoryFormationText
                    | CapabilityOperation::MemoryConsolidationText
            ) {
                return Err(Error::Invalid(
                    "Memory producer requires a Memory formation operation".into(),
                ));
            }
            Some(AuthorityStore::register_producer_in(tx, p).await?)
        } else {
            None
        };
        let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO memory_revisions(memory_revision_id,memory_id,subject_id,revision_no,parent_revision_id,revision_intent,formation_mode,grounding_occurrence_id,semantic_role,title,representation_text,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)")
            .bind(revision_id.0).bind(memory_id.0).bind(input.subject.0).bind(revision_no).bind(parent.map(|id| id.0)).bind(revision_intent.map(|value| value.as_str())).bind(input.formation_mode.as_str()).bind(input.grounding_occurrence_id.map(|id| id.0)).bind(&input.semantic_role).bind(&input.title).bind(&input.representation_text).bind(format!("{:?}",input.epistemic_class).to_lowercase()).bind(valid_kind).bind(valid_start).bind(valid_end).bind(input.formed_at).bind(recorded_at).bind(producer_id).execute(&mut **tx).await.map_err(db)?;
        for (index, support) in input.supports.iter().enumerate() {
            self.insert_support_in_tx(tx, revision_id, index as i32, support)
                .await?;
        }
        for entity in &input.aboutness {
            sqlx::query("INSERT INTO memory_revision_aboutness(memory_revision_id,entity_ref) VALUES($1,$2)").bind(revision_id.0).bind(entity.as_str()).execute(&mut **tx).await.map_err(db)?;
        }
        for tag in &input.tags {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE subject_id=$1 AND tag_id=$2 AND status='active')").bind(input.subject.0).bind(tag.0).fetch_one(&mut **tx).await.map_err(db)?;
            if !exists {
                return Err(Error::Invalid("tag does not belong to Subject".into()));
            }
            sqlx::query(
                "INSERT INTO memory_revision_tags(memory_revision_id,tag_id) VALUES($1,$2)",
            )
            .bind(revision_id.0)
            .bind(tag.0)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
        Ok(())
    }

    async fn insert_support_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        revision: MemoryRevisionId,
        evidence_no: i32,
        support: &RevisionSupport,
    ) -> Result<()> {
        match support {
            RevisionSupport::Evidence(evidence) => {
                let (source_region, derived_representation, derived_region) = match evidence.locator
                {
                    EvidenceLocator::WholeOccurrence => (None, None, None),
                    EvidenceLocator::SourceRegion(id) => (Some(id.0), None, None),
                    EvidenceLocator::DerivedRepresentation(id) => (None, Some(id.0), None),
                    EvidenceLocator::DerivedRegion(id) => (None, None, Some(id.0)),
                };
                sqlx::query("INSERT INTO memory_revision_evidence(memory_revision_id,occurrence_id,evidence_no,source_region_id,derived_representation_id,derived_region_id,support_role) VALUES($1,$2,$3,$4,$5,$6,$7)")
                    .bind(revision.0).bind(evidence.occurrence_id.0).bind(evidence_no).bind(source_region).bind(derived_representation).bind(derived_region).bind(evidence.support_role.as_str()).execute(&mut **tx).await.map_err(db)?;
            }
            RevisionSupport::CognitionDependency(dependency) => {
                let (kind, value) = reference_parts(&dependency.target_revision);
                if !matches!(
                    dependency.target_revision,
                    CognitiveRef::MemoryRevision(_) | CognitiveRef::CognitiveSchemaRevision(_)
                ) {
                    return Err(Error::Invalid(
                        "cognition dependency must target an exact revision".into(),
                    ));
                }
                sqlx::query("INSERT INTO memory_revision_dependencies(memory_revision_id,target_ref_kind,target_ref,support_role) VALUES($1,$2,$3,$4)").bind(revision.0).bind(kind).bind(value).bind(dependency.support_role.as_str()).execute(&mut **tx).await.map_err(db)?;
            }
            RevisionSupport::Seed(_) => {
                return Err(Error::Invalid(
                    "Memory revisions cannot use Cognitive Seed support".into(),
                ));
            }
        }
        Ok(())
    }

    async fn validate_supports_for_subject(
        &self,
        subject: SubjectId,
        supports: &[RevisionSupport],
    ) -> Result<()> {
        for support in supports {
            match support {
                RevisionSupport::Evidence(evidence) => {
                    self.validate_evidence(subject, evidence).await?
                }
                RevisionSupport::CognitionDependency(dependency) => {
                    if !matches!(
                        dependency.target_revision,
                        CognitiveRef::MemoryRevision(_) | CognitiveRef::CognitiveSchemaRevision(_)
                    ) {
                        return Err(Error::Invalid(
                            "cognition dependency must target an exact revision".into(),
                        ));
                    }
                    self.store
                        .validate_reference(subject, &dependency.target_revision)
                        .await?;
                }
                RevisionSupport::Seed(_) => {
                    return Err(Error::Invalid(
                        "Memory revisions cannot use Cognitive Seed support".into(),
                    ));
                }
            }
        }
        // Resolve the complete exact-revision dependency closure before the
        // mutation starts.  This is also the deterministic cycle check; a
        // model or caller cannot bypass it by presenting a syntactically
        // valid direct edge.
        self.provenance_summary(subject, supports).await?;
        Ok(())
    }

    async fn validate_evidence(&self, subject: SubjectId, evidence: &EvidenceRef) -> Result<()> {
        let row = sqlx::query("SELECT artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2")
            .bind(subject.0).bind(evidence.occurrence_id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(|| Error::Invalid("evidence occurrence is outside Subject".into()))?;
        let occurrence_artifact: Option<Uuid> = row.try_get("artifact_id").map_err(db)?;
        match evidence.locator {
            EvidenceLocator::WholeOccurrence => {}
            EvidenceLocator::SourceRegion(id) => {
                let artifact: Uuid = sqlx::query_scalar("SELECT artifact_id FROM source_regions WHERE subject_id=$1 AND source_region_id=$2").bind(subject.0).bind(id.0).fetch_one(self.store.pool()).await.map_err(db)?;
                if Some(artifact) != occurrence_artifact {
                    return Err(Error::Invalid(
                        "source region artifact does not match occurrence".into(),
                    ));
                }
            }
            EvidenceLocator::DerivedRepresentation(id) => {
                let artifact: Option<Uuid> = sqlx::query_scalar("SELECT sr.artifact_id FROM representation_source_regions($1,$2) roots JOIN source_regions sr USING(source_region_id) WHERE sr.artifact_id=(SELECT artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$3)").bind(subject.0).bind(id.0).bind(evidence.occurrence_id.0).fetch_optional(self.store.pool()).await.map_err(db)?.flatten();
                if artifact.is_none() || artifact != occurrence_artifact {
                    return Err(Error::Invalid(
                        "derived representation source does not match occurrence".into(),
                    ));
                }
            }
            EvidenceLocator::DerivedRegion(id) => {
                let artifact: Option<Uuid> = sqlx::query_scalar("SELECT sr.artifact_id FROM derived_regions dr JOIN LATERAL representation_source_regions($1,dr.derived_representation_id) roots ON true JOIN source_regions sr USING(source_region_id) WHERE dr.subject_id=$1 AND dr.derived_region_id=$2 AND sr.artifact_id=(SELECT artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$3)").bind(subject.0).bind(id.0).bind(evidence.occurrence_id.0).fetch_optional(self.store.pool()).await.map_err(db)?.flatten();
                if artifact.is_none() || artifact != occurrence_artifact {
                    return Err(Error::Invalid(
                        "derived region source does not match occurrence".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn commit_formation_proposal(
        &self,
        subject: SubjectId,
        proposal: MemoryFormationProposal,
        allowed_entity_refs: &[EntityRef],
    ) -> Result<MemoryView> {
        if proposal
            .aboutness
            .iter()
            .any(|entity| !allowed_entity_refs.contains(entity))
        {
            return Err(Error::Invalid(
                "formation proposal contains an EntityRef outside Host context".into(),
            ));
        }
        self.form_memory(proposal.into_input(subject)).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "memory revision owns fencing, identity guard and immutable content commit"
    )]
    pub async fn revise_memory(&self, mut input: ReviseMemoryInput) -> Result<MemoryView> {
        input.producer = input
            .producer
            .as_ref()
            .map(AuthorityStore::canonical_producer)
            .transpose()?;
        if input.operation_id.0.is_nil() {
            return Err(Error::Invalid("operation_id is required".into()));
        }
        validate_content(&input.semantic_role, &input.representation_text)?;
        input.valid_time.validate()?;
        self.store.require_subject(input.subject).await?;
        self.validate_supports_for_subject(input.subject, &input.supports)
            .await?;
        self.validate_formation_semantics(input.subject, input.formation_mode, &input.supports)
            .await?;
        let digest = operation_digest(
            "revise_memory",
            input.subject,
            &serde_json::json!({"memory_id":input.memory_id,"expected_object_epoch":input.expected_object_epoch,"intent":input.intent,"formation_mode":input.formation_mode,"grounding_occurrence_id":input.grounding_occurrence_id,"semantic_role":input.semantic_role,"representation_text":input.representation_text,"title":input.title,"supports":input.supports,"aboutness":input.aboutness,"valid_time":input.valid_time,"formed_at":input.formed_at,"epistemic_class":input.epistemic_class,"producer":input.producer}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "revise_memory",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self
                    .memory(
                        input.subject,
                        input.memory_id,
                        receipt.result_revision.map(MemoryRevisionId),
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "revise_memory operation is already in progress".into(),
            ));
        }
        let row = sqlx::query("SELECT current_revision_id,object_epoch,cognitive_role,purge_state FROM memory_objects WHERE subject_id=$1 AND memory_id=$2 FOR UPDATE")
            .bind(input.subject.0).bind(input.memory_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(|| Error::NotFound("memory not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(Error::Conflict("expected object epoch is stale".into()));
        }
        if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
            return Err(Error::FailedPrecondition("memory is purging".into()));
        }
        self.validate_object_dependency_cycle(
            input.subject,
            &format!("memory:{}", input.memory_id.0),
            &input.supports,
        )
        .await?;
        self.validate_supports_in_tx(&mut tx, input.subject, &input.supports)
            .await?;
        let parent = MemoryRevisionId(row.try_get("current_revision_id").map_err(db)?);
        let parent_aboutness = sqlx::query_scalar::<_, String>(
            "SELECT entity_ref FROM memory_revision_aboutness WHERE memory_revision_id=$1",
        )
        .bind(parent.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !parent_aboutness.is_empty()
            && !input.aboutness.iter().any(|entity| {
                parent_aboutness
                    .iter()
                    .any(|value| value == entity.as_str())
            })
        {
            return Err(Error::FailedPrecondition(
                "revision aboutness is disjoint from the existing cognitive matter; form a new Memory object".into(),
            ));
        }
        let revision_no: i32 = sqlx::query_scalar(
            "SELECT revision_no+1 FROM memory_revisions WHERE memory_revision_id=$1",
        )
        .bind(parent.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let revision_id = MemoryRevisionId::new();
        let role: CognitiveRole =
            parse_enum(row.try_get("cognitive_role").map_err(db)?, "cognitive role")?;
        let input_for_insert = ExplicitMemoryInput {
            producer: input.producer,
            operation_id: input.operation_id,
            subject: input.subject,
            cognitive_role: role,
            formation_mode: input.formation_mode,
            grounding_occurrence_id: input.grounding_occurrence_id,
            semantic_role: input.semantic_role,
            representation_text: input.representation_text,
            title: input.title,
            supports: input.supports,
            aboutness: input.aboutness,
            tags: Vec::new(),
            valid_time: input.valid_time,
            formed_at: input.formed_at,
            epistemic_class: input.epistemic_class,
        };
        input_for_insert.validate()?;
        self.insert_revision_in_tx(
            &mut tx,
            &input_for_insert,
            input.memory_id,
            revision_id,
            Some(parent),
            Some(input.intent),
            revision_no,
            Utc::now(),
        )
        .await?;
        sqlx::query("UPDATE memory_objects SET current_revision_id=$3,object_epoch=object_epoch+1,integrity_state='valid' WHERE subject_id=$1 AND memory_id=$2").bind(input.subject.0).bind(input.memory_id.0).bind(revision_id.0).execute(&mut *tx).await.map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, input.subject, ProjectionInvalidation::all())
            .await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "memory_revision",
            Some(&input.memory_id.0.to_string()),
            Some(revision_id.0),
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.memory(input.subject, input.memory_id, Some(revision_id))
            .await
    }

    pub async fn link_revisions(
        &self,
        subject: SubjectId,
        operation_id: OperationId,
        from: MemoryRevisionId,
        to: MemoryRevisionId,
        relation: MemoryRelation,
    ) -> Result<()> {
        if from == to {
            return Err(Error::Invalid("a relation cannot target itself".into()));
        }
        self.store
            .validate_reference(subject, &CognitiveRef::MemoryRevision(from))
            .await?;
        self.store
            .validate_reference(subject, &CognitiveRef::MemoryRevision(to))
            .await?;
        let digest = operation_digest(
            "link_memory_revisions",
            subject,
            &serde_json::json!({"from":from,"to":to,"relation":relation}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "link_memory_revisions",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Ok(());
            }
            return Err(Error::Unavailable(
                "relation operation is already in progress".into(),
            ));
        }
        sqlx::query("INSERT INTO memory_revision_relations(from_revision_id,to_revision_id,relation,created_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(from.0).bind(to.0).bind(relation.as_str()).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::topology()).await?;
        commit_receipt(&mut tx, subject, operation_id, "relation", None, None, None).await?;
        tx.commit().await.map_err(db)
    }

    pub async fn create_tag(&self, subject: SubjectId, input: CreateTagRequest) -> Result<Tag> {
        if input.label.trim().is_empty() || input.label.len() > 256 {
            return Err(Error::Invalid("tag label is invalid".into()));
        }
        let digest = operation_digest(
            "create_tag",
            subject,
            &serde_json::json!({"label":input.label,"description":input.description,"kind_hint":input.kind_hint,"origin":input.origin}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, input.operation_id).await?;
        if let Some(receipt) =
            check_receipt(&mut tx, subject, input.operation_id, "create_tag", &digest).await?
        {
            let id = receipt.result_ref;
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self
                    .tag(
                        subject,
                        TagId(
                            id.ok_or_else(|| {
                                Error::Infrastructure("tag receipt missing result".into())
                            })?
                            .parse()
                            .map_err(|_| Error::Infrastructure("invalid tag receipt".into()))?,
                        ),
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "tag operation is already in progress".into(),
            ));
        }
        let tag_id = TagId::new();
        let revision_id = Uuid::now_v7();
        let now = Utc::now();
        sqlx::query("INSERT INTO tags(tag_id,subject_id,current_revision_id,created_at,status) VALUES($1,$2,$3,$4,'active')").bind(tag_id.0).bind(subject.0).bind(revision_id).bind(now).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO tag_revisions(tag_revision_id,tag_id,revision_no,label,description,kind_hint,origin,created_at) VALUES($1,$2,1,$3,$4,$5,$6,$7)").bind(revision_id).bind(tag_id.0).bind(&input.label).bind(&input.description).bind(&input.kind_hint).bind(&input.origin).bind(now).execute(&mut *tx).await.map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::topology()).await?;
        commit_receipt(
            &mut tx,
            subject,
            input.operation_id,
            "tag",
            Some(&tag_id.0.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.tag(subject, tag_id).await
    }

    async fn tag(&self, subject: SubjectId, tag_id: TagId) -> Result<Tag> {
        let row=sqlx::query("SELECT tag_id,subject_id,current_revision_id,created_at,status FROM tags WHERE subject_id=$1 AND tag_id=$2").bind(subject.0).bind(tag_id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(||Error::NotFound("tag not found".into()))?;
        Ok(Tag {
            tag_id: TagId(row.try_get("tag_id").map_err(db)?),
            subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
            current_revision_id: row.try_get("current_revision_id").map_err(db)?,
            status: row.try_get("status").map_err(db)?,
            created_at: row.try_get("created_at").map_err(db)?,
        })
    }
}
