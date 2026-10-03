use super::lane::{LaneCandidate, LaneOutput, LaneStatus};
use super::query_materialization::*;
use super::query_support::resolve_memory_references;
use super::schema_lane::{materialize_schema_revisions, schema_direct_lane};
use super::*;
use async_trait::async_trait;
use nous_runtime::{BoundQuery, CognitiveContributor, QueryPlan};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;
#[derive(Debug, Clone)]
pub(super) struct Candidate {
    pub(super) view: MemoryView,
}
#[derive(Debug, Clone)]
pub(super) struct FinalMemoryState {
    pub(super) acceptance_state: AcceptanceState,
    pub(super) integrity_state: IntegrityState,
    pub(super) suppression_state: SuppressionState,
    pub(super) purge_state: PurgeState,
    pub(super) accessibility_mode: AccessibilityMode,
}

#[async_trait]
impl CognitiveContributor for MemoryService {
    fn owns(&self, reference: &CognitiveRef) -> bool {
        matches!(
            reference,
            CognitiveRef::Memory(_)
                | CognitiveRef::MemoryRevision(_)
                | CognitiveRef::Episode(_)
                | CognitiveRef::EpisodeRevision(_)
                | CognitiveRef::Journal(_)
                | CognitiveRef::JournalRevision(_)
                | CognitiveRef::CognitiveSchema(_)
                | CognitiveRef::CognitiveSchemaRevision(_)
        )
    }

    async fn direct_lanes(&self, bound: &BoundQuery, plan: &QueryPlan) -> Result<Vec<LaneOutput>> {
        let mut outputs = Vec::new();
        if bound.lane_enabled(EvidenceFamily::Entity) {
            outputs.push(entity_lane(self, bound, plan).await?);
        }
        if bound.lane_enabled(EvidenceFamily::Temporal) {
            outputs.push(temporal_lane(self, bound, plan).await?);
        }
        if bound.lane_enabled(EvidenceFamily::SchemaDirect) {
            outputs.push(schema_direct_lane(self, bound, plan).await?);
        }
        Ok(outputs)
    }

    async fn validate_and_materialize(
        &self,
        _subject: SubjectId,
        references: &[CognitiveRef],
        bound: &BoundQuery,
    ) -> Result<(Vec<CognitiveHit>, BTreeMap<String, usize>)> {
        let subject = bound.source_query.subject;
        let accessibility_policy = resolve_accessibility_policy(&bound.config_snapshot)?;
        let memory_references = references
            .iter()
            .filter(|reference| {
                matches!(
                    reference,
                    CognitiveRef::Memory(_) | CognitiveRef::MemoryRevision(_)
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let resolved = resolve_memory_references(self, subject, &memory_references).await?;
        let revision_ids = resolved
            .values()
            .map(|(_, revision)| revision.0)
            .collect::<Vec<_>>();
        let views = self
            .memories_for_query(subject, &revision_ids, &accessibility_policy)
            .await?;
        let mut drops = BTreeMap::new();
        let source_objects = self
            .source_objects_for_revisions(subject, &revision_ids)
            .await?;
        let mut hits = Vec::new();
        for reference in &memory_references {
            let Some((_, revision)) = resolved.get(reference).copied() else {
                increment_drop(&mut drops, "not_in_subject");
                continue;
            };
            let Some(view) = views.get(&revision.0).cloned() else {
                increment_drop(&mut drops, "not_materialized");
                continue;
            };
            let historical = bound.revision_policy.allows_historical(reference)
                && view.object.current_revision_id != revision;
            let exact = bound.exact_bindings.iter().any(|binding| {
                binding.bound_ref == *reference
                    || binding.bound_ref == CognitiveRef::MemoryRevision(revision)
            });
            let state = FinalMemoryState {
                acceptance_state: view.object.acceptance_state,
                integrity_state: view.object.integrity_state,
                suppression_state: view.object.suppression_state,
                purge_state: view.object.purge_state,
                accessibility_mode: view.object.accessibility_mode,
            };
            if let Some(binding) = bound
                .exact_bindings
                .iter()
                .find(|binding| binding.bound_ref == *reference)
                && binding.mutable_object
                && (binding.bound_object_epoch != Some(view.object.object_epoch)
                    || (!historical && view.object.current_revision_id != revision))
            {
                increment_drop(&mut drops, "stale_exact_binding");
                continue;
            }
            if !historical && view.object.current_revision_id != revision {
                increment_drop(&mut drops, "stale_revision");
                continue;
            }
            let candidate = Candidate { view };
            if !final_state_filter(&bound.source_query, &candidate, &state, exact, historical) {
                increment_drop(&mut drops, &lifecycle_drop_reason(&state));
                continue;
            }
            let hit_reference = CognitiveRef::MemoryRevision(revision);
            let mut hit = to_hit(&bound.source_query, &candidate, hit_reference);
            hit.preference_refs
                .extend(source_objects.get(&revision.0).cloned().unwrap_or_default());
            hits.push(hit);
        }
        let (longitudinal_hits, longitudinal_drops) =
            super::longitudinal_query::materialize_longitudinal(self, bound, references).await?;
        hits.extend(longitudinal_hits);
        for (reason, count) in longitudinal_drops {
            *drops.entry(reason).or_default() += count;
        }
        let (schema_hits, schema_drops) =
            materialize_schema_revisions(self, bound, references).await?;
        hits.extend(schema_hits);
        for (reason, count) in schema_drops {
            *drops.entry(reason).or_default() += count;
        }
        Ok((hits, drops))
    }
}

async fn entity_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> Result<LaneOutput> {
    let query = &bound.source_query;
    let values = query
        .expression
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Entity(value) => Some(value.entity_ref.as_str().to_owned()),
            _ => None,
        })
        .chain(
            query
                .expression
                .constraints
                .entity_requirements
                .iter()
                .map(|value| value.as_str().to_owned()),
        )
        .collect::<BTreeSet<_>>();
    let values = values.into_iter().collect::<Vec<_>>();
    let mut output = LaneOutput::empty(EvidenceFamily::Entity, LaneStatus::Ready);
    if values.is_empty() {
        return Ok(output);
    }
    let rows = sqlx::query(
        "SELECT r.memory_revision_id,r.memory_id,COUNT(DISTINCT a.entity_ref)::bigint AS matched,MIN(array_position($2::text[],a.entity_ref)) AS best_cue FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id JOIN memory_revision_aboutness a USING(memory_revision_id) WHERE o.subject_id=$1 AND a.entity_ref=ANY($2::text[]) GROUP BY r.memory_revision_id,r.memory_id ORDER BY matched DESC,best_cue ASC,r.memory_revision_id LIMIT $3",
    )
    .bind(query.subject.0)
    .bind(&values)
    .bind(plan.lane_budget(EvidenceFamily::Entity) as i64)
    .fetch_all(service.store.pool())
    .await
    .map_err(nous_persistence::database_error)?;
    for (rank, row) in rows.into_iter().enumerate() {
        output.candidates.push(LaneCandidate {
            reference: CognitiveRef::MemoryRevision(MemoryRevisionId(
                row.try_get("memory_revision_id")
                    .map_err(nous_persistence::database_error)?,
            )),
            rank: (rank + 1) as u32,
            variants: vec!["entity:aboutness".into()],
            provider_metadata: serde_json::json!({
                "matched_cues": row.try_get::<i64, _>("matched").map_err(nous_persistence::database_error)?,
                "best_cue": row.try_get::<Option<i32>, _>("best_cue").map_err(nous_persistence::database_error)?,
            }),
        });
    }
    Ok(output)
}

#[expect(
    clippy::too_many_lines,
    reason = "Typed temporal axes share one bounded lane and final deterministic ordering"
)]
async fn temporal_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> Result<LaneOutput> {
    let query = &bound.source_query;
    let valid = query
        .expression
        .constraints
        .valid
        .into_iter()
        .collect::<Vec<_>>();
    let occurred = query
        .expression
        .constraints
        .occurred
        .into_iter()
        .collect::<Vec<_>>();
    let observed = query
        .expression
        .constraints
        .observed
        .into_iter()
        .collect::<Vec<_>>();
    let formed = query
        .expression
        .constraints
        .formed
        .into_iter()
        .collect::<Vec<_>>();
    let recorded = query
        .expression
        .constraints
        .recorded
        .into_iter()
        .collect::<Vec<_>>();
    let mut matches = HashMap::<Uuid, (Uuid, usize)>::new();
    let mut next_rank = 1usize;
    for interval in valid {
        let rows = sqlx::query(
            "SELECT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND ((r.valid_time_kind='instant' AND ($2::timestamptz IS NULL OR r.valid_time_start >= $2) AND ($3::timestamptz IS NULL OR r.valid_time_start < $3)) OR (r.valid_time_kind='interval' AND (r.valid_time_end IS NULL OR $2::timestamptz IS NULL OR r.valid_time_end>$2) AND (r.valid_time_start IS NULL OR $3::timestamptz IS NULL OR r.valid_time_start<$3))) ORDER BY r.recorded_at DESC,r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_persistence::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_persistence::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    for interval in occurred {
        let rows = sqlx::query(
            "SELECT DISTINCT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id JOIN memory_revision_evidence e USING(memory_revision_id) JOIN observation_occurrences oc USING(occurrence_id) WHERE o.subject_id=$1 AND ((oc.occurred_time_kind='instant' AND ($2::timestamptz IS NULL OR oc.occurred_time_start >= $2) AND ($3::timestamptz IS NULL OR oc.occurred_time_start < $3)) OR (oc.occurred_time_kind='interval' AND (oc.occurred_time_end IS NULL OR $2::timestamptz IS NULL OR oc.occurred_time_end>$2) AND (oc.occurred_time_start IS NULL OR $3::timestamptz IS NULL OR oc.occurred_time_start<$3))) ORDER BY r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_persistence::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_persistence::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    for interval in observed {
        let rows = sqlx::query(
            "SELECT DISTINCT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id JOIN memory_revision_evidence e USING(memory_revision_id) JOIN observation_occurrences oc USING(occurrence_id) WHERE o.subject_id=$1 AND oc.observed_at IS NOT NULL AND ($2::timestamptz IS NULL OR oc.observed_at>=$2) AND ($3::timestamptz IS NULL OR oc.observed_at<$3) ORDER BY r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_persistence::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_persistence::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    for interval in formed {
        let rows = sqlx::query(
            "SELECT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND ($2::timestamptz IS NULL OR r.formed_at >= $2) AND ($3::timestamptz IS NULL OR r.formed_at < $3) ORDER BY r.formed_at DESC,r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_persistence::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_persistence::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    for interval in recorded {
        let rows = sqlx::query(
            "SELECT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND ($2::timestamptz IS NULL OR r.recorded_at >= $2) AND ($3::timestamptz IS NULL OR r.recorded_at < $3) ORDER BY r.recorded_at DESC,r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_persistence::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_persistence::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    let mut output = LaneOutput::empty(EvidenceFamily::Temporal, LaneStatus::Ready);
    let mut values = matches.into_iter().collect::<Vec<_>>();
    values.sort_by(|left, right| left.1.1.cmp(&right.1.1).then_with(|| left.0.cmp(&right.0)));
    for (revision, (_memory, rank)) in values
        .into_iter()
        .take(plan.lane_budget(EvidenceFamily::Temporal))
    {
        output.candidates.push(LaneCandidate {
            reference: CognitiveRef::MemoryRevision(MemoryRevisionId(revision)),
            rank: rank as u32,
            variants: vec!["temporal:authority".into()],
            provider_metadata: serde_json::Value::Null,
        });
    }
    Ok(output)
}
