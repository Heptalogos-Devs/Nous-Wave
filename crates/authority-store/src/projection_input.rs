//! Coherent read snapshots used to build disposable serving artifacts.
use crate::{AuthorityStore, database_error as db};
use nous_core::*;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextProjectionSource {
    pub reference: CognitiveRef,
    pub revision: Option<CognitiveRef>,
    pub text: Option<String>,
    pub content_hash: Option<String>,
    pub title: Option<String>,
    pub media_type: String,
    pub source_class: Option<String>,
    pub source_region: Option<SourceRegionId>,
    pub entity_refs: Vec<String>,
    pub tag_ids: Vec<String>,
    pub schema_ids: Vec<String>,
}

pub struct TextProjectionInput {
    pub watermark: i64,
    pub sources: Vec<TextProjectionSource>,
}

pub(crate) async fn watermark(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    family: &str,
    space: &str,
) -> Result<i64> {
    sqlx::query_scalar("SELECT COALESCE(max(desired_authority_seq),0) FROM projection_watermarks WHERE subject_id=$1 AND family=$2 AND (space_signature=$3 OR space_signature='*')")
        .bind(subject.0).bind(family).bind(space).fetch_one(&mut **tx).await.map_err(db)
}

impl AuthorityStore {
    pub async fn projection_watermark(
        &self,
        subject: SubjectId,
        family: &str,
        space: &str,
    ) -> Result<i64> {
        let mut tx = self.begin().await?;
        watermark(&mut tx, subject, family, space).await
    }

    pub async fn text_projection_input(
        &self,
        subject: SubjectId,
        family: &str,
        space: &str,
        memory_enabled: bool,
        self_enabled: bool,
        social_enabled: bool,
    ) -> Result<TextProjectionInput> {
        let mut tx = self.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let watermark = watermark(&mut tx, subject, family, space).await?;
        let mut sources = material_sources(&mut tx, subject).await?;
        if self_enabled {
            sources.extend(self_sources(&mut tx, subject).await?);
        }
        if social_enabled {
            sources.extend(social_sources(&mut tx, subject).await?);
        }
        if memory_enabled {
            sources.extend(memory_sources(&mut tx, subject).await?);
        }
        sources.sort_by_key(|source| source.reference.to_string());
        tx.commit().await.map_err(db)?;
        Ok(TextProjectionInput { watermark, sources })
    }
}

pub(crate) async fn memory_sources(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<Vec<TextProjectionSource>> {
    let rows = sqlx::query("SELECT o.memory_id,r.memory_revision_id,r.representation_text,r.title FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal' ORDER BY o.memory_id")
        .bind(subject.0).fetch_all(&mut **tx).await.map_err(db)?;
    let mut sources = Vec::new();
    for row in rows {
        let revision: Uuid = row.try_get("memory_revision_id").map_err(db)?;
        let entities = sqlx::query_scalar("SELECT DISTINCT entity_ref FROM memory_revision_aboutness WHERE memory_revision_id=$1 ORDER BY entity_ref")
            .bind(revision).fetch_all(&mut **tx).await.map_err(db)?;
        let tags = sqlx::query_scalar("SELECT tag_id::text FROM memory_revision_tags WHERE memory_revision_id=$1 ORDER BY tag_id")
            .bind(revision).fetch_all(&mut **tx).await.map_err(db)?;
        let schema_ids = sqlx::query_scalar("SELECT DISTINCT s.schema_id::text FROM cognitive_schemas s JOIN cognitive_schema_revisions sr ON sr.schema_revision_id=s.current_revision_id JOIN cognitive_schema_evidence_links l ON l.schema_revision_id=sr.schema_revision_id WHERE s.subject_id=$1 AND s.acceptance_state='accepted' AND s.integrity_state='valid' AND s.suppression_state='normal' AND s.purge_state='normal' AND l.revoked_at IS NULL AND l.support_kind='memory_revision' AND l.support_ref=$2")
            .bind(subject.0).bind(revision.to_string()).fetch_all(&mut **tx).await.map_err(db)?;
        sources.push(TextProjectionSource {
            reference: CognitiveRef::MemoryRevision(MemoryRevisionId(revision)),
            revision: Some(CognitiveRef::MemoryRevision(MemoryRevisionId(revision))),
            text: Some(row.try_get("representation_text").map_err(db)?),
            content_hash: None,
            title: row.try_get("title").map_err(db)?,
            media_type: "text/plain".into(),
            source_class: None,
            source_region: None,
            entity_refs: entities,
            tag_ids: tags,
            schema_ids,
        });
    }
    for (table, sql) in [
        (
            "tags",
            "SELECT o.tag_id AS id,NULL::uuid AS revision_id,r.label FROM tags o JOIN tag_revisions r ON r.tag_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.status='active'",
        ),
        (
            "cognitive_schemas",
            "SELECT o.schema_id AS id,r.schema_revision_id AS revision_id,r.structural_claim AS label FROM cognitive_schemas o JOIN cognitive_schema_revisions r ON r.schema_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.acceptance_state='accepted'",
        ),
    ] {
        for row in sqlx::query(sql)
            .bind(subject.0)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?
        {
            let id: Uuid = row.try_get("id").map_err(db)?;
            let revision: Option<Uuid> = row.try_get("revision_id").map_err(db)?;
            let reference = if table == "tags" {
                CognitiveRef::Tag(TagId(id))
            } else {
                CognitiveRef::CognitiveSchemaRevision(CognitiveSchemaRevisionId(
                    revision
                        .ok_or_else(|| Error::Infrastructure("schema revision missing".into()))?,
                ))
            };
            sources.push(TextProjectionSource {
                revision: match &reference {
                    CognitiveRef::CognitiveSchemaRevision(value) => {
                        Some(CognitiveRef::CognitiveSchemaRevision(*value))
                    }
                    _ => None,
                },
                reference,
                text: Some(row.try_get("label").map_err(db)?),
                content_hash: None,
                title: None,
                media_type: "text/plain".into(),
                source_class: None,
                source_region: None,
                entity_refs: Vec::new(),
                tag_ids: Vec::new(),
                schema_ids: Vec::new(),
            });
        }
    }
    Ok(sources)
}

pub(crate) async fn self_sources(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<Vec<TextProjectionSource>> {
    let mut sources = Vec::new();
    let facets = sqlx::query("SELECT r.self_facet_revision_id,f.kind,f.key,r.statement FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_revision_id=f.current_revision_id WHERE f.subject_id=$1 AND f.acceptance_state='accepted' AND f.integrity_state='valid' AND f.suppression_state='normal' AND f.purge_state='normal' ORDER BY CASE f.kind WHEN 'identity' THEN 0 WHEN 'role' THEN 1 WHEN 'limitation' THEN 2 WHEN 'capability' THEN 3 WHEN 'value' THEN 4 WHEN 'preference' THEN 5 WHEN 'tendency' THEN 6 ELSE 99 END,f.key,r.self_facet_revision_id")
        .bind(subject.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    for row in facets {
        let revision = CognitiveRef::SelfFacetRevision(SelfFacetRevisionId(
            row.try_get("self_facet_revision_id").map_err(db)?,
        ));
        sources.push(TextProjectionSource {
            reference: revision.clone(),
            revision: Some(revision),
            text: Some(row.try_get("statement").map_err(db)?),
            content_hash: None,
            title: Some(row.try_get::<String, _>("key").map_err(db)?),
            media_type: "text/plain".into(),
            source_class: Some("self".into()),
            source_region: None,
            entity_refs: Vec::new(),
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    let narratives = sqlx::query("SELECT r.narrative_identity_revision_id,n.key,r.text FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_revision_id=n.current_revision_id WHERE n.subject_id=$1 AND n.acceptance_state='accepted' AND n.integrity_state='valid' AND n.suppression_state='normal' AND n.purge_state='normal' ORDER BY n.key,r.narrative_identity_revision_id")
        .bind(subject.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    for row in narratives {
        let revision = CognitiveRef::NarrativeIdentityRevision(NarrativeIdentityRevisionId(
            row.try_get("narrative_identity_revision_id").map_err(db)?,
        ));
        sources.push(TextProjectionSource {
            reference: revision.clone(),
            revision: Some(revision),
            text: Some(row.try_get("text").map_err(db)?),
            content_hash: None,
            title: Some(row.try_get::<String, _>("key").map_err(db)?),
            media_type: "text/plain".into(),
            source_class: Some("self:narrative".into()),
            source_region: None,
            entity_refs: Vec::new(),
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    Ok(sources)
}

pub(crate) async fn social_sources(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<Vec<TextProjectionSource>> {
    let mut sources = Vec::new();
    let rows = sqlx::query("SELECT r.relationship_revision_id,t.key,a.from_kind,a.from_entity_ref,a.to_kind,a.to_entity_ref,r.degree_value FROM relationship_assertions a JOIN relation_type_definitions t USING(relation_type_id) JOIN relationship_revisions r ON r.relationship_revision_id=a.current_revision_id WHERE a.subject_id=$1 AND a.acceptance_state='accepted' AND a.integrity_state='valid' AND a.suppression_state='normal' AND a.purge_state='normal' ORDER BY r.relationship_revision_id")
        .bind(subject.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    for row in rows {
        let revision = RelationshipRevisionId(row.try_get("relationship_revision_id").map_err(db)?);
        let from_kind: String = row.try_get("from_kind").map_err(db)?;
        let from_ref: Option<String> = row.try_get("from_entity_ref").map_err(db)?;
        let to_kind: String = row.try_get("to_kind").map_err(db)?;
        let to_ref: Option<String> = row.try_get("to_entity_ref").map_err(db)?;
        let from = from_ref
            .clone()
            .map_or(from_kind.clone(), |value| format!("{from_kind}:{value}"));
        let to = to_ref
            .clone()
            .map_or(to_kind.clone(), |value| format!("{to_kind}:{value}"));
        let entity_refs = [from_ref, to_ref].into_iter().flatten().collect::<Vec<_>>();
        sources.push(TextProjectionSource {
            reference: CognitiveRef::RelationshipRevision(revision),
            revision: Some(CognitiveRef::RelationshipRevision(revision)),
            text: Some(format!(
                "relationship\ntype={}\nfrom={from}\nto={to}\ndegree={}",
                row.try_get::<String, _>("key").map_err(db)?,
                row.try_get::<Option<serde_json::Value>, _>("degree_value")
                    .map_err(db)?
                    .map_or_else(|| "none".into(), |value| value.to_string())
            )),
            content_hash: None,
            title: None,
            media_type: "text/plain".into(),
            source_class: Some("social:relationship".into()),
            source_region: None,
            entity_refs,
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    let rows = sqlx::query("SELECT r.convention_revision_id,c.expression,c.scope_kind,c.scope_refs,r.meaning,r.pragmatic_role FROM language_conventions c JOIN language_convention_revisions r ON r.convention_revision_id=c.current_revision_id WHERE c.subject_id=$1 AND c.acceptance_state='accepted' AND c.integrity_state='valid' AND c.suppression_state='normal' AND c.purge_state='normal' ORDER BY r.convention_revision_id")
        .bind(subject.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    for row in rows {
        let revision =
            LanguageConventionRevisionId(row.try_get("convention_revision_id").map_err(db)?);
        let scope_refs: Vec<String> = row.try_get("scope_refs").map_err(db)?;
        sources.push(TextProjectionSource {
            reference: CognitiveRef::LanguageConventionRevision(revision),
            revision: Some(CognitiveRef::LanguageConventionRevision(revision)),
            text: Some(format!(
                "language_convention\nexpression={}\nmeaning={}\nrole={}\nscope={}:{}",
                row.try_get::<String, _>("expression").map_err(db)?,
                row.try_get::<String, _>("meaning").map_err(db)?,
                row.try_get::<Option<String>, _>("pragmatic_role")
                    .map_err(db)?
                    .unwrap_or_default(),
                row.try_get::<String, _>("scope_kind").map_err(db)?,
                scope_refs.join("|")
            )),
            content_hash: None,
            title: None,
            media_type: "text/plain".into(),
            source_class: Some("social:language_convention".into()),
            source_region: None,
            entity_refs: Vec::new(),
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    Ok(sources)
}

async fn material_sources(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<Vec<TextProjectionSource>> {
    let rows = sqlx::query("SELECT o.occurrence_id,o.source_class,a.content_hash,a.media_type,o.artifact_id FROM observation_occurrences o LEFT JOIN artifacts a ON a.artifact_id=o.artifact_id WHERE o.subject_id=$1 ORDER BY o.occurrence_id")
        .bind(subject.0).fetch_all(&mut **tx).await.map_err(db)?;
    let mut sources = Vec::new();
    for row in rows {
        let occurrence: Uuid = row.try_get("occurrence_id").map_err(db)?;
        let entities = sqlx::query_scalar("SELECT DISTINCT b.entity_ref FROM entity_mentions m JOIN entity_binding_revisions b USING(mention_id) WHERE m.occurrence_id=$1 AND b.revision_no=(SELECT max(b2.revision_no) FROM entity_binding_revisions b2 WHERE b2.mention_id=b.mention_id) AND b.binding_state='bound' AND b.entity_ref IS NOT NULL")
            .bind(occurrence).fetch_all(&mut **tx).await.map_err(db)?;
        let region: Option<Uuid> = sqlx::query_scalar("SELECT source_region_id FROM source_regions WHERE artifact_id=$1 AND coordinate_kind='whole_artifact' ORDER BY source_region_id LIMIT 1")
            .bind(row.try_get::<Option<Uuid>,_>("artifact_id").map_err(db)?).fetch_optional(&mut **tx).await.map_err(db)?;
        sources.push(TextProjectionSource {
            reference: CognitiveRef::Occurrence(OccurrenceId(occurrence)),
            revision: None,
            text: None,
            content_hash: row.try_get("content_hash").map_err(db)?,
            title: None,
            media_type: row
                .try_get::<Option<String>, _>("media_type")
                .map_err(db)?
                .unwrap_or_else(|| "application/octet-stream".into()),
            source_class: Some(row.try_get("source_class").map_err(db)?),
            source_region: region.map(SourceRegionId),
            entity_refs: entities,
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    let rows = sqlx::query("SELECT r.source_region_id,r.created_at,a.content_hash,a.media_type,o.source_class FROM source_regions r JOIN artifacts a ON a.artifact_id=r.artifact_id LEFT JOIN LATERAL (SELECT source_class FROM observation_occurrences WHERE subject_id=r.subject_id AND artifact_id=r.artifact_id ORDER BY observed_at DESC LIMIT 1) o ON true WHERE r.subject_id=$1 ORDER BY r.source_region_id")
        .bind(subject.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    for row in rows {
        sources.push(TextProjectionSource {
            reference: CognitiveRef::SourceRegion(SourceRegionId(
                row.try_get("source_region_id").map_err(db)?,
            )),
            revision: None,
            text: None,
            content_hash: Some(row.try_get("content_hash").map_err(db)?),
            title: None,
            media_type: row.try_get("media_type").map_err(db)?,
            source_class: row.try_get("source_class").map_err(db)?,
            source_region: Some(SourceRegionId(row.try_get("source_region_id").map_err(db)?)),
            entity_refs: Vec::new(),
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    let rows = sqlx::query("SELECT d.derived_representation_id,d.source_region_id,d.payload_text,d.representation_kind,a.content_hash,a.media_type FROM derived_representations d LEFT JOIN artifacts a ON a.artifact_id=d.payload_artifact_id WHERE d.subject_id=$1 ORDER BY d.derived_representation_id")
        .bind(subject.0).fetch_all(&mut **tx).await.map_err(db)?;
    for row in rows {
        let text: Option<String> = row.try_get("payload_text").map_err(db)?;
        sources.push(TextProjectionSource {
            reference: CognitiveRef::DerivedRepresentation(DerivedRepresentationId(
                row.try_get("derived_representation_id").map_err(db)?,
            )),
            revision: None,
            media_type: if text.is_some() {
                "text/plain".into()
            } else {
                row.try_get::<Option<String>, _>("media_type")
                    .map_err(db)?
                    .unwrap_or_else(|| "application/octet-stream".into())
            },
            text,
            content_hash: row.try_get("content_hash").map_err(db)?,
            title: Some(row.try_get("representation_kind").map_err(db)?),
            source_class: Some("derived".into()),
            source_region: Some(SourceRegionId(row.try_get("source_region_id").map_err(db)?)),
            entity_refs: Vec::new(),
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    let rows = sqlx::query("SELECT r.derived_region_id,r.created_at,d.source_region_id,d.payload_text,d.representation_kind,a.content_hash,a.media_type FROM derived_regions r JOIN derived_representations d ON d.derived_representation_id=r.derived_representation_id LEFT JOIN artifacts a ON a.artifact_id=d.payload_artifact_id WHERE r.subject_id=$1 ORDER BY r.derived_region_id")
        .bind(subject.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    for row in rows {
        let text: Option<String> = row.try_get("payload_text").map_err(db)?;
        sources.push(TextProjectionSource {
            reference: CognitiveRef::DerivedRegion(DerivedRegionId(
                row.try_get("derived_region_id").map_err(db)?,
            )),
            revision: None,
            media_type: if text.is_some() {
                "text/plain".into()
            } else {
                row.try_get::<Option<String>, _>("media_type")
                    .map_err(db)?
                    .unwrap_or_else(|| "application/octet-stream".into())
            },
            text,
            content_hash: row.try_get("content_hash").map_err(db)?,
            title: Some(row.try_get("representation_kind").map_err(db)?),
            source_class: Some("derived".into()),
            source_region: Some(SourceRegionId(row.try_get("source_region_id").map_err(db)?)),
            entity_refs: Vec::new(),
            tag_ids: Vec::new(),
            schema_ids: Vec::new(),
        });
    }
    Ok(sources)
}
