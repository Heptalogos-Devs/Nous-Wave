use crate::*;
use async_trait::async_trait;
use nous_cognitive_runtime::{CognitiveContributor, QueryPlan};
use nous_memory_retrieval::{SourceSeed, propagate_with_budget};
use nous_serving::TextEmbeddingRequest;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Candidate {
    view: MemoryView,
    lane_ranks: BTreeMap<EvidenceFamily, usize>,
}

#[async_trait]
impl CognitiveContributor for MemoryService {
    #[expect(
        clippy::too_many_lines,
        reason = "query contribution owns bounded candidate generation, fusion, and final validation"
    )]
    async fn contribute(
        &self,
        query: &CognitiveQuery,
        plan: &QueryPlan,
    ) -> Result<CognitiveQueryResult> {
        let enabled = enabled_lanes(query, plan);
        if enabled.is_empty() {
            return Err(Error::Invalid("query has no enabled retrieval lane".into()));
        }
        let snapshot = self.serving.publisher.snapshot_for(query.subject);
        let mut selected = BTreeMap::<Uuid, Option<Uuid>>::new();
        let mut exact_memory_ids = BTreeSet::new();
        let mut exact_revision_ids = BTreeSet::new();

        for target in &query.targets {
            let QueryTarget::Exact { reference } = target else {
                continue;
            };
            match reference {
                CognitiveRef::Memory(memory) => {
                    self.store
                        .validate_reference(query.subject, reference)
                        .await?;
                    selected.entry(memory.0).or_insert(None);
                    exact_memory_ids.insert(memory.0);
                }
                CognitiveRef::MemoryRevision(revision) => {
                    self.store
                        .validate_reference(query.subject, reference)
                        .await?;
                    let memory_id: Uuid = sqlx::query_scalar(
                        "SELECT memory_id FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2",
                    )
                    .bind(query.subject.0)
                    .bind(revision.0)
                    .fetch_one(self.store.pool())
                    .await
                    .map_err(nous_authority_store::database_error)?;
                    selected.insert(memory_id, Some(revision.0));
                    exact_revision_ids.insert(revision.0);
                }
                _ => {}
            }
        }

        let text_query = query
            .cues
            .iter()
            .filter_map(|cue| match cue {
                Cue::Text(value) => Some(value.text.as_str()),
                Cue::Example(value) => Some(value.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut lexical_ranks = HashMap::<CognitiveRef, usize>::new();
        if !text_query.trim().is_empty()
            && let Some(index) = snapshot.lexical.as_ref()
        {
            for (rank, hit) in index
                .search(&text_query, plan.candidate_limit)?
                .into_iter()
                .enumerate()
            {
                if let Some(reference) = hit.reference {
                    add_serving_reference(self, query.subject, &mut selected, &reference).await?;
                    lexical_ranks.insert(reference, rank + 1);
                }
            }
        }

        let mut dense_ranks = HashMap::<CognitiveRef, usize>::new();
        if !text_query.trim().is_empty()
            && query.capabilities.text_embedding != RequirementStrength::Forbidden
            && let Some(provider) = self.serving.embedding.as_ref()
            && !snapshot.dense.is_empty()
        {
            let output = provider
                .embed(TextEmbeddingRequest {
                    subject: query.subject,
                    text: text_query.clone(),
                    query: true,
                })
                .await?;
            for generation in &snapshot.dense {
                if !output.space.compatible_with(&generation.space) {
                    continue;
                }
                for (rank, hit) in generation
                    .search(&output.vector, plan.candidate_limit)?
                    .into_iter()
                    .enumerate()
                {
                    if let Some(record) = hit.record {
                        add_serving_reference(
                            self,
                            query.subject,
                            &mut selected,
                            &record.reference,
                        )
                        .await?;
                        let reference = record.reference;
                        dense_ranks
                            .entry(reference)
                            .and_modify(|old| *old = (*old).min(rank + 1))
                            .or_insert(rank + 1);
                    }
                }
            }
        }

        // Entity/runtime/temporal lanes use bounded Authority admission.
        // Exact reads above are always admitted regardless of this bound.
        let rows = sqlx::query(
            "SELECT memory_id FROM memory_objects WHERE subject_id=$1 ORDER BY memory_id LIMIT $2",
        )
        .bind(query.subject.0)
        .bind(plan.candidate_limit as i64)
        .fetch_all(self.store.pool())
        .await
        .map_err(nous_authority_store::database_error)?;
        for row in rows {
            let memory_id: Uuid = row
                .try_get("memory_id")
                .map_err(nous_authority_store::database_error)?;
            selected.entry(memory_id).or_insert(None);
        }

        let topology_ranks = topology_ranks(&snapshot, query, plan);
        let text_cues = query
            .cues
            .iter()
            .filter_map(|cue| match cue {
                Cue::Text(value) => Some(value.text.to_lowercase()),
                Cue::Example(value) => Some(value.text.to_lowercase()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let entity_cues = query
            .cues
            .iter()
            .filter_map(|cue| match cue {
                Cue::Entity(value) => Some(value.entity_ref.as_str().to_owned()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut candidates = Vec::new();
        for (memory_id, requested_revision) in selected {
            let memory_id = MemoryId(memory_id);
            let view = match self
                .memory(
                    query.subject,
                    memory_id,
                    requested_revision.map(MemoryRevisionId),
                )
                .await
            {
                Ok(value) => value,
                Err(Error::NotFound(_)) => continue,
                Err(error) => return Err(error),
            };
            let exact = exact_memory_ids.contains(&memory_id.0)
                || exact_revision_ids.contains(&view.revision.memory_revision_id.0);
            if !hard_filter(query, &view, exact) {
                continue;
            }
            let mut lane_ranks = BTreeMap::new();
            if exact {
                lane_ranks.insert(EvidenceFamily::Exact, 1);
            }
            if !text_cues.is_empty()
                && text_cues.iter().any(|cue| {
                    view.revision
                        .representation_text
                        .to_lowercase()
                        .contains(cue)
                })
            {
                let reference = CognitiveRef::MemoryRevision(view.revision.memory_revision_id);
                lane_ranks.insert(
                    EvidenceFamily::Lexical,
                    lexical_ranks.get(&reference).copied().unwrap_or(1),
                );
            }
            let reference = CognitiveRef::MemoryRevision(view.revision.memory_revision_id);
            if let Some(rank) = dense_ranks.get(&reference) {
                lane_ranks.insert(EvidenceFamily::Dense, *rank);
            }
            if !entity_cues.is_empty()
                && entity_cues
                    .iter()
                    .all(|entity| view.aboutness.iter().any(|value| value.as_str() == entity))
            {
                lane_ranks.insert(EvidenceFamily::Entity, 1);
            }
            if query.session.is_some()
                && self
                    .cognition
                    .reference_in_subject(query.subject, &reference)
                    .await?
            {
                lane_ranks.insert(EvidenceFamily::Runtime, 1);
            }
            if temporal_match(query, &view)
                && (query.constraints.valid.is_some()
                    || query.constraints.occurred.is_some()
                    || query.constraints.observed.is_some())
            {
                lane_ranks.insert(EvidenceFamily::Temporal, 1);
            }
            if let Some(rank) = topology_ranks.get(&reference) {
                lane_ranks.insert(EvidenceFamily::TopologyWave, *rank);
            }
            if lane_ranks.is_empty() {
                continue;
            }
            candidates.push(Candidate { view, lane_ranks });
        }

        let weights = lane_weights();
        let denominator: f64 = enabled.iter().map(|lane| weights[lane] / 61.0).sum();
        let mut hits = candidates
            .into_iter()
            .map(|candidate| {
                let raw: f64 = enabled
                    .iter()
                    .filter_map(|lane| {
                        candidate
                            .lane_ranks
                            .get(lane)
                            .map(|rank| weights[lane] / (60.0 + *rank as f64))
                    })
                    .sum();
                let score = if denominator == 0.0 {
                    0.0
                } else {
                    raw / denominator
                };
                let best_rank = *candidate.lane_ranks.values().min().unwrap_or(&usize::MAX);
                let families = candidate.lane_ranks.keys().copied().collect::<Vec<_>>();
                CognitiveHit {
                    reference: CognitiveRef::MemoryRevision(
                        candidate.view.revision.memory_revision_id,
                    ),
                    revision: Some(candidate.view.revision.memory_revision_id),
                    semantic_role: Some(candidate.view.revision.semantic_role.clone()),
                    cognitive_role: Some(candidate.view.object.cognitive_role.as_str().into()),
                    formation_mode: Some(candidate.view.revision.formation_mode.as_str().into()),
                    representation: Some(candidate.view.revision.representation_text.clone()),
                    authority: AuthorityClass::SubjectCognition,
                    freshness: FreshnessDescriptor {
                        observed_at: candidate.view.temporal_evidence.observed_at,
                        valid_time: candidate.view.revision.valid_time.clone(),
                        formed_at: Some(candidate.view.revision.formed_at),
                        recorded_at: Some(candidate.view.revision.recorded_at),
                    },
                    entity_refs: candidate.view.aboutness.clone(),
                    evidence: if query.result_need.need_evidence {
                        candidate
                            .view
                            .supports
                            .iter()
                            .map(|support| match support {
                                RevisionSupport::Evidence(value) => EvidenceHandle {
                                    reference: value.cognitive_ref(),
                                    support_role: value.support_role.as_str().into(),
                                },
                                RevisionSupport::CognitionDependency(value) => EvidenceHandle {
                                    reference: value.target_revision.clone(),
                                    support_role: value.support_role.as_str().into(),
                                },
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    match_evidence: MatchEvidence {
                        families,
                        best_lane_rank: best_rank as u32,
                        enabled_lane_count: candidate.lane_ranks.len() as u32,
                        base_rank_score: score,
                        final_score: score,
                        variants: Vec::new(),
                        explanation: None,
                    },
                    materialization: Vec::new(),
                }
            })
            .collect::<Vec<_>>();
        hits.sort_by(|a, b| {
            b.match_evidence
                .final_score
                .total_cmp(&a.match_evidence.final_score)
                .then_with(|| {
                    a.match_evidence
                        .best_lane_rank
                        .cmp(&b.match_evidence.best_lane_rank)
                })
                .then_with(|| {
                    b.match_evidence
                        .enabled_lane_count
                        .cmp(&a.match_evidence.enabled_lane_count)
                })
                .then_with(|| a.reference.to_string().cmp(&b.reference.to_string()))
        });

        let validation_bound = plan.final_validation_budget.min(hits.len());
        let mut validated = Vec::new();
        for hit in hits.into_iter().take(validation_bound) {
            let CognitiveRef::MemoryRevision(revision) = hit.reference else {
                validated.push(hit);
                continue;
            };
            let Ok(view) = self.revision(query.subject, revision).await else {
                continue;
            };
            let exact = query.targets.iter().any(|target| {
                matches!(
                    target,
                    QueryTarget::Exact {
                        reference: CognitiveRef::MemoryRevision(id)
                    } if *id == revision
                ) || matches!(
                    target,
                    QueryTarget::Exact {
                        reference: CognitiveRef::Memory(id)
                    } if *id == view.object.memory_id
                )
            });
            if hard_filter(query, &view, exact) {
                validated.push(hit);
            }
        }
        validated.truncate(query.result_need.limit);
        let mut diagnostics = QueryDiagnostics {
            candidate_counts: BTreeMap::new(),
            lane_status: BTreeMap::new(),
            topology_complete: None,
            topology_discarded_mass: None,
            trace: None,
        };
        diagnostics
            .candidate_counts
            .insert("planned_candidate_bound".into(), plan.candidate_limit);
        diagnostics
            .candidate_counts
            .insert("final_validation_bound".into(), validation_bound);
        diagnostics.lane_status.insert(
            "topology".into(),
            if plan.expand_topology {
                "enabled"
            } else {
                "skipped"
            }
            .into(),
        );
        Ok(CognitiveQueryResult {
            query_id: Uuid::now_v7(),
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
            status: QueryStatus::Complete,
            results: validated,
            resource_actions: Vec::new(),
            degradation: Vec::new(),
            diagnostics: Some(diagnostics),
        })
    }
}

async fn add_serving_reference(
    service: &MemoryService,
    subject: SubjectId,
    selected: &mut BTreeMap<Uuid, Option<Uuid>>,
    reference: &CognitiveRef,
) -> Result<()> {
    match reference {
        CognitiveRef::Memory(memory) => {
            service.store.validate_reference(subject, reference).await?;
            selected.entry(memory.0).or_insert(None);
        }
        CognitiveRef::MemoryRevision(revision) => {
            if !service
                .store
                .reference_in_subject(subject, reference)
                .await?
            {
                return Ok(());
            }
            let memory_id: Uuid = sqlx::query_scalar(
                "SELECT memory_id FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2",
            )
            .bind(subject.0)
            .bind(revision.0)
            .fetch_one(service.store.pool())
            .await
            .map_err(nous_authority_store::database_error)?;
            selected.entry(memory_id).or_insert(Some(revision.0));
        }
        _ => {}
    }
    Ok(())
}

fn hard_filter(query: &CognitiveQuery, view: &MemoryView, exact: bool) -> bool {
    matches!(view.object.acceptance_state, AcceptanceState::Accepted)
        && matches!(view.object.integrity_state, IntegrityState::Valid)
        && matches!(view.object.purge_state, PurgeState::Normal)
        && (query.constraints.include_suppressed
            || matches!(view.object.suppression_state, SuppressionState::Normal))
        && accessibility_eligible(view.accessibility_level, query.effort, exact)
        && query
            .constraints
            .cognitive_roles_include
            .iter()
            .all(|role| role == view.object.cognitive_role.as_str())
        && query
            .constraints
            .formation_modes_include
            .iter()
            .all(|mode| mode == view.revision.formation_mode.as_str())
        && query
            .constraints
            .entity_requirements
            .iter()
            .all(|entity| view.aboutness.contains(entity))
        && temporal_match(query, view)
}

fn temporal_match(query: &CognitiveQuery, view: &MemoryView) -> bool {
    query
        .constraints
        .valid
        .is_none_or(|interval| view.revision.valid_time.overlaps_interval(&interval))
        && query.constraints.occurred.is_none_or(|interval| {
            view.temporal_evidence
                .occurred
                .iter()
                .any(|value| value.overlaps_interval(&interval))
        })
        && query.constraints.observed.is_none_or(|interval| {
            view.temporal_evidence
                .observed_at
                .is_some_and(|value| interval.contains(value))
        })
}

fn topology_ranks(
    snapshot: &nous_memory_retrieval::ServingSnapshot,
    query: &CognitiveQuery,
    plan: &QueryPlan,
) -> HashMap<CognitiveRef, usize> {
    let Some(graph) = snapshot.topology.as_ref() else {
        return HashMap::new();
    };
    if !plan.expand_topology {
        return HashMap::new();
    }
    let mut seeds = Vec::new();
    for target in &query.targets {
        if let QueryTarget::Exact { reference } = target {
            seeds.push((reference.clone(), "exact_target"));
        }
        match target {
            QueryTarget::EntityNeighborhood { entity_ref } => {
                seeds.push((CognitiveRef::Entity(entity_ref.clone()), "entity_cue"));
            }
            QueryTarget::SchemaNeighborhood { schema } => {
                seeds.push((CognitiveRef::CognitiveSchema(*schema), "schema_cue"));
            }
            _ => {}
        }
    }
    for cue in &query.cues {
        match cue {
            Cue::Entity(value) => {
                seeds.push((CognitiveRef::Entity(value.entity_ref.clone()), "entity_cue"))
            }
            Cue::Tag(value) => seeds.push((CognitiveRef::Tag(value.tag), "tag_cue")),
            Cue::Schema(value) => {
                seeds.push((CognitiveRef::CognitiveSchema(value.schema), "schema_cue"))
            }
            Cue::Relation(value) => {
                seeds.push((value.from.clone(), "relation_cue"));
                seeds.push((value.to.clone(), "relation_cue"));
            }
            _ => {}
        }
    }
    let source_seeds = seeds
        .into_iter()
        .filter_map(|(reference, origin)| {
            graph.node_id(&reference).map(|node| SourceSeed {
                node,
                weight: 1.0,
                seed_family: origin.into(),
                origin_cue: origin.into(),
                hop_zero: true,
            })
        })
        .collect::<Vec<_>>();
    if source_seeds.is_empty() {
        return HashMap::new();
    }
    let river = propagate_with_budget(
        graph,
        &source_seeds,
        plan.topology_rounds,
        plan.topology_nodes,
    );
    let mut values = river
        .node_potential
        .iter()
        .filter_map(|(node, potential)| {
            graph
                .nodes
                .get(*node as usize)
                .map(|value| (value.reference.clone(), *potential))
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.to_string().cmp(&right.0.to_string()))
    });
    values
        .into_iter()
        .enumerate()
        .map(|(index, (reference, _))| (reference, index + 1))
        .collect()
}

fn enabled_lanes(query: &CognitiveQuery, plan: &QueryPlan) -> Vec<EvidenceFamily> {
    let mut lanes = Vec::new();
    if query
        .targets
        .iter()
        .any(|target| matches!(target, QueryTarget::Exact { .. }))
    {
        lanes.push(EvidenceFamily::Exact);
    }
    if query.session.is_some() || !query.situation.current_refs.is_empty() {
        lanes.push(EvidenceFamily::Runtime);
    }
    if query.cues.iter().any(|cue| matches!(cue, Cue::Entity(_))) {
        lanes.push(EvidenceFamily::Entity);
    }
    if query
        .cues
        .iter()
        .any(|cue| matches!(cue, Cue::Text(_) | Cue::Example(_)))
    {
        lanes.push(EvidenceFamily::Lexical);
        if query.capabilities.text_embedding != RequirementStrength::Forbidden {
            lanes.push(EvidenceFamily::Dense);
        }
    }
    if query.constraints.valid.is_some()
        || query.constraints.occurred.is_some()
        || query.constraints.observed.is_some()
        || query.cues.iter().any(|cue| matches!(cue, Cue::Temporal(_)))
    {
        lanes.push(EvidenceFamily::Temporal);
    }
    if plan.expand_topology {
        lanes.push(EvidenceFamily::TopologyWave);
    }
    lanes.sort();
    lanes.dedup();
    lanes
}

fn lane_weights() -> HashMap<EvidenceFamily, f64> {
    HashMap::from([
        (EvidenceFamily::Exact, 4.0),
        (EvidenceFamily::Runtime, 2.0),
        (EvidenceFamily::Entity, 2.5),
        (EvidenceFamily::Lexical, 1.5),
        (EvidenceFamily::Dense, 1.5),
        (EvidenceFamily::Temporal, 1.0),
        (EvidenceFamily::TopologyWave, 1.0),
    ])
}
