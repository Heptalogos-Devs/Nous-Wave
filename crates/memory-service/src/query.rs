use crate::lane::{LaneCandidate, LaneOutput, LaneStatus};
use crate::query_materialization::*;
use crate::query_support::resolve_memory_references;
use crate::schema_lane::final_schema_revision_states;
use crate::schema_lane::schema_direct_lane;
use crate::topology_lane::topology_ranks;
use crate::*;
use async_trait::async_trait;
use nous_cognitive_retrieval::{CandidateRankInput, RankedCandidate};
use nous_cognitive_runtime::{BoundQuery, CognitiveContributor, QueryPlan};
use nous_serving::TextEmbeddingRequest;
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use uuid::Uuid;
#[derive(Debug, Clone)]
pub(super) struct Candidate {
    pub(super) view: MemoryView,
    pub(super) lane_ranks: HashMap<EvidenceFamily, usize>,
    pub(super) variants: Vec<String>,
}
#[derive(Debug, Clone)]
pub(super) struct FinalMemoryState {
    pub(super) current_revision_id: MemoryRevisionId,
    pub(super) object_epoch: i64,
    pub(super) acceptance_state: AcceptanceState,
    pub(super) integrity_state: IntegrityState,
    pub(super) suppression_state: SuppressionState,
    pub(super) purge_state: PurgeState,
    pub(super) accessibility_mode: AccessibilityMode,
}

impl MemoryService {
    #[expect(
        clippy::too_many_lines,
        reason = "query contribution owns the bounded lane pipeline and final validation"
    )]
    async fn legacy_contribute(
        &self,
        bound: &BoundQuery,
        plan: &QueryPlan,
    ) -> Result<CognitiveQueryResult> {
        let query = &bound.source_query;
        let accessibility_policy = resolve_accessibility_policy(&bound.config_snapshot)?;
        let snapshot = self.serving.publisher.snapshot_for(query.subject);
        let mut selected = BTreeMap::<Uuid, Uuid>::new();
        let mut lane_ranks = BTreeMap::<Uuid, HashMap<EvidenceFamily, usize>>::new();
        let mut variants = BTreeMap::<Uuid, Vec<String>>::new();
        let mut degradation = Vec::new();
        let mut outputs = Vec::new();
        let mut schema_hits = Vec::new();

        if bound.lane_enabled(EvidenceFamily::Exact) {
            let mut output = LaneOutput::empty(EvidenceFamily::Exact, LaneStatus::Ready);
            for binding in &bound.exact_bindings {
                if matches!(
                    binding.bound_ref,
                    CognitiveRef::Memory(_) | CognitiveRef::MemoryRevision(_)
                ) {
                    output.candidates.push(LaneCandidate {
                        reference: binding.bound_ref.clone(),
                        rank: 1,
                        variants: Vec::new(),
                        provider_metadata: serde_json::json!({
                            "requested": binding.requested_ref.to_string(),
                            "mutable_object": binding.mutable_object,
                        }),
                    });
                }
            }
            outputs.push(output);
        }

        if bound.lane_enabled(EvidenceFamily::Lexical) {
            let output = lexical_lane(self, bound, plan, &snapshot).await?;
            if matches!(output.status, LaneStatus::Unavailable) {
                degradation.push(Degradation {
                    code: "lexical_projection_unavailable".into(),
                    detail: output.diagnostics.first().cloned(),
                });
            }
            outputs.push(output);
        }

        if bound.lane_enabled(EvidenceFamily::Dense) {
            let output = dense_lane(self, bound, plan, &snapshot).await?;
            if matches!(output.status, LaneStatus::Unavailable) {
                degradation.push(Degradation {
                    code: if query.capabilities.text_embedding == RequirementStrength::Required {
                        "required_dense_projection_unavailable"
                    } else {
                        "dense_projection_unavailable"
                    }
                    .into(),
                    detail: output.diagnostics.first().cloned(),
                });
            }
            outputs.push(output);
        }

        if bound.lane_enabled(EvidenceFamily::Entity) {
            outputs.push(entity_lane(self, bound, plan).await?);
        }
        if bound.lane_enabled(EvidenceFamily::Temporal) {
            outputs.push(temporal_lane(self, bound, plan).await?);
        }
        if bound.lane_enabled(EvidenceFamily::Runtime) {
            outputs.push(runtime_lane(self, bound, plan).await?);
        }
        if bound.lane_enabled(EvidenceFamily::SchemaDirect) {
            let (output, hits) = schema_direct_lane(self, bound, plan).await?;
            schema_hits = hits;
            outputs.push(output);
        }

        for output in outputs {
            apply_lane_output(
                self,
                query.subject,
                output,
                &mut selected,
                &mut lane_ranks,
                &mut variants,
            )
            .await?;
        }

        if bound.lane_enabled(EvidenceFamily::TopologyWave) {
            let topology = topology_ranks(&snapshot, bound, plan, &lane_ranks);
            let topology_refs = topology.keys().cloned().collect::<Vec<_>>();
            let resolved_topology =
                resolve_memory_references(self, query.subject, &topology_refs).await?;
            for (reference, rank) in topology {
                if let Some((memory_id, revision_id)) = resolved_topology.get(&reference).copied() {
                    selected.insert(revision_id.0, memory_id.0);
                    lane_ranks
                        .entry(revision_id.0)
                        .or_default()
                        .insert(EvidenceFamily::TopologyWave, rank);
                }
            }
        }

        let selected_revisions = selected.keys().copied().collect::<Vec<_>>();
        let materialized = self
            .memories_for_query(query.subject, &selected_revisions, &accessibility_policy)
            .await?;
        let mut candidates = Vec::new();
        for (revision_id, _memory_id) in selected {
            let revision_id = MemoryRevisionId(revision_id);
            let Some(view) = materialized.get(&revision_id.0).cloned() else {
                continue;
            };
            let exact = bound
                .exact_bindings
                .iter()
                .any(|binding| binding.bound_ref == CognitiveRef::MemoryRevision(revision_id));
            let historical = bound
                .revision_policy
                .allows_historical(&CognitiveRef::MemoryRevision(revision_id));
            if !hard_filter(query, &view, exact, historical) {
                continue;
            }
            let ranks = lane_ranks.remove(&revision_id.0).unwrap_or_default();
            if ranks.is_empty() {
                continue;
            }
            candidates.push((
                CognitiveRef::MemoryRevision(revision_id),
                Candidate {
                    view,
                    lane_ranks: ranks,
                    variants: variants.remove(&revision_id.0).unwrap_or_default(),
                },
            ));
        }

        let rank_inputs = candidates
            .iter()
            .map(|(reference, candidate)| CandidateRankInput {
                reference: reference.clone(),
                family_ranks: candidate.lane_ranks.clone(),
                family_view_ranks: HashMap::new(),
                variants: candidate.variants.clone(),
            })
            .collect::<Vec<_>>();
        let mut ranked = rank_inputs
            .into_iter()
            .map(|candidate| {
                let mut families = candidate.family_ranks.keys().copied().collect::<Vec<_>>();
                families.sort();
                let best_lane_rank = candidate
                    .family_ranks
                    .values()
                    .copied()
                    .min()
                    .unwrap_or(usize::MAX);
                let score = if best_lane_rank == usize::MAX {
                    0.0
                } else {
                    1.0 / (best_lane_rank as f64)
                };
                RankedCandidate {
                    reference: candidate.reference,
                    baseline_score: score,
                    final_score: score,
                    best_lane_rank,
                    exact_match: candidate.family_ranks.contains_key(&EvidenceFamily::Exact),
                    families,
                    variants: candidate.variants,
                }
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .final_score
                .total_cmp(&left.final_score)
                .then_with(|| left.reference.to_string().cmp(&right.reference.to_string()))
        });
        let candidates_by_ref = candidates.into_iter().collect::<HashMap<_, _>>();
        let validation_bound = plan.final_validation_budget.min(ranked.len());
        let validation_refs = ranked
            .iter()
            .take(validation_bound)
            .filter_map(|candidate| match candidate.reference {
                CognitiveRef::MemoryRevision(revision) => Some(revision.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let final_states = final_memory_states(self, query.subject, &validation_refs).await?;

        let mut validated = Vec::new();
        let mut drop_reasons = BTreeMap::<String, usize>::new();
        for ranked_candidate in ranked.iter().take(validation_bound) {
            let CognitiveRef::MemoryRevision(revision) = ranked_candidate.reference else {
                continue;
            };
            let Some(candidate) = candidates_by_ref.get(&ranked_candidate.reference) else {
                continue;
            };
            let Some(state) = final_states.get(&revision.0) else {
                increment_drop(&mut drop_reasons, "not_in_subject");
                continue;
            };
            let exact_binding = bound
                .exact_bindings
                .iter()
                .find(|binding| binding.bound_ref == CognitiveRef::MemoryRevision(revision));
            let historical = bound
                .revision_policy
                .allows_historical(&CognitiveRef::MemoryRevision(revision));
            if let Some(binding) = exact_binding
                && binding.mutable_object
                && binding.bound_object_epoch != Some(state.object_epoch)
            {
                increment_drop(&mut drop_reasons, "stale_exact_binding");
                continue;
            }
            if !historical && state.current_revision_id != revision {
                increment_drop(&mut drop_reasons, "not_current");
                continue;
            }
            if !final_state_filter(
                query,
                candidate,
                state,
                ranked_candidate.exact_match,
                historical,
            ) {
                let reason = lifecycle_drop_reason(state);
                increment_drop(&mut drop_reasons, &reason);
                continue;
            }
            validated.push(to_hit(query, candidate, ranked_candidate));
            if validated.len() >= query.result_need.limit {
                break;
            }
        }

        if validation_bound < ranked.len() && validated.len() < query.result_need.limit {
            degradation.push(Degradation {
                code: "validation_budget_exhausted".into(),
                detail: Some(format!(
                    "validated {validation_bound} of {} fused candidates",
                    ranked.len()
                )),
            });
        }
        if bound.rerank_policy.enabled {
            degradation.push(Degradation {
                code: "required_reranker_unavailable".into(),
                detail: Some("Reference runtime has no configured reranker provider".into()),
            });
        }
        let mut diagnostics = QueryDiagnostics {
            candidate_counts: BTreeMap::new(),
            lane_status: BTreeMap::new(),
            topology_complete: None,
            topology_discarded_mass: None,
            trace: None,
        };
        diagnostics
            .candidate_counts
            .insert("fused_candidates".into(), ranked.len());
        diagnostics
            .candidate_counts
            .insert("validation_budget".into(), plan.final_validation_budget);
        diagnostics
            .candidate_counts
            .insert("validated_candidates".into(), validated.len());
        for lane in &bound.enabled_lanes {
            diagnostics
                .lane_status
                .insert(format!("{lane:?}").to_lowercase(), "planned".into());
        }
        for (reason, count) in drop_reasons {
            diagnostics
                .candidate_counts
                .insert(format!("drop_{reason}"), count);
        }
        let status = if degradation.iter().any(|value| {
            value.code.contains("required") || value.code == "validation_budget_exhausted"
        }) {
            QueryStatus::Partial
        } else if degradation.is_empty() {
            QueryStatus::Complete
        } else {
            QueryStatus::Degraded
        };
        validated.extend(schema_hits);
        let schema_refs = validated
            .iter()
            .filter_map(|hit| match hit.reference {
                CognitiveRef::CognitiveSchemaRevision(revision) => Some(revision.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let valid_schema_refs =
            final_schema_revision_states(self, query.subject, &schema_refs).await?;
        validated.retain(|hit| match hit.reference {
            CognitiveRef::CognitiveSchemaRevision(revision) => {
                valid_schema_refs.get(&revision.0).is_some_and(|current| {
                    *current || bound.revision_policy.allows_historical(&hit.reference)
                })
            }
            _ => true,
        });
        validated.sort_by(|left, right| {
            right
                .match_evidence
                .final_score
                .total_cmp(&left.match_evidence.final_score)
                .then_with(|| left.reference.to_string().cmp(&right.reference.to_string()))
        });
        validated.truncate(query.result_need.limit);
        Ok(CognitiveQueryResult {
            query_id: bound.query_id,
            generation: QueryGenerationTrace {
                lexical: snapshot.lexical.as_ref().map(|value| value.generation_id),
                dense: snapshot
                    .dense
                    .iter()
                    .map(|value| value.generation_id)
                    .collect(),
                topology: snapshot.topology.as_ref().map(|value| value.generation_id),
                epa_basis: snapshot.epa.first().map(|value| value.generation_id),
                postings: snapshot.postings_generation,
            },
            status,
            results: validated,
            resource_actions: Vec::new(),
            degradation,
            diagnostics: (query.diagnostics != DiagnosticsRequest::None).then_some(diagnostics),
        })
    }
}

#[async_trait]
impl CognitiveContributor for MemoryService {
    fn owns(&self, reference: &CognitiveRef) -> bool {
        matches!(
            reference,
            CognitiveRef::Memory(_)
                | CognitiveRef::MemoryRevision(_)
                | CognitiveRef::CognitiveSchema(_)
                | CognitiveRef::CognitiveSchemaRevision(_)
        )
    }

    async fn direct_lanes(&self, bound: &BoundQuery, plan: &QueryPlan) -> Result<Vec<LaneOutput>> {
        let result = self.legacy_contribute(bound, plan).await?;
        let mut outputs = BTreeMap::<EvidenceFamily, LaneOutput>::new();
        for hit in result.results {
            for family in &hit.match_evidence.families {
                let output = outputs
                    .entry(*family)
                    .or_insert_with(|| LaneOutput::empty(*family, LaneStatus::Ready));
                output.candidates.push(LaneCandidate {
                    reference: hit.reference.clone(),
                    rank: hit.match_evidence.best_lane_rank,
                    variants: hit.match_evidence.variants.clone(),
                    provider_metadata: serde_json::Value::Null,
                });
            }
        }
        if let Some(diagnostics) = result.diagnostics {
            let output = outputs
                .entry(EvidenceFamily::Exact)
                .or_insert_with(|| LaneOutput::empty(EvidenceFamily::Exact, LaneStatus::Ready));
            for (reason, count) in diagnostics.candidate_counts {
                if reason.starts_with("drop_") {
                    output.diagnostics.push(format!("drop:{reason}={count}"));
                }
            }
        }
        Ok(outputs.into_values().collect())
    }

    async fn validate_and_materialize(
        &self,
        _subject: SubjectId,
        references: &[CognitiveRef],
        bound: &BoundQuery,
    ) -> Result<(Vec<CognitiveHit>, BTreeMap<String, usize>)> {
        let result = self
            .legacy_contribute(bound, &QueryPlan::for_bound_query(bound))
            .await?;
        let drops = result
            .diagnostics
            .as_ref()
            .map(|value| value.candidate_counts.clone())
            .unwrap_or_default();
        let hits = result
            .results
            .into_iter()
            .filter(|hit| references.contains(&hit.reference))
            .map(|mut hit| {
                hit.match_evidence = MatchEvidence::default();
                hit
            })
            .collect();
        Ok((hits, drops))
    }
}

async fn lexical_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
    snapshot: &nous_cognitive_retrieval::ServingSnapshot,
) -> Result<LaneOutput> {
    let mut output = LaneOutput::empty(EvidenceFamily::Lexical, LaneStatus::Unavailable);
    let text = text_query(&bound.source_query);
    let Some(index) = snapshot.lexical.as_ref() else {
        output
            .diagnostics
            .push("lexical serving generation is unavailable".into());
        return Ok(output);
    };
    output.status = LaneStatus::Ready;
    output.generation_ref = Some(index.generation_id);
    for (rank, hit) in index
        .search(&text, plan.lane_budget(EvidenceFamily::Lexical))?
        .into_iter()
        .enumerate()
    {
        if let Some(reference) = hit.reference
            && service
                .store
                .reference_in_subject(bound.source_query.subject, &reference)
                .await?
        {
            output.candidates.push(LaneCandidate {
                reference,
                rank: (rank + 1) as u32,
                variants: vec!["lexical:generation".into()],
                provider_metadata: serde_json::Value::Null,
            });
        }
    }
    Ok(output)
}

async fn dense_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
    snapshot: &nous_cognitive_retrieval::ServingSnapshot,
) -> Result<LaneOutput> {
    let mut output = LaneOutput::empty(EvidenceFamily::Dense, LaneStatus::Unavailable);
    let text = text_query(&bound.source_query);
    let Some(provider) = service.serving.embedding.as_ref() else {
        output
            .diagnostics
            .push("text embedding provider is unavailable".into());
        return Ok(output);
    };
    if snapshot.dense.is_empty() {
        output
            .diagnostics
            .push("dense serving generation is unavailable".into());
        return Ok(output);
    }
    let embedding = match provider
        .embed(TextEmbeddingRequest {
            subject: bound.source_query.subject,
            text,
            query: true,
        })
        .await
    {
        Ok(value) => value,
        Err(error) => {
            output.diagnostics.push(error.to_string());
            return Ok(output);
        }
    };
    output.status = LaneStatus::Ready;
    let mut best = HashMap::<CognitiveRef, (usize, Vec<String>)>::new();
    for generation in &snapshot.dense {
        if !embedding.space.compatible_with(&generation.space) {
            continue;
        }
        output.generation_ref = Some(generation.generation_id);
        for (rank, hit) in generation
            .search(&embedding.vector, plan.lane_budget(EvidenceFamily::Dense))?
            .into_iter()
            .enumerate()
        {
            let Some(record) = hit.record else {
                continue;
            };
            let rank = rank + 1;
            let entry = best
                .entry(record.reference.clone())
                .or_insert((rank, Vec::new()));
            if rank < entry.0 {
                entry.0 = rank;
            }
            entry
                .1
                .push(format!("dense:{}", generation.space.space_hash));
        }
    }
    if output.generation_ref.is_none() {
        output.status = LaneStatus::Unavailable;
        output
            .diagnostics
            .push("no dense generation matches the bound embedding space".into());
        return Ok(output);
    }
    let mut values = best.into_iter().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.1
            .0
            .cmp(&right.1.0)
            .then_with(|| left.0.to_string().cmp(&right.0.to_string()))
    });
    for (reference, (rank, variants)) in values.into_iter() {
        if service
            .store
            .reference_in_subject(bound.source_query.subject, &reference)
            .await?
        {
            output.candidates.push(LaneCandidate {
                reference,
                rank: rank as u32,
                variants,
                provider_metadata: serde_json::Value::Null,
            });
        }
    }
    Ok(output)
}

async fn entity_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> Result<LaneOutput> {
    let query = &bound.source_query;
    let values = query
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Entity(value) => Some(value.entity_ref.as_str().to_owned()),
            _ => None,
        })
        .chain(
            query
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
    .map_err(nous_authority_store::database_error)?;
    for (rank, row) in rows.into_iter().enumerate() {
        output.candidates.push(LaneCandidate {
            reference: CognitiveRef::MemoryRevision(MemoryRevisionId(
                row.try_get("memory_revision_id")
                    .map_err(nous_authority_store::database_error)?,
            )),
            rank: (rank + 1) as u32,
            variants: vec!["entity:aboutness".into()],
            provider_metadata: serde_json::json!({
                "matched_cues": row.try_get::<i64, _>("matched").map_err(nous_authority_store::database_error)?,
                "best_cue": row.try_get::<Option<i32>, _>("best_cue").map_err(nous_authority_store::database_error)?,
            }),
        });
    }
    Ok(output)
}

async fn temporal_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> Result<LaneOutput> {
    let query = &bound.source_query;
    let cue_intervals = query
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Temporal(value) => Some(value.interval),
            _ => None,
        })
        .collect::<Vec<_>>();
    let valid = query
        .constraints
        .valid
        .into_iter()
        .chain(cue_intervals.iter().copied())
        .collect::<Vec<_>>();
    let occurred = query.constraints.occurred.into_iter().collect::<Vec<_>>();
    let observed = query.constraints.observed.into_iter().collect::<Vec<_>>();
    let mut matches = HashMap::<Uuid, (Uuid, usize)>::new();
    let mut next_rank = 1usize;
    for interval in valid {
        let rows = sqlx::query(
            "SELECT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND r.valid_time_kind <> 'unknown' AND (COALESCE(r.valid_time_end,r.valid_time_start) IS NULL OR $2::timestamptz IS NULL OR COALESCE(r.valid_time_end,r.valid_time_start)>$2) AND (r.valid_time_start IS NULL OR $3::timestamptz IS NULL OR r.valid_time_start<$3) ORDER BY r.recorded_at DESC,r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_authority_store::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_authority_store::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_authority_store::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    for interval in occurred {
        let rows = sqlx::query(
            "SELECT DISTINCT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id JOIN memory_revision_evidence e USING(memory_revision_id) JOIN observation_occurrences oc USING(occurrence_id) WHERE o.subject_id=$1 AND oc.occurred_time_kind <> 'unknown' AND (COALESCE(oc.occurred_time_end,oc.occurred_time_start) IS NULL OR $2::timestamptz IS NULL OR COALESCE(oc.occurred_time_end,oc.occurred_time_start)>$2) AND (oc.occurred_time_start IS NULL OR $3::timestamptz IS NULL OR oc.occurred_time_start<$3) ORDER BY r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_authority_store::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_authority_store::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_authority_store::database_error)?;
            matches.entry(revision).or_insert((memory, next_rank));
            next_rank += 1;
        }
    }
    for interval in observed
        .iter()
        .copied()
        .chain(cue_intervals.iter().copied())
    {
        let rows = sqlx::query(
            "SELECT DISTINCT r.memory_revision_id,r.memory_id FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id JOIN memory_revision_evidence e USING(memory_revision_id) JOIN observation_occurrences oc USING(occurrence_id) WHERE o.subject_id=$1 AND oc.observed_at IS NOT NULL AND ($2::timestamptz IS NULL OR oc.observed_at>=$2) AND ($3::timestamptz IS NULL OR oc.observed_at<$3) ORDER BY r.memory_revision_id LIMIT $4",
        )
        .bind(query.subject.0)
        .bind(interval.start)
        .bind(interval.end)
        .bind(plan.lane_budget(EvidenceFamily::Temporal) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_authority_store::database_error)?;
        for row in rows {
            let revision: Uuid = row
                .try_get("memory_revision_id")
                .map_err(nous_authority_store::database_error)?;
            let memory: Uuid = row
                .try_get("memory_id")
                .map_err(nous_authority_store::database_error)?;
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

async fn runtime_lane(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> Result<LaneOutput> {
    let query = &bound.source_query;
    let mut output = LaneOutput::empty(EvidenceFamily::Runtime, LaneStatus::Ready);
    let mut seen = HashSet::new();
    let mut rank = 1u32;
    for reference in &bound.runtime_refs {
        if matches!(reference, CognitiveRef::MemoryRevision(_)) && seen.insert(reference.clone()) {
            output.candidates.push(LaneCandidate {
                reference: reference.clone(),
                rank,
                variants: vec!["runtime:situation_current_ref".into()],
                provider_metadata: serde_json::Value::Null,
            });
            rank += 1;
        }
    }
    if let Some(session) = query.session {
        let rows = sqlx::query(
            "SELECT ref_kind,ref_value FROM resident_refs r JOIN cognitive_sessions s USING(session_id) WHERE s.subject_id=$1 AND s.session_id=$2 AND s.closed_at IS NULL AND r.state='resident' ORDER BY r.last_meaningful_use_at DESC NULLS LAST,r.entered_at DESC,r.ref_kind,r.ref_value LIMIT $3",
        )
        .bind(query.subject.0)
        .bind(session.0)
        .bind(plan.lane_budget(EvidenceFamily::Runtime) as i64)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_authority_store::database_error)?;
        for row in rows {
            let reference = parse_reference(
                &row.try_get::<String, _>("ref_kind")
                    .map_err(nous_authority_store::database_error)?,
                &row.try_get::<String, _>("ref_value")
                    .map_err(nous_authority_store::database_error)?,
            )?;
            if matches!(reference, CognitiveRef::MemoryRevision(_))
                && seen.insert(reference.clone())
            {
                output.candidates.push(LaneCandidate {
                    reference,
                    rank,
                    variants: vec!["runtime:resident_ref".into()],
                    provider_metadata: serde_json::Value::Null,
                });
                rank += 1;
            }
        }
    }
    output
        .candidates
        .truncate(plan.lane_budget(EvidenceFamily::Runtime));
    Ok(output)
}
