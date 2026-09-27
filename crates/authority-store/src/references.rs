use crate::*;
use database_error as db;
use nous_core::*;
use sqlx::Row;

impl AuthorityStore {
    /// Resolve a query exact target once at binding time.  Mutable cognition
    /// object refs are deliberately converted to their current exact revision;
    /// callers retain the returned epoch and must reject a changed head rather
    /// than silently rebinding during execution.
    pub async fn bind_exact_reference(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<(CognitiveRef, Option<i64>, bool)> {
        let bound = match reference {
            CognitiveRef::Memory(memory) => {
                let row = sqlx::query(
                    "SELECT current_revision_id,object_epoch FROM memory_objects WHERE subject_id=$1 AND memory_id=$2",
                )
                .bind(subject.0)
                .bind(memory.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("memory exact target not found".into()))?;
                (
                    CognitiveRef::MemoryRevision(MemoryRevisionId(
                        row.try_get("current_revision_id").map_err(db)?,
                    )),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    true,
                )
            }
            CognitiveRef::MemoryRevision(revision) => {
                let row = sqlx::query(
                    "SELECT o.object_epoch FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2",
                )
                .bind(subject.0)
                .bind(revision.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("memory revision exact target not found".into()))?;
                (
                    reference.clone(),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    false,
                )
            }
            CognitiveRef::CognitiveSchema(schema) => {
                let row = sqlx::query(
                    "SELECT current_revision_id,object_epoch FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2",
                )
                .bind(subject.0)
                .bind(schema.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("schema exact target not found".into()))?;
                (
                    CognitiveRef::CognitiveSchemaRevision(CognitiveSchemaRevisionId(
                        row.try_get("current_revision_id").map_err(db)?,
                    )),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    true,
                )
            }
            CognitiveRef::CognitiveSchemaRevision(revision) => {
                let row = sqlx::query(
                    "SELECT s.object_epoch FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2",
                )
                .bind(subject.0)
                .bind(revision.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("schema revision exact target not found".into()))?;
                (
                    reference.clone(),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    false,
                )
            }
            _ => {
                self.validate_reference(subject, reference).await?;
                (reference.clone(), None, false)
            }
        };
        Ok(bound)
    }

    pub async fn require_subject(&self, subject: SubjectId) -> Result<()> {
        if self.subject_exists(subject).await? {
            Ok(())
        } else {
            Err(Error::NotFound("subject not found".into()))
        }
    }

    pub async fn reference_in_subject(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<bool> {
        let valid = match reference {
            CognitiveRef::Memory(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM memory_objects WHERE subject_id=$1 AND memory_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::MemoryRevision(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::CognitiveSchema(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::CognitiveSchemaRevision(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::Artifact(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM artifacts WHERE subject_id=$1 AND artifact_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::SourceRegion(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM source_regions WHERE subject_id=$1 AND source_region_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::DerivedRepresentation(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM derived_representations WHERE subject_id=$1 AND derived_representation_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::DerivedRegion(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM derived_regions WHERE subject_id=$1 AND derived_region_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::Tag(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE subject_id=$1 AND tag_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::Resource(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM resources WHERE subject_id=$1 AND resource_ref=$2)",
            )
            .bind(subject.0)
            .bind(id.as_str())
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::ExternalObject(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM observation_occurrences WHERE subject_id=$1 AND external_object_ref=$2)",
            )
            .bind(subject.0)
            .bind(id.as_str())
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::Occurrence(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::Entity(id) => {
                EntityRef::new(id.as_str())?;
                sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM observation_occurrences WHERE subject_id=$1 AND actor_entity_ref=$2) OR EXISTS(SELECT 1 FROM memory_revision_aboutness e JOIN memory_revisions r USING(memory_revision_id) WHERE r.subject_id=$1 AND e.entity_ref=$2) OR EXISTS(SELECT 1 FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND $2 = ANY(r.aboutness))",
                )
                .bind(subject.0)
                .bind(id.as_str())
                .fetch_one(self.pool())
                .await
                .map_err(db)?
            }
            CognitiveRef::Session(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM cognitive_sessions WHERE subject_id=$1 AND session_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
        };
        Ok(valid)
    }

    pub async fn validate_reference(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<()> {
        self.require_subject(subject).await?;
        if self.reference_in_subject(subject, reference).await? {
            Ok(())
        } else {
            Err(Error::NotFound("cognitive reference not found".into()))
        }
    }
}
