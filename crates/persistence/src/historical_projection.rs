//! Disposable projection inputs selected by semantic owners before candidate generation.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
use crate::*;
use nous_core::*;
use sqlx::Row;
use std::collections::{HashMap, HashSet};

pub struct HistoricalProjectionInput {
    pub sources: Vec<TextProjectionSource>,
    pub evidence_roots: HashMap<CognitiveRef, HashSet<String>>,
    pub concepts: ConceptProjectionInput,
    pub topology: TopologyProjectionInput,
}

impl AuthorityStore {
    #[expect(
        clippy::too_many_lines,
        reason = "one repeatable-read projection assembles only owner-selected historical identities and their immutable content"
    )]
    pub async fn historical_projection_input(
        &self,
        view: &HistoricalAuthoritySnapshot,
        budget: EpisodeTextBudget,
    ) -> Result<HistoricalProjectionInput> {
        let mut tx = self.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        let memory_allowed: bool =
            sqlx::query_scalar("SELECT memory FROM subject_capabilities WHERE subject_id=$1")
                .bind(view.subject.0)
                .fetch_one(&mut *tx)
                .await
                .map_err(database_error)?;
        // Selection is frozen at the owner level. Current purge is a hard fence,
        // while current head, suppression and canonical Tag never select this corpus.
        let selected: Vec<_> = view
            .cognition
            .iter()
            .filter(|_| memory_allowed)
            .filter(|state| {
                state.state["acceptance_state"] == "accepted"
                    && state.state["integrity_state"] == "valid"
                    && state.state["suppression_state"] == "normal"
                    && state.state["purge_state"] == "normal"
            })
            .flat_map(|state| state.revisions.iter().cloned())
            .collect();
        let (kinds, values): (Vec<_>, Vec<_>) = selected
            .iter()
            .chain(view.material_documents.iter())
            .map(reference_parts)
            .unzip();
        let rows = sqlx::query(r#"
WITH selected AS (SELECT * FROM unnest($2::text[],$3::text[]) r(kind,value)), catalog AS (
 SELECT s.kind,s.value,r.title,r.representation_text text,NULL::text hash,'text/plain'::text media,NULL::text source_class FROM selected s JOIN memory_revisions r ON s.kind='memory_revision' AND r.memory_revision_id::text=s.value JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND o.purge_state='normal'
 UNION ALL SELECT s.kind,s.value,r.title,r.structural_claim,NULL,'text/plain',NULL FROM selected s JOIN cognitive_schema_revisions r ON s.kind='cognitive_schema_revision' AND r.schema_revision_id::text=s.value JOIN cognitive_schemas o USING(schema_id) WHERE o.subject_id=$1 AND o.purge_state='normal'
 UNION ALL SELECT s.kind,s.value,r.title,COALESCE(r.title,'')||E'\n'||r.boundary_explanation||E'\nExperience '||r.experience_time_kind||' '||COALESCE(r.experience_time_start::text,'')||' '||COALESCE(r.experience_time_end::text,''),NULL,'text/plain',NULL FROM selected s JOIN episode_revisions r ON s.kind='episode_revision' AND r.episode_revision_id::text=s.value JOIN episode_objects o USING(episode_id) WHERE r.subject_id=$1 AND o.purge_state='normal'
 UNION ALL SELECT s.kind,s.value,r.title,COALESCE(r.title,'')||E'\n'||r.narrative||E'\n'||COALESCE((SELECT string_agg(text,E'\n' ORDER BY ordinal) FROM journal_revision_points WHERE journal_revision_id=r.journal_revision_id),''),NULL,'text/plain',NULL FROM selected s JOIN journal_revisions r ON s.kind='journal_revision' AND r.journal_revision_id::text=s.value JOIN journal_objects o USING(journal_id) WHERE r.subject_id=$1 AND o.purge_state='normal'
 UNION ALL SELECT s.kind,s.value,NULL,NULL,a.content_hash,a.media_type,o.source_class FROM selected s JOIN observation_occurrences o ON s.kind='occurrence' AND o.occurrence_id::text=s.value JOIN artifacts a USING(artifact_id) WHERE o.subject_id=$1
 UNION ALL SELECT s.kind,s.value,NULL,NULL,a.content_hash,a.media_type,NULL FROM selected s JOIN source_regions r ON s.kind='source_region' AND r.source_region_id::text=s.value JOIN artifacts a USING(artifact_id) WHERE r.subject_id=$1
 UNION ALL SELECT s.kind,s.value,d.representation_kind,d.payload_text,a.content_hash,CASE WHEN d.payload_text IS NOT NULL THEN 'text/plain' ELSE COALESCE(a.media_type,'application/octet-stream') END,'derived' FROM selected s JOIN derived_representations d ON s.kind='derived_representation' AND d.derived_representation_id::text=s.value LEFT JOIN artifacts a ON a.artifact_id=d.payload_artifact_id WHERE d.subject_id=$1
 UNION ALL SELECT s.kind,s.value,d.representation_kind,d.payload_text,a.content_hash,CASE WHEN d.payload_text IS NOT NULL THEN 'text/plain' ELSE COALESCE(a.media_type,'application/octet-stream') END,'derived' FROM selected s JOIN derived_regions r ON s.kind='derived_region' AND r.derived_region_id::text=s.value JOIN derived_representations d USING(derived_representation_id) LEFT JOIN artifacts a ON a.artifact_id=d.payload_artifact_id WHERE r.subject_id=$1
) SELECT * FROM catalog ORDER BY kind,value
"#).bind(view.subject.0).bind(kinds).bind(values).fetch_all(&mut *tx).await.map_err(database_error)?;
        let mut sources = Vec::new();
        for row in rows {
            let reference = parse_reference(
                &row.get::<String, _>("kind"),
                &row.get::<String, _>("value"),
            )?;
            sources.push(TextProjectionSource {
                revision: selected.contains(&reference).then(|| reference.clone()),
                reference: reference.clone(),
                text: row.get("text"),
                content_hash: row.get("hash"),
                title: row.get("title"),
                media_type: row.get("media"),
                source_class: row.get("source_class"),
                member_fragments: vec![],
                source_region: match reference {
                    CognitiveRef::SourceRegion(id) => Some(id),
                    _ => None,
                },
                entity_refs: vec![],
                tag_ids: vec![],
                schema_ids: vec![],
            });
        }
        let mut tags: Vec<_> = view
            .tags
            .iter()
            .filter(|tag| memory_allowed && tag.status == "active")
            .map(|tag| ConceptProjectionTag {
                tag: tag.tag,
                revision: tag.revision_id,
                semantic: tag.semantic.clone(),
                attachments: vec![],
            })
            .collect();
        for tag in &tags {
            sources.push(TextProjectionSource {
                reference: CognitiveRef::Tag(tag.tag),
                revision: None,
                text: Some(tag.semantic.text.clone()),
                content_hash: None,
                member_fragments: vec![],
                title: None,
                media_type: "text/plain".into(),
                source_class: None,
                source_region: None,
                entity_refs: vec![],
                tag_ids: vec![],
                schema_ids: vec![],
            });
        }
        let indices: HashMap<_, _> = sources
            .iter()
            .enumerate()
            .map(|(i, s)| (s.reference.clone(), i))
            .collect();
        let memory_ids: Vec<_> = sources
            .iter()
            .filter_map(|s| match s.reference {
                CognitiveRef::MemoryRevision(id) => Some(id.0),
                _ => None,
            })
            .collect();
        let schema_ids: Vec<_> = sources
            .iter()
            .filter_map(|s| match s.reference {
                CognitiveRef::CognitiveSchemaRevision(id) => Some(id.0),
                _ => None,
            })
            .collect();
        let rows = sqlx::query(r#"
 SELECT 'memory_revision' kind,t.memory_revision_id::text value,t.tag_id FROM memory_revision_tags t WHERE t.memory_revision_id=ANY($1::uuid[])
 UNION ALL SELECT 'cognitive_schema_revision',r.schema_revision_id::text,unnest(r.tags) FROM cognitive_schema_revisions r WHERE r.schema_revision_id=ANY($2::uuid[])
"#).bind(&memory_ids).bind(schema_ids).fetch_all(&mut *tx).await.map_err(database_error)?;
        for row in rows {
            let reference = parse_reference(
                &row.get::<String, _>("kind"),
                &row.get::<String, _>("value"),
            )?;
            if let Some(tag) = view.canonical_tag(TagId(row.get("tag_id")))
                && let Some(&index) = indices.get(&reference)
            {
                sources[index].tag_ids.push(tag.0.to_string());
            }
        }
        let rows = sqlx::query("SELECT memory_revision_id,entity_ref FROM memory_revision_aboutness WHERE memory_revision_id=ANY($1::uuid[]) ORDER BY memory_revision_id,entity_ref")
            .bind(memory_ids).fetch_all(&mut *tx).await.map_err(database_error)?;
        for row in rows {
            if let Some(&index) = indices.get(&CognitiveRef::MemoryRevision(MemoryRevisionId(
                row.get("memory_revision_id"),
            ))) {
                sources[index].entity_refs.push(row.get("entity_ref"));
            }
        }
        let bindings: Vec<_> = view.entity_bindings.to_vec();
        let rows = sqlx::query("SELECT m.occurrence_id,b.entity_ref FROM entity_binding_revisions b JOIN entity_mentions m USING(mention_id) WHERE b.binding_revision_id=ANY($1::uuid[]) AND b.binding_state='bound' AND b.entity_ref IS NOT NULL ORDER BY m.occurrence_id,b.entity_ref")
            .bind(bindings).fetch_all(&mut *tx).await.map_err(database_error)?;
        for row in rows {
            if let Some(&index) = indices.get(&CognitiveRef::Occurrence(OccurrenceId(
                row.get("occurrence_id"),
            ))) {
                sources[index].entity_refs.push(row.get("entity_ref"));
            }
        }
        let mut nodes: HashSet<_> = sources.iter().map(|s| s.reference.clone()).collect();
        let mut edges = Vec::new();
        for source in &sources {
            for tag in &source.tag_ids {
                let target = CognitiveRef::Tag(TagId(
                    tag.parse()
                        .map_err(|_| Error::Infrastructure("invalid concept identity".into()))?,
                ));
                historical_pair(
                    &mut edges,
                    source.reference.clone(),
                    target,
                    "tag_attachment",
                    "derived_structure",
                    "positive",
                    format!("structure:tag:{}:{tag}", source.reference),
                );
            }
            for entity in &source.entity_refs {
                let target = CognitiveRef::Entity(EntityRef::new(entity.clone())?);
                nodes.insert(target.clone());
                historical_pair(
                    &mut edges,
                    source.reference.clone(),
                    target,
                    "aboutness",
                    "derived_structure",
                    "positive",
                    format!("structure:aboutness:{}:{entity}", source.reference),
                );
            }
        }
        let associations: Vec<_> = view
            .associations
            .iter()
            .filter(|_| memory_allowed)
            .map(|id| id.0)
            .collect();
        let rows = sqlx::query("SELECT * FROM association_evidence WHERE subject_id=$1 AND association_evidence_id=ANY($2::uuid[]) ORDER BY association_evidence_id")
            .bind(view.subject.0).bind(associations).fetch_all(&mut *tx).await.map_err(database_error)?;
        for row in rows {
            let from = historical_endpoint(
                view,
                &row.get::<String, _>("from_ref_kind"),
                &row.get::<String, _>("from_ref"),
            )?;
            let to = historical_endpoint(
                view,
                &row.get::<String, _>("to_ref_kind"),
                &row.get::<String, _>("to_ref"),
            )?;
            let (Some(from), Some(to)) = (from, to) else {
                continue;
            };
            if !nodes.contains(&from) || !nodes.contains(&to) || from == to {
                continue;
            }
            let relation: String = row.get("relation_kind");
            let polarity: String = row.get("polarity");
            if relation == "tag_attachment"
                && polarity == "positive"
                && let CognitiveRef::Tag(tag) = to
                && let Some(&index) = indices.get(&from)
            {
                sources[index].tag_ids.push(tag.0.to_string());
            }
            historical_pair(
                &mut edges,
                from,
                to,
                &relation,
                &row.get::<String, _>("support_class"),
                &polarity,
                format!(
                    "association:{}",
                    row.get::<uuid::Uuid, _>("association_evidence_id")
                ),
            );
        }
        let links: Vec<_> = if memory_allowed {
            view.schema_evidence_links.clone()
        } else {
            vec![]
        };
        let rows = sqlx::query("SELECT schema_revision_id,support_kind,support_ref FROM cognitive_schema_evidence_links WHERE subject_id=$1 AND link_id=ANY($2::uuid[]) AND role='support' AND support_role<>'contradiction' AND support_kind<>'evidence'")
            .bind(view.subject.0).bind(links).fetch_all(&mut *tx).await.map_err(database_error)?;
        for row in rows {
            let from = CognitiveRef::CognitiveSchemaRevision(CognitiveSchemaRevisionId(
                row.get("schema_revision_id"),
            ));
            let to = parse_reference(
                &row.get::<String, _>("support_kind"),
                &row.get::<String, _>("support_ref"),
            )?;
            if nodes.contains(&from) && nodes.contains(&to) {
                if let Some(&index) = indices.get(&to)
                    && let Some(state) = view.cognition_for(&from)
                {
                    sources[index]
                        .schema_ids
                        .push(reference_parts(&state.object).1);
                }
                historical_pair(
                    &mut edges,
                    from.clone(),
                    to.clone(),
                    "schema_support",
                    "derived_structure",
                    "positive",
                    format!("structure:schema:{from}:{to}"),
                );
            }
        }
        let (selected_kinds, selected_values): (Vec<_>, Vec<_>) = sources
            .iter()
            .map(|s| reference_parts(&s.reference))
            .unzip();
        let rows=sqlx::query(r#"
WITH selected AS (SELECT * FROM unnest($2::text[],$3::text[]) r(kind,value)), links AS (
 SELECT 'memory_revision' from_kind,d.memory_revision_id::text from_value,d.target_ref_kind to_kind,d.target_ref to_value FROM memory_revision_dependencies d JOIN selected s ON s.kind='memory_revision' AND s.value=d.memory_revision_id::text WHERE d.support_role<>'contradiction'
 UNION ALL SELECT 'journal_revision',d.journal_revision_id::text,d.ref_kind,d.ref_value FROM journal_revision_sources d JOIN selected s ON s.kind='journal_revision' AND s.value=d.journal_revision_id::text
 UNION ALL SELECT 'episode_revision',d.episode_revision_id::text,d.ref_kind,d.ref_value FROM episode_revision_members d JOIN selected s ON s.kind='episode_revision' AND s.value=d.episode_revision_id::text
 UNION ALL SELECT 'episode_revision',d.episode_revision_id::text,d.support_kind,d.support_ref FROM episode_revision_supports d JOIN selected s ON s.kind='episode_revision' AND s.value=d.episode_revision_id::text WHERE d.support_kind<>'evidence' AND d.support_role<>'contradiction'
) SELECT * FROM links WHERE $1::uuid IS NOT NULL ORDER BY from_kind,from_value,to_kind,to_value
"#).bind(view.subject.0).bind(selected_kinds).bind(selected_values).fetch_all(&mut *tx).await.map_err(database_error)?;
        for row in rows {
            let from = parse_reference(
                &row.get::<String, _>("from_kind"),
                &row.get::<String, _>("from_value"),
            )?;
            let to = parse_reference(
                &row.get::<String, _>("to_kind"),
                &row.get::<String, _>("to_value"),
            )?;
            if nodes.contains(&from) && nodes.contains(&to) && from != to {
                historical_pair(
                    &mut edges,
                    from.clone(),
                    to.clone(),
                    "cognition_support",
                    "derived_structure",
                    "positive",
                    format!("structure:cognition-support:{from}:{to}"),
                );
            }
        }
        for source in &mut sources {
            source.tag_ids.sort();
            source.tag_ids.dedup();
            for tag in &source.tag_ids {
                if let Some(concept) = tags.iter_mut().find(|t| t.tag.0.to_string() == *tag) {
                    concept.attachments.push(source.reference.clone());
                }
            }
        }
        for tag in &mut tags {
            tag.attachments.sort_by_key(ToString::to_string);
            tag.attachments.dedup();
        }
        sources.sort_by_key(|source| source.reference.to_string());
        // Episode fragment selection is likewise bounded by the same effective Material set.
        let episode_ids = sources
            .iter()
            .filter_map(|s| match s.reference {
                CognitiveRef::EpisodeRevision(id) => Some(id.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut fragments = crate::episode_text::episode_member_text_input_in_view(
            &mut tx,
            view.subject,
            &episode_ids,
            budget,
            Some(view),
        )
        .await?;
        for source in &mut sources {
            if let CognitiveRef::EpisodeRevision(id) = source.reference {
                source.member_fragments = fragments.remove(&id.0).unwrap_or_default();
            }
        }
        let mut evidence_roots = HashMap::new();
        for source in &sources {
            let (kind, value) = reference_parts(&source.reference);
            evidence_roots.insert(
                source.reference.clone(),
                crate::topology_input::revision_roots_in_view(
                    &mut tx,
                    view.subject,
                    &kind,
                    &value,
                    &mut HashSet::new(),
                    Some(view),
                )
                .await?,
            );
        }
        tx.commit().await.map_err(database_error)?;
        Ok(HistoricalProjectionInput {
            sources,
            evidence_roots,
            concepts: ConceptProjectionInput { watermark: 0, tags },
            topology: TopologyProjectionInput {
                watermark: 0,
                nodes: {
                    let mut nodes: Vec<_> = nodes.into_iter().collect();
                    nodes.sort_by_key(ToString::to_string);
                    nodes
                },
                edges,
            },
        })
    }
}
fn historical_endpoint(
    view: &HistoricalAuthoritySnapshot,
    kind: &str,
    value: &str,
) -> Result<Option<CognitiveRef>> {
    let reference = parse_reference(kind, value)?;
    Ok(match reference {
        CognitiveRef::Tag(tag) => view.canonical_tag(tag).map(CognitiveRef::Tag),
        _ => Some(
            view.cognition_for(&reference)
                .filter(|state| state.object == reference)
                .map_or(reference.clone(), |state| state.head.clone()),
        ),
    })
}
fn historical_pair(
    edges: &mut Vec<TopologyEdgeSource>,
    from: CognitiveRef,
    to: CognitiveRef,
    kind: &str,
    class: &str,
    polarity: &str,
    root: String,
) {
    for (from, to) in [(from.clone(), to.clone()), (to, from)] {
        edges.push(TopologyEdgeSource {
            from,
            to,
            support_class: class.into(),
            association_kind: kind.into(),
            polarity: polarity.into(),
            support_mass: 1.0,
            provenance_root: Some(root.clone()),
        });
    }
}
