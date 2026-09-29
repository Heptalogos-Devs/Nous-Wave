//! Subject identity, initialization material, configuration and lifecycle.

mod seed;

use chrono::{DateTime, Utc};
use nous_authority_store::AuthorityStore;
use nous_configuration_service::{
    ConfigurationService, process_capabilities, subject_default_capabilities,
};
use nous_core::{Error, OperationId, Result, SubjectId};
use nous_object_store::ObjectStore;
use serde::{Deserialize, Serialize};
use sqlx::Row;

pub use nous_configuration_service::SubjectCapabilities;
pub use seed::{
    COGNITIVE_SEED_FORMAT, CognitiveSeedInput, CognitiveSeedVersion, CognitiveSeedView,
    SeedAdoptionKind, SubjectSeedAdoption,
};

#[derive(Clone)]
pub struct SubjectCoreService {
    pub store: AuthorityStore,
    pub objects: ObjectStore,
    pub configuration: ConfigurationService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubject {
    pub subject_id: Option<SubjectId>,
    pub operation_id: OperationId,
    pub cognitive_seed: CognitiveSeedInput,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub capabilities: Option<SubjectCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectView {
    pub subject_id: SubjectId,
    pub created_at: DateTime<Utc>,
    pub authority_seq: i64,
    pub status: String,
    pub metadata: serde_json::Value,
    pub capabilities: SubjectCapabilities,
}

impl SubjectCoreService {
    pub fn new(
        store: AuthorityStore,
        objects: ObjectStore,
        configuration: ConfigurationService,
    ) -> Self {
        Self {
            store,
            objects,
            configuration,
        }
    }

    pub async fn create_subject(&self, input: CreateSubject) -> Result<SubjectView> {
        input.cognitive_seed.validate()?;
        if input.operation_id.0.is_nil() {
            return Err(Error::Invalid("operation_id is required".into()));
        }
        let subject = input.subject_id.unwrap_or_default();
        let snapshot = self.configuration.active_system_snapshot()?;
        let process = process_capabilities(&snapshot)?;
        let capabilities = input
            .capabilities
            .unwrap_or(subject_default_capabilities(&snapshot)?);
        let capabilities = capabilities.require_process(process)?;
        let guard = self.objects.reference_guard(false).await?;
        let hash = self
            .objects
            .put(input.cognitive_seed.text.as_bytes().to_vec())
            .await?;
        let mut tx = self.store.begin().await?;
        sqlx::query("INSERT INTO subjects(subject_id,created_at,metadata) VALUES($1,$2,$3)")
            .bind(subject.0)
            .bind(Utc::now())
            .bind(input.metadata)
            .execute(&mut *tx)
            .await
            .map_err(nous_authority_store::database_error)?;
        sqlx::query("INSERT INTO subject_capabilities(subject_id,memory) VALUES($1,$2)")
            .bind(subject.0)
            .bind(capabilities.memory)
            .execute(&mut *tx)
            .await
            .map_err(nous_authority_store::database_error)?;
        seed::insert_seed(
            &mut tx,
            subject,
            input.operation_id,
            input.cognitive_seed,
            hash,
            SeedAdoptionKind::Initial,
            &nous_core::canonical_request_digest("subject.create", subject, &input.operation_id)?,
        )
        .await?;
        tx.commit()
            .await
            .map_err(nous_authority_store::database_error)?;
        drop(guard);
        self.subject(subject).await
    }

    pub async fn subject(&self, subject: SubjectId) -> Result<SubjectView> {
        let row = sqlx::query("SELECT s.subject_id,s.created_at,s.authority_seq,s.status,s.metadata,c.memory FROM subjects s JOIN subject_capabilities c USING(subject_id) WHERE s.subject_id=$1")
            .bind(subject.0).fetch_optional(self.store.pool()).await
            .map_err(nous_authority_store::database_error)?
            .ok_or_else(|| Error::NotFound("subject not found".into()))?;
        Ok(SubjectView {
            subject_id: subject,
            created_at: row
                .try_get("created_at")
                .map_err(nous_authority_store::database_error)?,
            authority_seq: row
                .try_get("authority_seq")
                .map_err(nous_authority_store::database_error)?,
            status: row
                .try_get("status")
                .map_err(nous_authority_store::database_error)?,
            metadata: row
                .try_get("metadata")
                .map_err(nous_authority_store::database_error)?,
            capabilities: SubjectCapabilities {
                memory: row
                    .try_get("memory")
                    .map_err(nous_authority_store::database_error)?,
            },
        })
    }
}
