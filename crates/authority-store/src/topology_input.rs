use crate::{AuthorityStore, database_error as db, projection_input::watermark};
use nous_core::*;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdgeSource {
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub support_class: String,
    pub association_kind: String,
    pub polarity: String,
    pub support_mass: f64,
    pub provenance_root: Option<String>,
}

pub struct TopologyProjectionInput {
    pub watermark: i64,
    pub nodes: Vec<CognitiveRef>,
    pub edges: Vec<TopologyEdgeSource>,
}

fn link(
    from: CognitiveRef,
    to: CognitiveRef,
    kind: &str,
    provenance_root: impl Into<Option<String>>,
) -> TopologyEdgeSource {
    TopologyEdgeSource {
        from,
        to,
        support_class: "derived_structure".into(),
        association_kind: kind.into(),
        polarity: "positive".into(),
        support_mass: 1.0,
        provenance_root: provenance_root.into(),
    }
}

impl AuthorityStore {
    #[expect(
        clippy::excessive_nesting,
        clippy::too_many_lines,
        reason = "topology projection assembles one repeatable-read Authority snapshot"
    )]
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
                    let root = format!("structure:tag:{}:{}", source.reference, tag);
                    edges.push(link(
                        source.reference.clone(),
                        tag.clone(),
                        "tag_attachment",
                        Some(root.clone()),
                    ));
                    edges.push(link(
                        tag,
                        source.reference.clone(),
                        "tag_attachment",
                        Some(root),
                    ));
                }
                for schema in source.schema_ids {
                    let schema_id: Uuid = schema
                        .parse()
                        .map_err(|_| Error::Infrastructure("invalid schema id".into()))?;
                    let revision = sqlx::query_scalar::<_, Uuid>(
                        "SELECT current_revision_id FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 AND acceptance_state='accepted' AND integrity_state='valid' AND suppression_state='normal' AND purge_state='normal'",
                    )
                    .bind(subject.0)
                    .bind(schema_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
                    let Some(revision) = revision else { continue };
                    let schema =
                        CognitiveRef::CognitiveSchemaRevision(CognitiveSchemaRevisionId(revision));
                    nodes.insert(schema.clone());
                    let root = format!("structure:schema:{}:{}", source.reference, schema);
                    edges.push(link(
                        source.reference.clone(),
                        schema.clone(),
                        "schema_support",
                        Some(root.clone()),
                    ));
                    edges.push(link(
                        schema,
                        source.reference.clone(),
                        "schema_support",
                        Some(root),
                    ));
                }
                for entity in source.entity_refs {
                    let entity = CognitiveRef::Entity(EntityRef::new(entity)?);
                    nodes.insert(entity.clone());
                    let root = format!("structure:aboutness:{}:{}", source.reference, entity);
                    edges.push(link(
                        source.reference.clone(),
                        entity.clone(),
                        "aboutness",
                        Some(root.clone()),
                    ));
                    edges.push(link(
                        entity,
                        source.reference.clone(),
                        "aboutness",
                        Some(root),
                    ));
                }
            }

            let association_rows = sqlx::query(
                "SELECT association_evidence_id,from_ref_kind,from_ref,to_ref_kind,to_ref,support_class,relation_kind,polarity FROM association_evidence WHERE subject_id=$1 AND revoked_at IS NULL",
            )
            .bind(subject.0)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for row in association_rows {
                let association_id: Uuid = row.try_get("association_evidence_id").map_err(db)?;
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
                let support_class: String = row.try_get("support_class").map_err(db)?;
                let relation_kind: String = row.try_get("relation_kind").map_err(db)?;
                let polarity: String = row.try_get("polarity").map_err(db)?;
                let supports = sqlx::query(
                    "SELECT support_kind,support_ref,occurrence_id FROM association_evidence_supports WHERE association_evidence_id=$1 ORDER BY support_kind,support_ref",
                )
                .bind(association_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                let mut roots = HashSet::new();
                for support in supports {
                    let kind: String = support.try_get("support_kind").map_err(db)?;
                    match kind.as_str() {
                        "evidence" => {
                            if let Some(occurrence) = support
                                .try_get::<Option<Uuid>, _>("occurrence_id")
                                .map_err(db)?
                            {
                                roots.extend(occurrence_roots(&mut tx, subject, occurrence).await?);
                            }
                        }
                        "memory_revision" | "cognitive_schema_revision" => {
                            roots.extend(
                                revision_roots(
                                    &mut tx,
                                    subject,
                                    &kind,
                                    &support.try_get::<String, _>("support_ref").map_err(db)?,
                                    &mut HashSet::new(),
                                )
                                .await?,
                            );
                        }
                        "use_event" => {
                            roots.insert(format!(
                                "use_event:{}",
                                support.try_get::<String, _>("support_ref").map_err(db)?
                            ));
                        }
                        _ => {}
                    }
                }
                if roots.is_empty() {
                    roots.insert(format!("unknown-association:{association_id}"));
                }
                for root in roots {
                    edges.push(TopologyEdgeSource {
                        from: from.clone(),
                        to: to.clone(),
                        support_class: support_class.clone(),
                        association_kind: relation_kind.clone(),
                        polarity: polarity.clone(),
                        support_mass: 1.0,
                        provenance_root: Some(root),
                    });
                }
            }

            let rows = sqlx::query(
                "SELECT rr.from_revision_id,rr.to_revision_id,rr.relation FROM memory_revision_relations rr JOIN memory_revisions l ON l.memory_revision_id=rr.from_revision_id JOIN memory_revisions r ON r.memory_revision_id=rr.to_revision_id JOIN memory_objects lo ON lo.memory_id=l.memory_id JOIN memory_objects ro ON ro.memory_id=r.memory_id WHERE lo.subject_id=$1 AND ro.subject_id=$1 AND lo.current_revision_id=l.memory_revision_id AND ro.current_revision_id=r.memory_revision_id AND lo.acceptance_state='accepted' AND ro.acceptance_state='accepted' AND lo.integrity_state='valid' AND ro.integrity_state='valid' AND lo.suppression_state='normal' AND ro.suppression_state='normal' AND lo.purge_state='normal' AND ro.purge_state='normal'",
            )
            .bind(subject.0)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for row in rows {
                let from = CognitiveRef::MemoryRevision(MemoryRevisionId(
                    row.try_get("from_revision_id").map_err(db)?,
                ));
                let to = CognitiveRef::MemoryRevision(MemoryRevisionId(
                    row.try_get("to_revision_id").map_err(db)?,
                ));
                nodes.insert(from.clone());
                nodes.insert(to.clone());
                let relation: String = row.try_get("relation").map_err(db)?;
                let root = format!("structure:relation:{from}:{to}:{relation}");
                edges.push(link(from, to, &relation, Some(root)));
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

async fn occurrence_roots(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    occurrence: Uuid,
) -> Result<HashSet<String>> {
    let row = sqlx::query(
        "SELECT source_class,external_object_ref,artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2",
    )
    .bind(subject.0)
    .bind(occurrence)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(|| Error::Invalid("association evidence occurrence is outside Subject".into()))?;
    let source_class: String = row.try_get("source_class").map_err(db)?;
    let external: Option<String> = row.try_get("external_object_ref").map_err(db)?;
    let artifact: Option<Uuid> = row.try_get("artifact_id").map_err(db)?;
    let root = if let Some(external) = external {
        format!("external:{source_class}:{external}")
    } else if let Some(artifact) = artifact {
        format!("artifact:{artifact}")
    } else {
        format!("occurrence:{occurrence}")
    };
    Ok(HashSet::from([root]))
}

#[expect(
    clippy::excessive_nesting,
    reason = "iterative provenance closure keeps exact support semantics together"
)]
async fn revision_roots(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    kind: &str,
    value: &str,
    visited: &mut HashSet<(String, String)>,
) -> Result<HashSet<String>> {
    let mut stack = vec![(kind.to_owned(), value.to_owned())];
    let mut roots = HashSet::new();
    while let Some((kind, value)) = stack.pop() {
        if !visited.insert((kind.clone(), value.clone())) {
            roots.insert(format!("unknown-dependency:{kind}:{value}"));
            continue;
        }
        let id = value
            .parse::<Uuid>()
            .map_err(|_| Error::Invalid("invalid revision support".into()))?;
        if kind == "memory_revision" {
            let rows = sqlx::query(
                "SELECT occurrence_id FROM memory_revision_evidence e JOIN memory_revisions r USING(memory_revision_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2",
            )
            .bind(subject.0)
            .bind(id)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for row in rows {
                roots.extend(
                    occurrence_roots(tx, subject, row.try_get("occurrence_id").map_err(db)?)
                        .await?,
                );
            }
            let rows = sqlx::query(
                "SELECT target_ref_kind,target_ref FROM memory_revision_dependencies d JOIN memory_revisions r USING(memory_revision_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2",
            )
            .bind(subject.0)
            .bind(id)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for row in rows {
                stack.push((
                    row.try_get("target_ref_kind").map_err(db)?,
                    row.try_get("target_ref").map_err(db)?,
                ));
            }
        } else if kind == "cognitive_schema_revision" {
            let rows = sqlx::query(
                "SELECT support_kind,support_ref,occurrence_id FROM cognitive_schema_evidence_links WHERE subject_id=$1 AND schema_revision_id=$2 AND revoked_at IS NULL",
            )
            .bind(subject.0)
            .bind(id)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for row in rows {
                let support_kind: String = row.try_get("support_kind").map_err(db)?;
                if support_kind == "evidence" {
                    if let Some(occurrence) = row
                        .try_get::<Option<Uuid>, _>("occurrence_id")
                        .map_err(db)?
                    {
                        roots.extend(occurrence_roots(tx, subject, occurrence).await?);
                    }
                } else {
                    stack.push((support_kind, row.try_get("support_ref").map_err(db)?));
                }
            }
        } else {
            roots.insert(format!("unknown-dependency:{kind}:{value}"));
        }
    }
    if roots.is_empty() {
        roots.insert(format!("unknown-dependency:{kind}:{value}"));
    }
    Ok(roots)
}

fn allowed(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::MemoryRevision(_)
            | CognitiveRef::CognitiveSchemaRevision(_)
            | CognitiveRef::Tag(_)
            | CognitiveRef::Entity(_)
            | CognitiveRef::Resource(_)
    )
}
