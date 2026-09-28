use crate::{BoundQuery, CognitiveRuntimeService, QueryPlan};
use nous_cognitive_retrieval::{
    CandidateRankInput, LaneCandidate, LaneOutput, LaneStatus, rank_candidates_with_policy,
};
use nous_core::*;
use std::collections::{BTreeMap, HashMap};

#[async_trait::async_trait]
pub trait CognitiveContributor: Send + Sync {
    fn owns(&self, reference: &CognitiveRef) -> bool;

    async fn direct_lanes(&self, bound: &BoundQuery, plan: &QueryPlan) -> Result<Vec<LaneOutput>>;

    async fn validate_and_materialize(
        &self,
        subject: SubjectId,
        references: &[CognitiveRef],
        bound: &BoundQuery,
    ) -> Result<(Vec<CognitiveHit>, BTreeMap<String, usize>)>;
}

/// Shared Serving owns projection-backed lexical/dense/topology lanes. Domain
/// owners receive the resulting references and only validate/materialize their
/// own Authority objects after the runtime performs one fusion.
#[async_trait::async_trait]
pub trait SharedLaneProvider: Send + Sync {
    async fn lanes(&self, bound: &BoundQuery, plan: &QueryPlan) -> Result<Vec<LaneOutput>>;
}

/// The current fixed cognition-owner set. Adding an owner is a compile-time
/// composition change; callers do not discover domains dynamically.
pub struct CognitiveContributors<'a> {
    pub shared: Option<&'a dyn SharedLaneProvider>,
    pub memory: Option<&'a dyn CognitiveContributor>,
    pub self_cognition: Option<&'a dyn CognitiveContributor>,
    pub social: Option<&'a dyn CognitiveContributor>,
}

impl CognitiveRuntimeService {
    pub async fn query(
        &self,
        query: CognitiveQuery,
        contributors: CognitiveContributors<'_>,
    ) -> Result<CognitiveQueryResult> {
        let bound = self.bind_query(query).await?;
        let plan = QueryPlan::for_bound_query(&bound);
        self.query_with_plan(bound, contributors, plan).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "query orchestration keeps single fusion, owner batching, and final ordering in one semantic boundary"
    )]
    pub async fn query_with_plan(
        &self,
        bound: BoundQuery,
        contributors: CognitiveContributors<'_>,
        plan: QueryPlan,
    ) -> Result<CognitiveQueryResult> {
        let query = &bound.source_query;
        let mut result = CognitiveQueryResult {
            query_id: bound.query_id,
            generation: QueryGenerationTrace::default(),
            status: QueryStatus::Complete,
            results: Vec::new(),
            resource_actions: Vec::new(),
            degradation: Vec::new(),
            diagnostics: None,
        };
        let mut lane_outputs = Vec::new();
        if let Some(shared) = contributors.shared {
            lane_outputs.extend(shared.lanes(&bound, &plan).await?);
        }
        if let Some(memory) = contributors.memory {
            lane_outputs.extend(memory.direct_lanes(&bound, &plan).await?);
        } else if query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::Memory))
        {
            return Err(Error::Unavailable("Memory MicroSystem is disabled".into()));
        }
        if let Some(self_cognition) = contributors.self_cognition {
            lane_outputs.extend(self_cognition.direct_lanes(&bound, &plan).await?);
        } else if query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::SelfCognition))
        {
            return Err(Error::Unavailable("Self Authority is disabled".into()));
        }
        if let Some(social) = contributors.social {
            lane_outputs.extend(social.direct_lanes(&bound, &plan).await?);
        } else if query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::Social))
        {
            return Err(Error::Unavailable("Social Cognition is disabled".into()));
        }
        let mut exact_output = LaneOutput::empty(EvidenceFamily::Exact, LaneStatus::Ready);
        for binding in &bound.exact_bindings {
            exact_output.candidates.push(LaneCandidate {
                reference: binding.bound_ref.clone(),
                rank: 1,
                variants: Vec::new(),
                provider_metadata: serde_json::Value::Null,
            });
        }
        if !exact_output.candidates.is_empty() {
            lane_outputs.push(exact_output);
        }
        let runtime_allowed = query.targets.is_empty()
            || query.targets.iter().any(|target| {
                matches!(
                    target,
                    QueryTarget::AnyRelevantCognition | QueryTarget::Evidence
                )
            });
        if runtime_allowed && let Some(session) = query.session {
            for resident in self.session(query.subject, session).await?.resident {
                lane_outputs.push(LaneOutput {
                    family: EvidenceFamily::Runtime,
                    status: LaneStatus::Ready,
                    generation_ref: None,
                    authority_watermark: None,
                    candidates: vec![LaneCandidate {
                        reference: resident.reference,
                        rank: 1,
                        variants: vec!["runtime:resident_ref".into()],
                        provider_metadata: serde_json::Value::Null,
                    }],
                    diagnostics: Vec::new(),
                });
            }
        }
        let mut candidates = HashMap::<CognitiveRef, CandidateRankInput>::new();
        let mut lane_drops = BTreeMap::new();
        for output in &lane_outputs {
            for diagnostic in &output.diagnostics {
                if let Some(value) = diagnostic.strip_prefix("drop:")
                    && let Some((reason, count)) = value.split_once('=')
                    && let Ok(count) = count.parse::<usize>()
                {
                    *lane_drops.entry(reason.to_owned()).or_default() += count;
                }
            }
            if matches!(output.status, LaneStatus::Unavailable | LaneStatus::Stale) {
                result.degradation.push(Degradation {
                    code: format!("{:?}_lane_unavailable", output.family).to_lowercase(),
                    detail: output.diagnostics.first().cloned(),
                });
            }
            for candidate in &output.candidates {
                let entry = candidates
                    .entry(candidate.reference.clone())
                    .or_insert_with(|| CandidateRankInput {
                        reference: candidate.reference.clone(),
                        family_ranks: HashMap::new(),
                        family_view_ranks: HashMap::new(),
                        variants: Vec::new(),
                    });
                entry
                    .family_ranks
                    .entry(output.family)
                    .and_modify(|rank| *rank = (*rank).min(candidate.rank as usize))
                    .or_insert(candidate.rank as usize);
                entry.variants.extend(candidate.variants.clone());
            }
        }
        let rank_inputs = candidates.into_values().collect::<Vec<_>>();
        let ranked = rank_candidates_with_policy(
            &rank_inputs,
            &bound.enabled_lanes,
            &bound.retrieval_policy,
        );
        let validation_bound = plan.final_validation_budget.min(ranked.len());
        let validation_budget_exhausted = ranked.len() > validation_bound;
        if validation_budget_exhausted {
            result.status = QueryStatus::Partial;
            result.degradation.push(Degradation {
                code: "validation_budget_exhausted".into(),
                detail: Some(format!(
                    "validated {validation_bound} of {} ranked candidates",
                    ranked.len()
                )),
            });
        }
        let mut by_owner = [Vec::<CognitiveRef>::new(), Vec::new(), Vec::new()];
        let mut generic = Vec::new();
        for candidate in ranked.iter().take(validation_bound) {
            let memory_owner = contributors
                .memory
                .filter(|owner| owner.owns(&candidate.reference));
            let self_owner = contributors
                .self_cognition
                .filter(|owner| owner.owns(&candidate.reference));
            let social_owner = contributors
                .social
                .filter(|owner| owner.owns(&candidate.reference));
            let owner_count = usize::from(memory_owner.is_some())
                + usize::from(self_owner.is_some())
                + usize::from(social_owner.is_some());
            if owner_count > 1 {
                return Err(Error::Infrastructure(format!(
                    "multiple cognition owners claim {}",
                    candidate.reference
                )));
            }
            if memory_owner.is_some() {
                by_owner[0].push(candidate.reference.clone());
            } else if self_owner.is_some() {
                by_owner[1].push(candidate.reference.clone());
            } else if social_owner.is_some() {
                by_owner[2].push(candidate.reference.clone());
            } else if is_persistent_cognition(&candidate.reference) {
                return Err(Error::Unavailable(format!(
                    "no cognition owner is available for {}",
                    candidate.reference
                )));
            } else {
                generic.push(candidate.reference.clone());
            }
        }
        let mut materialized = HashMap::new();
        let mut validation_drops = BTreeMap::new();
        merge_counts(&mut validation_drops, lane_drops);
        if validation_budget_exhausted {
            validation_drops.insert(
                "validation_budget_exhausted".into(),
                ranked.len().saturating_sub(validation_bound),
            );
        }
        if let Some(owner) = contributors.memory {
            let (hits, drops) = owner
                .validate_and_materialize(query.subject, &by_owner[0], &bound)
                .await?;
            for hit in hits {
                materialized.insert(hit.reference.clone(), hit);
            }
            merge_counts(&mut validation_drops, drops);
        }
        if let Some(owner) = contributors.self_cognition {
            let (hits, drops) = owner
                .validate_and_materialize(query.subject, &by_owner[1], &bound)
                .await?;
            for hit in hits {
                materialized.insert(hit.reference.clone(), hit);
            }
            merge_counts(&mut validation_drops, drops);
        }
        if let Some(owner) = contributors.social {
            let (hits, drops) = owner
                .validate_and_materialize(query.subject, &by_owner[2], &bound)
                .await?;
            for hit in hits {
                materialized.insert(hit.reference.clone(), hit);
            }
            merge_counts(&mut validation_drops, drops);
        }
        for binding in &bound.exact_bindings {
            if binding.mutable_object {
                let owner_validates = contributors
                    .memory
                    .is_some_and(|owner| owner.owns(&binding.bound_ref))
                    || contributors
                        .self_cognition
                        .is_some_and(|owner| owner.owns(&binding.bound_ref))
                    || contributors
                        .social
                        .is_some_and(|owner| owner.owns(&binding.bound_ref));
                if owner_validates {
                    continue;
                }
                let (current, epoch, _) = self
                    .store
                    .bind_exact_reference(query.subject, &binding.requested_ref)
                    .await?;
                if current != binding.bound_ref || epoch != binding.bound_object_epoch {
                    *validation_drops
                        .entry("stale_exact_binding".into())
                        .or_default() += 1;
                }
            }
        }
        for reference in generic {
            self.store
                .validate_reference(query.subject, &reference)
                .await?;
            materialized.insert(
                reference.clone(),
                reference_hit(reference, EvidenceFamily::Exact, query),
            );
        }
        for candidate in ranked.into_iter().take(validation_bound) {
            if let Some(mut hit) = materialized.remove(&candidate.reference) {
                hit.match_evidence = MatchEvidence {
                    families: candidate.families.clone(),
                    base_rank_score: candidate.baseline_score,
                    best_lane_rank: candidate.best_lane_rank as u32,
                    enabled_lane_count: candidate.families.len() as u32,
                    final_score: candidate.final_score,
                    variants: candidate.variants,
                    explanation: None,
                };
                result.results.push(hit);
            }
        }
        let (actions, degradation) = self.resource_actions_for_query(query, &plan).await?;
        result.resource_actions = actions;
        result.degradation.extend(degradation);
        result.results.truncate(query.result_need.limit);
        if !result.degradation.is_empty() && result.status != QueryStatus::Partial {
            result.status = if result
                .degradation
                .iter()
                .any(|d| d.code == "required_external_authority_unresolved")
            {
                QueryStatus::Partial
            } else {
                QueryStatus::Degraded
            };
        }
        if validation_budget_exhausted {
            result.status = QueryStatus::Partial;
        }
        explain_plan(&mut result, query, &plan);
        if !validation_drops.is_empty() {
            let diagnostics = result.diagnostics.get_or_insert(QueryDiagnostics {
                candidate_counts: BTreeMap::new(),
                lane_status: BTreeMap::new(),
                topology_complete: None,
                topology_discarded_mass: None,
                trace: None,
            });
            for (reason, count) in validation_drops {
                diagnostics
                    .candidate_counts
                    .insert(format!("drop_{reason}"), count);
            }
        }
        if validation_budget_exhausted {
            result.status = QueryStatus::Partial;
        }
        Ok(result)
    }
}

fn merge_counts(target: &mut BTreeMap<String, usize>, incoming: BTreeMap<String, usize>) {
    for (key, value) in incoming {
        *target.entry(key).or_default() += value;
    }
}

fn is_persistent_cognition(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::Memory(_)
            | CognitiveRef::MemoryRevision(_)
            | CognitiveRef::CognitiveSchema(_)
            | CognitiveRef::CognitiveSchemaRevision(_)
            | CognitiveRef::SelfFacet(_)
            | CognitiveRef::SelfFacetRevision(_)
            | CognitiveRef::NarrativeIdentity(_)
            | CognitiveRef::NarrativeIdentityRevision(_)
            | CognitiveRef::RelationshipAssertion(_)
            | CognitiveRef::RelationshipRevision(_)
            | CognitiveRef::LanguageConvention(_)
            | CognitiveRef::LanguageConventionRevision(_)
    )
}

fn explain_plan(result: &mut CognitiveQueryResult, query: &CognitiveQuery, plan: &QueryPlan) {
    if query.diagnostics != DiagnosticsRequest::None {
        let diagnostics = result.diagnostics.get_or_insert(QueryDiagnostics {
            candidate_counts: BTreeMap::new(),
            lane_status: BTreeMap::new(),
            topology_complete: None,
            topology_discarded_mass: None,
            trace: None,
        });
        diagnostics.lane_status.insert(
            "effort".into(),
            format!("{:?}", query.effort).to_lowercase(),
        );
        diagnostics.lane_status.insert(
            "cue_sensing_plan".into(),
            if plan.sense_cues {
                "enabled"
            } else {
                "skipped"
            }
            .into(),
        );
        diagnostics.lane_status.insert(
            "topology_plan".into(),
            if plan.expand_topology {
                "enabled"
            } else {
                "skipped"
            }
            .into(),
        );
        diagnostics
            .candidate_counts
            .insert("planned_candidate_bound".into(), plan.candidate_limit);
        diagnostics.lane_status.insert(
            "resource_plan".into(),
            format!(
                "limit={},synopsis_preferred={}",
                plan.resource_limit, plan.prefer_resource_synopsis
            ),
        );
    }
}

fn reference_hit(
    reference: CognitiveRef,
    family: EvidenceFamily,
    query: &CognitiveQuery,
) -> CognitiveHit {
    let authority = match reference {
        CognitiveRef::Memory(_)
        | CognitiveRef::MemoryRevision(_)
        | CognitiveRef::Tag(_)
        | CognitiveRef::CognitiveSchema(_)
        | CognitiveRef::CognitiveSchemaRevision(_)
        | CognitiveRef::CognitiveSeedVersion(_)
        | CognitiveRef::SelfFacet(_)
        | CognitiveRef::SelfFacetRevision(_)
        | CognitiveRef::NarrativeIdentity(_)
        | CognitiveRef::NarrativeIdentityRevision(_)
        | CognitiveRef::RelationshipAssertion(_)
        | CognitiveRef::RelationshipRevision(_)
        | CognitiveRef::LanguageConvention(_)
        | CognitiveRef::LanguageConventionRevision(_) => AuthorityClass::SubjectCognition,
        CognitiveRef::Resource(_) => AuthorityClass::ResourceDescriptor,
        CognitiveRef::DerivedRepresentation(_) | CognitiveRef::DerivedRegion(_) => {
            AuthorityClass::Interpretation
        }
        _ => AuthorityClass::Evidence,
    };
    CognitiveHit {
        reference: reference.clone(),
        revision: None,
        semantic_role: Some("reference".into()),
        cognitive_role: None,
        formation_mode: None,
        representation: None,
        authority,
        freshness: FreshnessDescriptor {
            observed_at: None,
            valid_time: TemporalExtent::Unknown,
            formed_at: None,
            recorded_at: None,
        },
        entity_refs: Vec::new(),
        evidence: if query.result_need.need_evidence {
            vec![EvidenceHandle {
                reference: reference.clone(),
                support_role: "source".into(),
            }]
        } else {
            Vec::new()
        },
        match_evidence: MatchEvidence {
            families: vec![family],
            base_rank_score: 1.0,
            best_lane_rank: 1,
            enabled_lane_count: 1,
            final_score: 1.0,
            variants: Vec::new(),
            explanation: None,
        },
        materialization: if query.result_need.need_materialization_handles {
            vec![MaterializationHandle {
                reference,
                level: "source".into(),
            }]
        } else {
            Vec::new()
        },
    }
}
