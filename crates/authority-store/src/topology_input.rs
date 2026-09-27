use crate::{AuthorityStore, database_error as db, projection_input::watermark};
use nous_core::*;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdgeSource {
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub support_class: String,
    pub association_kind: String,
    pub polarity: String,
    pub support_mass: f64,
}

pub struct TopologyProjectionInput {
    pub watermark: i64,
    pub nodes: Vec<CognitiveRef>,
    pub edges: Vec<TopologyEdgeSource>,
}

fn link(from: CognitiveRef, to: CognitiveRef, kind: &str) -> TopologyEdgeSource {
    TopologyEdgeSource {
        from,
        to,
        support_class: "derived_structure".into(),
        association_kind: kind.into(),
        polarity: "positive".into(),
        support_mass: 1.0,
    }
}

impl AuthorityStore {
    pub async fn topology_projection_input(
        &self,
        subject: SubjectId,
        memory_enabled: bool,
    ) -> Result<TopologyProjectionInput> {
        let mut tx = self.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let watermark = watermark(&mut tx, subject, "topology", "").await?;
        let mut nodes = HashSet::new();
        let mut edges = Vec::new();
        if memory_enabled {
            let sources = crate::projection_input::memory_sources(&mut tx, subject).await?;
            for source in sources {
                nodes.insert(source.reference.clone());
                for tag in source.tag_ids {
                    let tag = CognitiveRef::Tag(TagId(
                        tag.parse()
                            .map_err(|_| Error::Infrastructure("invalid Tag id".into()))?,
                    ));
                    nodes.insert(tag.clone());
                    edges.push(link(
                        source.reference.clone(),
                        tag.clone(),
                        "tag_attachment",
                    ));
                    edges.push(link(tag, source.reference.clone(), "tag_attachment"));
                }
                for schema in source.schema_ids {
                    let schema = CognitiveRef::CognitiveSchema(CognitiveSchemaId(
                        schema
                            .parse()
                            .map_err(|_| Error::Infrastructure("invalid schema id".into()))?,
                    ));
                    nodes.insert(schema.clone());
                    edges.push(link(
                        source.reference.clone(),
                        schema.clone(),
                        "schema_support",
                    ));
                    edges.push(link(schema, source.reference.clone(), "schema_support"));
                }
                for entity in source.entity_refs {
                    let entity = CognitiveRef::Entity(EntityRef::new(entity)?);
                    nodes.insert(entity.clone());
                    edges.push(link(source.reference.clone(), entity.clone(), "aboutness"));
                    edges.push(link(entity, source.reference.clone(), "aboutness"));
                }
            }
            let rows = sqlx::query(
                "SELECT from_ref_kind,from_ref,to_ref_kind,to_ref,support_class,relation_kind,polarity FROM association_evidence WHERE subject_id=$1 AND revoked_at IS NULL",
            )
            .bind(subject.0)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for row in rows {
                let from = parse_reference(
                    &row.try_get::<String, _>("from_ref_kind").map_err(db)?,
                    &row.try_get::<String, _>("from_ref").map_err(db)?,
                )?;
                let to = parse_reference(
                    &row.try_get::<String, _>("to_ref_kind").map_err(db)?,
                    &row.try_get::<String, _>("to_ref").map_err(db)?,
                )?;
                if !allowed(&from) || !allowed(&to) {
                    continue;
                }
                nodes.insert(from.clone());
                nodes.insert(to.clone());
                edges.push(TopologyEdgeSource {
                    from,
                    to,
                    support_class: row.try_get("support_class").map_err(db)?,
                    association_kind: row.try_get("relation_kind").map_err(db)?,
                    polarity: row.try_get("polarity").map_err(db)?,
                    support_mass: 1.0,
                });
            }
            let rows = sqlx::query(
                "SELECT rr.from_revision_id,rr.to_revision_id,rr.relation FROM memory_revision_relations rr JOIN memory_revisions l ON l.memory_revision_id=rr.from_revision_id JOIN memory_revisions r ON r.memory_revision_id=rr.to_revision_id JOIN memory_objects lo ON lo.memory_id=l.memory_id JOIN memory_objects ro ON ro.memory_id=r.memory_id WHERE lo.subject_id=$1 AND ro.subject_id=$1 AND lo.acceptance_state='accepted' AND ro.acceptance_state='accepted'",
            )
            .bind(subject.0)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for row in rows {
                edges.push(link(
                    CognitiveRef::MemoryRevision(MemoryRevisionId(
                        row.try_get("from_revision_id").map_err(db)?,
                    )),
                    CognitiveRef::MemoryRevision(MemoryRevisionId(
                        row.try_get("to_revision_id").map_err(db)?,
                    )),
                    &row.try_get::<String, _>("relation").map_err(db)?,
                ));
            }
        }
        for resource in sqlx::query_scalar::<_, String>(
            "SELECT resource_ref FROM resources WHERE subject_id=$1",
        )
        .bind(subject.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        {
            nodes.insert(CognitiveRef::Resource(ResourceRef::new(resource)?));
        }
        let mut nodes: Vec<_> = nodes.into_iter().collect();
        nodes.sort_by_key(ToString::to_string);
        tx.commit().await.map_err(db)?;
        Ok(TopologyProjectionInput {
            watermark,
            nodes,
            edges,
        })
    }
}

fn allowed(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::Memory(_)
            | CognitiveRef::MemoryRevision(_)
            | CognitiveRef::CognitiveSchema(_)
            | CognitiveRef::CognitiveSchemaRevision(_)
            | CognitiveRef::Tag(_)
            | CognitiveRef::Entity(_)
            | CognitiveRef::Resource(_)
    )
}
