use super::*;
use nous_runtime::BoundQuery;
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn materialize_longitudinal(
    service: &MemoryService,
    bound: &BoundQuery,
    references: &[CognitiveRef],
) -> Result<(Vec<CognitiveHit>, BTreeMap<String, usize>)> {
    let subject = bound.source_query.subject;
    let mut objects = BTreeMap::<&str, Vec<Uuid>>::new();
    let mut revisions = BTreeMap::<&str, Vec<Uuid>>::new();
    for reference in references {
        match reference {
            CognitiveRef::Episode(id) => objects.entry("episode").or_default().push(id.0),
            CognitiveRef::Journal(id) => objects.entry("journal").or_default().push(id.0),
            CognitiveRef::EpisodeRevision(id) => revisions.entry("episode").or_default().push(id.0),
            CognitiveRef::JournalRevision(id) => revisions.entry("journal").or_default().push(id.0),
            _ => (),
        }
    }
    let mut hits = Vec::new();
    let mut drops = BTreeMap::new();
    for (kind, query) in [
        (
            "episode",
            "SELECT o.episode_id AS object_id,o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state,r.episode_revision_id AS revision_id,r.title,r.boundary_explanation AS text,r.experience_time_kind AS time_kind,r.experience_time_start AS time_start,r.experience_time_end AS time_end,r.formed_at,r.recorded_at FROM episode_objects o JOIN episode_revisions r USING(episode_id) WHERE o.subject_id=$1 AND ((o.episode_id=ANY($2::uuid[]) AND r.episode_revision_id=o.current_revision_id) OR r.episode_revision_id=ANY($3::uuid[]))",
        ),
        (
            "journal",
            "SELECT o.journal_id AS object_id,o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state,r.journal_revision_id AS revision_id,r.title,r.narrative AS text,r.temporal_scope_kind AS time_kind,r.temporal_scope_start AS time_start,r.temporal_scope_end AS time_end,r.formed_at,r.recorded_at FROM journal_objects o JOIN journal_revisions r USING(journal_id) WHERE o.subject_id=$1 AND ((o.journal_id=ANY($2::uuid[]) AND r.journal_revision_id=o.current_revision_id) OR r.journal_revision_id=ANY($3::uuid[]))",
        ),
    ] {
        let object_ids = objects.remove(kind).unwrap_or_default();
        let revision_ids = revisions.remove(kind).unwrap_or_default();
        if object_ids.is_empty() && revision_ids.is_empty() {
            continue;
        }
        let rows = sqlx::query(query)
            .bind(subject.0)
            .bind(&object_ids)
            .bind(&revision_ids)
            .fetch_all(service.store.pool())
            .await
            .map_err(db)?;
        let ids: Vec<Uuid> = rows.iter().map(|row| row.get("revision_id")).collect();
        let evidence = longitudinal_evidence(service, kind, &ids).await?;
        let mut renderings = longitudinal_renderings(service, kind, &ids).await?;
        let mut metadata = longitudinal_metadata(service, subject, kind, &ids).await?;
        for row in rows {
            let revision: Uuid = row.get("revision_id");
            let object: Uuid = row.get("object_id");
            let exact = if kind == "episode" {
                CognitiveRef::EpisodeRevision(EpisodeRevisionId(revision))
            } else {
                CognitiveRef::JournalRevision(JournalRevisionId(revision))
            };
            let mutable = if kind == "episode" {
                CognitiveRef::Episode(EpisodeId(object))
            } else {
                CognitiveRef::Journal(JournalId(object))
            };
            let requested = if references.contains(&exact) {
                &exact
            } else {
                &mutable
            };
            let historical = bound.revision_policy.allows_historical(requested)
                && row.get::<Uuid, _>("current_revision_id") != revision;
            let changed_binding = bound.exact_bindings.iter().any(|binding| {
                binding.bound_ref == exact
                    && binding.mutable_object
                    && (binding.bound_object_epoch != Some(row.get("object_epoch"))
                        || row.get::<Uuid, _>("current_revision_id") != revision)
            });
            if changed_binding
                || row.get::<String, _>("purge_state") != "normal"
                || (!historical
                    && (row.get::<Uuid, _>("current_revision_id") != revision
                        || row.get::<String, _>("acceptance_state") != "accepted"
                        || row.get::<String, _>("integrity_state") != "valid"
                        || (row.get::<String, _>("suppression_state") != "normal"
                            && !bound.source_query.expression.constraints.include_suppressed)))
            {
                *drops.entry("longitudinal_unavailable".into()).or_default() += 1;
                continue;
            }
            let time = temporal_from_columns(
                row.get("time_kind"),
                row.get("time_start"),
                row.get("time_end"),
            )?;
            let metadata = metadata.remove(&revision).unwrap_or_default();
            if !longitudinal_filter(&bound.source_query, &row, &time, kind, &metadata) {
                *drops.entry("longitudinal_constraints".into()).or_default() += 1;
                continue;
            }
            let supports = if bound.source_query.result_need.need_evidence {
                evidence.get(&revision).cloned().unwrap_or_default()
            } else {
                vec![]
            };
            hits.push(longitudinal_hit(
                &row,
                kind,
                exact,
                supports,
                renderings.remove(&revision).unwrap_or_default(),
                metadata,
            ));
        }
    }
    Ok((hits, drops))
}
fn longitudinal_hit(
    row: &sqlx::postgres::PgRow,
    kind: &str,
    exact: CognitiveRef,
    evidence: Vec<EvidenceHandle>,
    rendering: String,
    metadata: LongitudinalMetadata,
) -> CognitiveHit {
    let mut text = format!(
        "{}\n{}\n{}",
        row.get::<Option<String>, _>("title").unwrap_or_default(),
        row.get::<String, _>("text"),
        rendering
    );
    if text.len() > 65536 {
        let mut end = 65536;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    CognitiveHit {
        authority_epoch: Some(row.get("object_epoch")),
        preference_refs: vec![],
        reference: exact.clone(),
        revision: Some(exact),
        semantic_role: Some(kind.into()),
        cognitive_role: None,
        formation_mode: None,
        representation: Some(text),
        authority: AuthorityClass::SubjectCognition,
        freshness: FreshnessDescriptor {
            occurred: metadata.occurred,
            observed_at: metadata.observed_at.or_else(|| {
                row.get::<Option<chrono::DateTime<Utc>>, _>("time_end")
                    .or_else(|| row.get("time_start"))
            }),
            valid_time: TemporalExtent::Unknown,
            formed_at: Some(row.get("formed_at")),
            recorded_at: Some(row.get("recorded_at")),
        },
        entity_refs: metadata.entities,
        evidence,
        match_evidence: MatchEvidence::default(),
        materialization: vec![],
    }
}

fn longitudinal_filter(
    query: &CognitiveQuery,
    row: &sqlx::postgres::PgRow,
    time: &TemporalExtent,
    kind: &str,
    metadata: &LongitudinalMetadata,
) -> bool {
    let constraints = &query.expression.constraints;
    constraints
        .authority
        .is_none_or(|authority| authority == AuthorityClass::SubjectCognition)
        && constraints.cognitive_roles_include.is_empty()
        && constraints.formation_modes_include.is_empty()
        && constraints
            .entity_requirements
            .iter()
            .all(|entity| metadata.entities.contains(entity))
        && constraints.valid.is_none()
        && constraints.occurred.is_none_or(|interval| {
            metadata
                .occurred
                .iter()
                .any(|extent| extent.overlaps_interval(&interval))
        })
        && (constraints.source_classes_include.is_empty()
            || constraints
                .source_classes_include
                .iter()
                .any(|class| metadata.source_classes.contains(class)))
        && !constraints
            .source_classes_exclude
            .iter()
            .any(|class| metadata.source_classes.contains(class))
        && (constraints.modalities.is_empty() || constraints.modalities.contains(&Modality::Text))
        && (constraints.evidence_classes.is_empty()
            || constraints.evidence_classes.iter().any(|class| {
                class
                    == if kind == "journal" {
                        "narrative"
                    } else {
                        "derived"
                    }
            }))
        && constraints
            .observed
            .is_none_or(|interval| time.overlaps_interval(&interval))
        && constraints
            .formed
            .is_none_or(|interval| interval.contains(row.get("formed_at")))
        && constraints
            .recorded
            .is_none_or(|interval| interval.contains(row.get("recorded_at")))
}
async fn longitudinal_evidence(
    service: &MemoryService,
    kind: &str,
    ids: &[Uuid],
) -> Result<BTreeMap<Uuid, Vec<EvidenceHandle>>> {
    let mut result = BTreeMap::<Uuid, Vec<EvidenceHandle>>::new();
    if kind == "episode" {
        let rows=sqlx::query("SELECT episode_revision_id,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id FROM episode_revision_supports WHERE episode_revision_id=ANY($1::uuid[]) ORDER BY episode_revision_id,support_no")
            .bind(ids).fetch_all(service.store.pool()).await.map_err(db)?;
        for row in rows {
            let reference = if row.get::<String, _>("support_kind") == "evidence" {
                if let Some(id) = row.get::<Option<Uuid>, _>("derived_region_id") {
                    CognitiveRef::DerivedRegion(DerivedRegionId(id))
                } else if let Some(id) = row.get::<Option<Uuid>, _>("derived_representation_id") {
                    CognitiveRef::DerivedRepresentation(DerivedRepresentationId(id))
                } else if let Some(id) = row.get::<Option<Uuid>, _>("source_region_id") {
                    CognitiveRef::SourceRegion(SourceRegionId(id))
                } else {
                    CognitiveRef::Occurrence(OccurrenceId(row.get("occurrence_id")))
                }
            } else {
                parse_reference(
                    &row.get::<String, _>("support_kind"),
                    &row.get::<String, _>("support_ref"),
                )?
            };
            result
                .entry(row.get("episode_revision_id"))
                .or_default()
                .push(EvidenceHandle {
                    reference,
                    support_role: row.get("support_role"),
                });
        }
    } else {
        let rows=sqlx::query("SELECT journal_revision_id,support FROM journal_point_supports WHERE journal_revision_id=ANY($1::uuid[]) ORDER BY journal_revision_id,ordinal,support_no")
            .bind(ids).fetch_all(service.store.pool()).await.map_err(db)?;
        let mut seen = BTreeSet::new();
        for row in rows {
            let revision: Uuid = row.get("journal_revision_id");
            let support: RevisionSupport = serde_json::from_value(row.get("support"))
                .map_err(|error| Error::Infrastructure(error.to_string()))?;
            if !seen.insert((revision, support.canonical_key())) {
                continue;
            }
            let (reference, role) = match support {
                RevisionSupport::Evidence(evidence) => {
                    (evidence.cognitive_ref(), evidence.support_role)
                }
                RevisionSupport::CognitionDependency(dependency) => {
                    (dependency.target_revision, dependency.support_role)
                }
                RevisionSupport::Seed(_) => {
                    return Err(Error::Infrastructure(
                        "Journal has invalid Seed support".into(),
                    ));
                }
            };
            result.entry(revision).or_default().push(EvidenceHandle {
                reference,
                support_role: role.as_str().into(),
            });
        }
    }
    Ok(result)
}

async fn longitudinal_renderings(
    service: &MemoryService,
    kind: &str,
    ids: &[Uuid],
) -> Result<BTreeMap<Uuid, String>> {
    let mut result = BTreeMap::<Uuid, String>::new();
    if kind == "journal" {
        let rows = sqlx::query("SELECT journal_revision_id,text FROM journal_revision_points WHERE journal_revision_id=ANY($1::uuid[]) ORDER BY journal_revision_id,ordinal")
            .bind(ids).fetch_all(service.store.pool()).await.map_err(db)?;
        for row in rows {
            append_rendering(
                result.entry(row.get("journal_revision_id")).or_default(),
                &row.get::<String, _>("text"),
            );
        }
    } else {
        let rows = sqlx::query("SELECT m.episode_revision_id,a.content_hash,a.byte_length FROM episode_revision_members m JOIN observation_occurrences o ON m.ref_kind='occurrence' AND m.ref_value=o.occurrence_id::text JOIN artifacts a USING(artifact_id) WHERE m.episode_revision_id=ANY($1::uuid[]) AND m.ordinal<16 AND (a.media_type LIKE 'text/%' OR a.media_type='application/json') ORDER BY m.episode_revision_id,m.ordinal")
            .bind(ids).fetch_all(service.store.pool()).await.map_err(db)?;
        for row in rows {
            let bytes = service
                .objects
                .read_range(
                    &row.get::<String, _>("content_hash"),
                    0,
                    (row.get::<i64, _>("byte_length") as u64).min(2048),
                )
                .await?;
            let text = match std::str::from_utf8(&bytes) {
                Ok(text) => text,
                Err(error) if error.error_len().is_none() => {
                    std::str::from_utf8(&bytes[..error.valid_up_to()])
                        .map_err(|error| Error::Invalid(error.to_string()))?
                }
                Err(_) => continue,
            };
            append_rendering(
                result.entry(row.get("episode_revision_id")).or_default(),
                text,
            );
        }
    }
    Ok(result)
}

fn append_rendering(output: &mut String, text: &str) {
    let remaining = 65536_usize.saturating_sub(output.len() + 1);
    if remaining == 0 {
        return;
    }
    let mut end = text.len().min(remaining);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    output.push('\n');
    output.push_str(&text[..end]);
}

#[derive(Default)]
struct LongitudinalMetadata {
    entities: Vec<EntityRef>,
    source_classes: Vec<SourceClass>,
    occurred: Vec<TemporalExtent>,
    observed_at: Option<chrono::DateTime<Utc>>,
}

async fn longitudinal_metadata(
    service: &MemoryService,
    subject: SubjectId,
    kind: &str,
    ids: &[Uuid],
) -> Result<BTreeMap<Uuid, LongitudinalMetadata>> {
    let rows = sqlx::query(r"
WITH RECURSIVE lineage(root,kind,value) AS (
    SELECT id,$3::text,id::text FROM unnest($2::uuid[]) id
    UNION
    SELECT l.root,e.kind,e.value FROM lineage l CROSS JOIN LATERAL (
        SELECT m.ref_kind AS kind,m.ref_value AS value FROM episode_revision_members m
        WHERE m.episode_revision_id=CASE WHEN l.kind='episode_revision' THEN l.value::uuid END
        UNION SELECT CASE WHEN s.support_kind='evidence' THEN 'occurrence' ELSE s.support_kind END,
            CASE WHEN s.support_kind='evidence' THEN s.occurrence_id::text ELSE s.support_ref END
        FROM episode_revision_supports s WHERE s.episode_revision_id=CASE WHEN l.kind='episode_revision' THEN l.value::uuid END
        UNION SELECT s.ref_kind,s.ref_value FROM journal_revision_sources s
        WHERE s.journal_revision_id=CASE WHEN l.kind='journal_revision' THEN l.value::uuid END
        UNION SELECT d.target_ref_kind,d.target_ref FROM memory_revision_dependencies d
        WHERE d.memory_revision_id=CASE WHEN l.kind='memory_revision' THEN l.value::uuid END
        UNION SELECT 'occurrence',e.occurrence_id::text FROM memory_revision_evidence e
        WHERE e.memory_revision_id=CASE WHEN l.kind='memory_revision' THEN l.value::uuid END
        UNION SELECT CASE WHEN e.support_kind='evidence' THEN 'occurrence' ELSE e.support_kind END,
            CASE WHEN e.support_kind='evidence' THEN e.occurrence_id::text ELSE e.support_ref END
        FROM cognitive_schema_evidence_links e WHERE e.schema_revision_id=CASE WHEN l.kind='cognitive_schema_revision' THEN l.value::uuid END AND e.revoked_at IS NULL
    ) e WHERE e.value IS NOT NULL
)
SELECT DISTINCT l.root,o.occurrence_id,o.source_class,o.actor_entity_ref,
    o.occurred_time_kind,o.occurred_time_start,o.occurred_time_end,o.observed_at,
    ARRAY(SELECT DISTINCT b.entity_ref FROM entity_mentions m CROSS JOIN LATERAL (
        SELECT entity_ref,binding_state FROM entity_binding_revisions WHERE mention_id=m.mention_id ORDER BY revision_no DESC LIMIT 1
    ) b WHERE m.subject_id=$1 AND m.occurrence_id=o.occurrence_id AND b.binding_state='bound' AND b.entity_ref IS NOT NULL) AS entities
FROM lineage l JOIN observation_occurrences o ON o.occurrence_id=CASE WHEN l.kind='occurrence' THEN l.value::uuid END
WHERE o.subject_id=$1 ORDER BY l.root,o.occurrence_id")
        .bind(subject.0).bind(ids).bind(format!("{kind}_revision"))
        .fetch_all(service.store.pool()).await.map_err(db)?;
    let mut result = BTreeMap::<Uuid, LongitudinalMetadata>::new();
    for row in rows {
        let value = result.entry(row.get("root")).or_default();
        let source = SourceClass::from(row.get::<String, _>("source_class"));
        if !value.source_classes.contains(&source) {
            value.source_classes.push(source);
        }
        let mut entities = row.get::<Vec<String>, _>("entities");
        entities.extend(row.get::<Option<String>, _>("actor_entity_ref"));
        for entity in entities {
            let entity = EntityRef::new(entity)?;
            if !value.entities.contains(&entity) {
                value.entities.push(entity);
            }
        }
        let occurred = temporal_from_columns(
            row.get("occurred_time_kind"),
            row.get("occurred_time_start"),
            row.get("occurred_time_end"),
        )?;
        if !value.occurred.contains(&occurred) {
            value.occurred.push(occurred);
        }
        let observed = row.get::<chrono::DateTime<Utc>, _>("observed_at");
        value.observed_at = Some(
            value
                .observed_at
                .map_or(observed, |current| current.max(observed)),
        );
    }
    Ok(result)
}
