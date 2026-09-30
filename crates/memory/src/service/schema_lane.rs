use super::lane::{LaneCandidate, LaneOutput, LaneStatus};
use super::*;
use nous_runtime::{BoundQuery, QueryPlan};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;

#[expect(
    clippy::too_many_lines,
    reason = "schema direct lane batches schema and support projection semantics"
)]
pub(crate) async fn schema_direct_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    _plan: &QueryPlan,
) -> Result<LaneOutput> {
    let query = &bound.source_query;
    let mut schema_ids = BTreeSet::new();
    let mut exact_revisions = BTreeSet::new();
    for cue in &query.expression.cues {
        if let Cue::Schema(value) = cue {
            schema_ids.insert(value.schema.0);
        }
    }
    for target in &query.expression.targets {
        if let QueryTarget::SchemaNeighborhood { schema } = target {
            schema_ids.insert(schema.0);
        }
    }
    for binding in &bound.exact_bindings {
        match binding.bound_ref {
            CognitiveRef::CognitiveSchema(schema) => {
                schema_ids.insert(schema.0);
            }
            CognitiveRef::CognitiveSchemaRevision(revision) => {
                exact_revisions.insert(revision.0);
            }
            _ => {}
        }
    }
    let mut output = LaneOutput::empty(EvidenceFamily::SchemaDirect, LaneStatus::Ready);
    if schema_ids.is_empty() && exact_revisions.is_empty() {
        return Ok(output);
    }
    let rows = sqlx::query(
        "SELECT s.schema_id,s.current_revision_id,s.object_epoch,s.acceptance_state,s.integrity_state,s.suppression_state,s.purge_state,r.schema_revision_id,r.structural_claim,r.aboutness,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM cognitive_schemas s JOIN cognitive_schema_revisions r ON r.schema_id=s.schema_id WHERE s.subject_id=$1 AND (s.schema_id=ANY($2::uuid[]) OR r.schema_revision_id=ANY($3::uuid[]))",
    )
    .bind(query.subject.0)
    .bind(schema_ids.iter().copied().collect::<Vec<_>>())
    .bind(exact_revisions.iter().copied().collect::<Vec<_>>())
    .fetch_all(service.store.pool())
    .await
    .map_err(nous_persistence::database_error)?;
    let mut rows = rows;
    rows.sort_by(|left, right| {
        let left_id: Uuid = left.try_get("schema_revision_id").unwrap_or_default();
        let right_id: Uuid = right.try_get("schema_revision_id").unwrap_or_default();
        let left_exact = exact_revisions.contains(&left_id);
        let right_exact = exact_revisions.contains(&right_id);
        right_exact
            .cmp(&left_exact)
            .then_with(|| left_id.cmp(&right_id))
    });
    let revision_keys = rows
        .iter()
        .filter_map(|row| row.try_get::<Uuid, _>("schema_revision_id").ok())
        .collect::<Vec<_>>();
    let source_rows = sqlx::query(
        "SELECT l.schema_revision_id,o.source_class FROM cognitive_schema_evidence_links l JOIN observation_occurrences o USING(occurrence_id) WHERE l.subject_id=$1 AND l.schema_revision_id=ANY($2::uuid[]) AND l.revoked_at IS NULL",
    )
    .bind(query.subject.0)
    .bind(&revision_keys)
    .fetch_all(service.store.pool())
    .await
    .map_err(nous_persistence::database_error)?;
    let mut schema_sources = HashMap::<Uuid, Vec<SourceClass>>::new();
    for row in source_rows {
        let revision: Uuid = row
            .try_get("schema_revision_id")
            .map_err(nous_persistence::database_error)?;
        let source_class: String = row
            .try_get("source_class")
            .map_err(nous_persistence::database_error)?;
        schema_sources
            .entry(revision)
            .or_default()
            .push(SourceClass::from(source_class));
    }
    let mut revision_ids = Vec::new();
    for (index, row) in rows.into_iter().enumerate() {
        let revision = CognitiveSchemaRevisionId(
            row.try_get("schema_revision_id")
                .map_err(nous_persistence::database_error)?,
        );
        revision_ids.push(revision.0);
        let aboutness = row
            .try_get::<Vec<String>, _>("aboutness")
            .map_err(nous_persistence::database_error)?
            .into_iter()
            .map(EntityRef::new)
            .collect::<Result<Vec<_>>>()?;
        let historical = bound
            .revision_policy
            .allows_historical(&CognitiveRef::CognitiveSchemaRevision(revision));
        let accepted = row
            .try_get::<String, _>("acceptance_state")
            .map_err(nous_persistence::database_error)?
            == "accepted";
        let valid = row
            .try_get::<String, _>("integrity_state")
            .map_err(nous_persistence::database_error)?
            == "valid";
        let normal = row
            .try_get::<String, _>("suppression_state")
            .map_err(nous_persistence::database_error)?
            == "normal";
        let purge = row
            .try_get::<String, _>("purge_state")
            .map_err(nous_persistence::database_error)?
            == "normal";
        let requirements_match = query
            .expression
            .constraints
            .entity_requirements
            .iter()
            .all(|entity| aboutness.contains(entity));
        let source_matches = schema_sources.get(&revision.0).cloned().unwrap_or_default();
        let source_constraints_match = (query
            .expression
            .constraints
            .source_classes_include
            .is_empty()
            || query
                .expression
                .constraints
                .source_classes_include
                .iter()
                .any(|value| source_matches.contains(value)))
            && !query
                .expression
                .constraints
                .source_classes_exclude
                .iter()
                .any(|value| source_matches.contains(value));
        let modality_matches = query.expression.constraints.modalities.is_empty()
            || query
                .expression
                .constraints
                .modalities
                .contains(&Modality::Text);
        let evidence_matches = query.expression.constraints.evidence_classes.is_empty();
        let current = row
            .try_get::<Uuid, _>("current_revision_id")
            .map_err(nous_persistence::database_error)?
            == revision.0;
        if purge
            && (historical || (accepted && valid && normal && current))
            && requirements_match
            && source_constraints_match
            && modality_matches
            && evidence_matches
        {
            let reference = CognitiveRef::CognitiveSchemaRevision(revision);
            output.candidates.push(LaneCandidate {
                reference: reference.clone(),
                rank: (index + 1) as u32,
                variants: vec!["schema:direct".into()],
                provider_metadata: serde_json::Value::Null,
            });
        }
    }
    if !revision_ids.is_empty() {
        let rows = sqlx::query(
            "SELECT support_kind,support_ref FROM cognitive_schema_evidence_links WHERE subject_id=$1 AND role='support' AND schema_revision_id=ANY($2::uuid[]) AND revoked_at IS NULL ORDER BY support_kind,support_ref",
        )
        .bind(query.subject.0)
        .bind(revision_ids)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
        let rank = (output.candidates.len() + 1) as u32;
        for row in rows {
            let kind: String = row
                .try_get("support_kind")
                .map_err(nous_persistence::database_error)?;
            if let Ok(reference) = parse_reference(
                &kind,
                &row.try_get::<String, _>("support_ref")
                    .map_err(nous_persistence::database_error)?,
            ) {
                output.candidates.push(LaneCandidate {
                    reference,
                    rank,
                    variants: vec!["schema:direct_support".into()],
                    provider_metadata: serde_json::Value::Null,
                });
            }
        }
    }
    Ok(output)
}

#[expect(
    clippy::too_many_lines,
    reason = "Schema owner batch materialization keeps lifecycle, constraints, and payload aligned"
)]
pub(crate) async fn materialize_schema_revisions(
    service: &MemoryService,
    bound: &BoundQuery,
    references: &[CognitiveRef],
) -> Result<(Vec<CognitiveHit>, BTreeMap<String, usize>)> {
    let revision_ids = references
        .iter()
        .filter_map(|reference| match reference {
            CognitiveRef::CognitiveSchemaRevision(value) => Some(value.0),
            _ => None,
        })
        .collect::<Vec<_>>();
    if revision_ids.is_empty() {
        return Ok((Vec::new(), BTreeMap::new()));
    }
    let rows = sqlx::query("SELECT s.current_revision_id,s.object_epoch,s.acceptance_state,s.integrity_state,s.suppression_state,s.purge_state,r.schema_revision_id,r.structural_claim,r.aboutness,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM cognitive_schemas s JOIN cognitive_schema_revisions r ON r.schema_id=s.schema_id WHERE s.subject_id=$1 AND r.schema_revision_id=ANY($2::uuid[])")
        .bind(bound.source_query.subject.0)
        .bind(&revision_ids)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
    let mut drops = BTreeMap::new();
    let mut hits = Vec::new();
    for row in rows {
        let revision = CognitiveSchemaRevisionId(
            row.try_get("schema_revision_id")
                .map_err(nous_persistence::database_error)?,
        );
        let reference = CognitiveRef::CognitiveSchemaRevision(revision);
        let historical = bound.revision_policy.allows_historical(&reference);
        let current: Uuid = row
            .try_get("current_revision_id")
            .map_err(nous_persistence::database_error)?;
        if !historical && current != revision.0 {
            *drops.entry("not_current".into()).or_default() += 1;
            continue;
        }
        let accepted: String = row
            .try_get("acceptance_state")
            .map_err(nous_persistence::database_error)?;
        let valid: String = row
            .try_get("integrity_state")
            .map_err(nous_persistence::database_error)?;
        let suppression: String = row
            .try_get("suppression_state")
            .map_err(nous_persistence::database_error)?;
        let purge: String = row
            .try_get("purge_state")
            .map_err(nous_persistence::database_error)?;
        if accepted != "accepted"
            || valid != "valid"
            || suppression != "normal"
            || purge != "normal"
        {
            *drops.entry("unavailable_lifecycle".into()).or_default() += 1;
            continue;
        }
        if let Some(binding) = bound
            .exact_bindings
            .iter()
            .find(|binding| binding.bound_ref == reference)
            && binding.mutable_object
            && binding.bound_object_epoch
                != Some(
                    row.try_get("object_epoch")
                        .map_err(nous_persistence::database_error)?,
                )
        {
            *drops.entry("stale_exact_binding".into()).or_default() += 1;
            continue;
        }
        let aboutness = row
            .try_get::<Vec<String>, _>("aboutness")
            .map_err(nous_persistence::database_error)?
            .into_iter()
            .map(EntityRef::new)
            .collect::<Result<Vec<_>>>()?;
        if !bound
            .source_query
            .expression
            .constraints
            .entity_requirements
            .iter()
            .all(|entity| aboutness.contains(entity))
            || (!bound
                .source_query
                .expression
                .constraints
                .modalities
                .is_empty()
                && !bound
                    .source_query
                    .expression
                    .constraints
                    .modalities
                    .contains(&Modality::Text))
            || !bound
                .source_query
                .expression
                .constraints
                .evidence_classes
                .is_empty()
        {
            *drops.entry("query_constraints".into()).or_default() += 1;
            continue;
        }
        hits.push(CognitiveHit {
            reference: reference.clone(),
            revision: None,
            semantic_role: Some("cognitive_schema".into()),
            cognitive_role: None,
            formation_mode: None,
            representation: Some(
                row.try_get("structural_claim")
                    .map_err(nous_persistence::database_error)?,
            ),
            authority: AuthorityClass::SubjectCognition,
            freshness: FreshnessDescriptor {
                observed_at: None,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind")
                        .map_err(nous_persistence::database_error)?,
                    row.try_get("valid_time_start")
                        .map_err(nous_persistence::database_error)?,
                    row.try_get("valid_time_end")
                        .map_err(nous_persistence::database_error)?,
                )?,
                formed_at: Some(
                    row.try_get("formed_at")
                        .map_err(nous_persistence::database_error)?,
                ),
                recorded_at: Some(
                    row.try_get("recorded_at")
                        .map_err(nous_persistence::database_error)?,
                ),
            },
            entity_refs: aboutness,
            evidence: Vec::new(),
            match_evidence: MatchEvidence::default(),
            materialization: Vec::new(),
        });
    }
    Ok((hits, drops))
}
