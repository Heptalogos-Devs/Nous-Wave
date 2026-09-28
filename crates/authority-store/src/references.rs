use crate::*;
use database_error as db;
use nous_core::*;
use sqlx::Row;

impl AuthorityStore {
    /// Resolve a query exact target once at binding time.  Mutable cognition
    /// object refs are deliberately converted to their current exact revision;
    /// callers retain the returned epoch and must reject a changed head rather
    /// than silently rebinding during execution.
    #[expect(
        clippy::too_many_lines,
        reason = "exact reference binding keeps all owner-specific fencing branches together"
    )]
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
            CognitiveRef::SelfFacet(facet) => {
                let row = sqlx::query(
                    "SELECT current_revision_id,object_epoch FROM self_facets WHERE subject_id=$1 AND self_facet_id=$2",
                )
                .bind(subject.0)
                .bind(facet.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("Self facet exact target not found".into()))?;
                (
                    CognitiveRef::SelfFacetRevision(SelfFacetRevisionId(
                        row.try_get("current_revision_id").map_err(db)?,
                    )),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    true,
                )
            }
            CognitiveRef::SelfFacetRevision(revision) => {
                let row = sqlx::query(
                    "SELECT f.object_epoch FROM self_facet_revisions r JOIN self_facets f USING(self_facet_id) WHERE r.subject_id=$1 AND r.self_facet_revision_id=$2",
                )
                .bind(subject.0)
                .bind(revision.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("Self facet revision exact target not found".into()))?;
                (
                    reference.clone(),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    false,
                )
            }
            CognitiveRef::NarrativeIdentity(narrative) => {
                let row = sqlx::query(
                    "SELECT current_revision_id,object_epoch FROM narrative_identities WHERE subject_id=$1 AND narrative_identity_id=$2",
                )
                .bind(subject.0)
                .bind(narrative.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("Narrative Identity exact target not found".into()))?;
                (
                    CognitiveRef::NarrativeIdentityRevision(NarrativeIdentityRevisionId(
                        row.try_get("current_revision_id").map_err(db)?,
                    )),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    true,
                )
            }
            CognitiveRef::NarrativeIdentityRevision(revision) => {
                let row = sqlx::query(
                    "SELECT n.object_epoch FROM narrative_identity_revisions r JOIN narrative_identities n USING(narrative_identity_id) WHERE r.subject_id=$1 AND r.narrative_identity_revision_id=$2",
                )
                .bind(subject.0)
                .bind(revision.0)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("Narrative Identity revision exact target not found".into()))?;
                (
                    reference.clone(),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    false,
                )
            }
            CognitiveRef::RelationshipAssertion(relationship) => {
                let row = sqlx::query("SELECT current_revision_id,object_epoch FROM relationship_assertions WHERE subject_id=$1 AND relationship_id=$2")
                    .bind(subject.0)
                    .bind(relationship.0)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db)?
                    .ok_or_else(|| Error::NotFound("relationship exact target not found".into()))?;
                (
                    CognitiveRef::RelationshipRevision(RelationshipRevisionId(
                        row.try_get("current_revision_id").map_err(db)?,
                    )),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    true,
                )
            }
            CognitiveRef::RelationshipRevision(revision) => {
                let row = sqlx::query("SELECT a.object_epoch FROM relationship_revisions r JOIN relationship_assertions a USING(relationship_id) WHERE r.subject_id=$1 AND r.relationship_revision_id=$2")
                    .bind(subject.0)
                    .bind(revision.0)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db)?
                    .ok_or_else(|| Error::NotFound("relationship revision exact target not found".into()))?;
                (
                    reference.clone(),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    false,
                )
            }
            CognitiveRef::LanguageConvention(convention) => {
                let row = sqlx::query("SELECT current_revision_id,object_epoch FROM language_conventions WHERE subject_id=$1 AND convention_id=$2")
                    .bind(subject.0)
                    .bind(convention.0)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db)?
                    .ok_or_else(|| Error::NotFound("LanguageConvention exact target not found".into()))?;
                (
                    CognitiveRef::LanguageConventionRevision(LanguageConventionRevisionId(
                        row.try_get("current_revision_id").map_err(db)?,
                    )),
                    Some(row.try_get("object_epoch").map_err(db)?),
                    true,
                )
            }
            CognitiveRef::LanguageConventionRevision(revision) => {
                let row = sqlx::query("SELECT c.object_epoch FROM language_convention_revisions r JOIN language_conventions c USING(convention_id) WHERE r.subject_id=$1 AND r.convention_revision_id=$2")
                    .bind(subject.0)
                    .bind(revision.0)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db)?
                    .ok_or_else(|| Error::NotFound("LanguageConvention revision exact target not found".into()))?;
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

    #[expect(
        clippy::too_many_lines,
        reason = "subject ownership checks are explicit per cognitive reference owner"
    )]
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
            CognitiveRef::CognitiveSeedVersion(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM cognitive_seed_versions WHERE subject_id=$1 AND seed_version_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::SelfFacet(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM self_facets WHERE subject_id=$1 AND self_facet_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::SelfFacetRevision(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM self_facet_revisions WHERE subject_id=$1 AND self_facet_revision_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::NarrativeIdentity(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM narrative_identities WHERE subject_id=$1 AND narrative_identity_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::NarrativeIdentityRevision(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM narrative_identity_revisions WHERE subject_id=$1 AND narrative_identity_revision_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::RelationshipAssertion(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM relationship_assertions WHERE subject_id=$1 AND relationship_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::RelationshipRevision(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM relationship_revisions WHERE subject_id=$1 AND relationship_revision_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::LanguageConvention(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM language_conventions WHERE subject_id=$1 AND convention_id=$2)",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(self.pool())
            .await
            .map_err(db)?,
            CognitiveRef::LanguageConventionRevision(id) => sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM language_convention_revisions WHERE subject_id=$1 AND convention_revision_id=$2)",
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
                "SELECT EXISTS(SELECT 1 FROM observation_occurrences WHERE subject_id=$1 AND actor_entity_ref=$2) OR EXISTS(SELECT 1 FROM entity_mentions m JOIN entity_binding_revisions b USING(mention_id) WHERE m.subject_id=$1 AND b.entity_ref=$2 AND b.binding_state='bound') OR EXISTS(SELECT 1 FROM memory_revision_aboutness e JOIN memory_revisions r USING(memory_revision_id) WHERE r.subject_id=$1 AND e.entity_ref=$2) OR EXISTS(SELECT 1 FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND $2 = ANY(r.aboutness))",
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
