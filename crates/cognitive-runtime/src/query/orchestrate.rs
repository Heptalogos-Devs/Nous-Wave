use crate::{BoundQuery, CognitiveRuntimeService, QueryPlan};
use nous_core::*;
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait CognitiveContributor: Send + Sync {
    async fn contribute(
        &self,
        bound: &BoundQuery,
        plan: &QueryPlan,
    ) -> Result<CognitiveQueryResult>;
}

/// The current fixed cognition-owner set. Adding an owner is a compile-time
/// composition change; callers do not discover domains dynamically.
pub struct CognitiveContributors<'a> {
    pub memory: Option<&'a dyn CognitiveContributor>,
    pub self_cognition: Option<&'a dyn CognitiveContributor>,
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

    pub async fn query_with_plan(
        &self,
        bound: BoundQuery,
        contributors: CognitiveContributors<'_>,
        plan: QueryPlan,
    ) -> Result<CognitiveQueryResult> {
        let query = &bound.source_query;
        let mut result = CognitiveQueryResult {
            query_id: Uuid::now_v7(),
            generation: QueryGenerationTrace::default(),
            status: QueryStatus::Complete,
            results: Vec::new(),
            resource_actions: Vec::new(),
            degradation: Vec::new(),
            diagnostics: None,
        };
        if let Some(memory) = contributors.memory {
            let memory_result = memory.contribute(&bound, &plan).await?;
            result.generation = memory_result.generation;
            result.results.extend(memory_result.results);
            result
                .resource_actions
                .extend(memory_result.resource_actions);
            result.degradation.extend(memory_result.degradation);
            result.diagnostics = memory_result.diagnostics;
        } else if query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::Memory))
        {
            return Err(Error::Unavailable("Memory MicroSystem is disabled".into()));
        }
        if let Some(self_cognition) = contributors.self_cognition {
            let self_result = self_cognition.contribute(&bound, &plan).await?;
            result.results.extend(self_result.results);
            result.resource_actions.extend(self_result.resource_actions);
            result.degradation.extend(self_result.degradation);
            if result.diagnostics.is_none() {
                result.diagnostics = self_result.diagnostics;
            }
        } else if query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::SelfCognition))
        {
            return Err(Error::Unavailable("Self Authority is disabled".into()));
        }
        result.query_id = bound.query_id;
        let mut seen: HashSet<CognitiveRef> = result
            .results
            .iter()
            .map(|hit| hit.reference.clone())
            .collect();
        for binding in &bound.exact_bindings {
            let reference = &binding.bound_ref;

            if matches!(
                reference,
                CognitiveRef::Memory(_)
                    | CognitiveRef::MemoryRevision(_)
                    | CognitiveRef::SelfFacet(_)
                    | CognitiveRef::SelfFacetRevision(_)
                    | CognitiveRef::NarrativeIdentity(_)
                    | CognitiveRef::NarrativeIdentityRevision(_)
            ) {
                continue;
            }
            self.store
                .validate_reference(query.subject, reference)
                .await?;
            if seen.insert(reference.clone()) {
                result.results.push(reference_hit(
                    reference.clone(),
                    EvidenceFamily::Exact,
                    query,
                ));
            }
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
                if matches!(
                    resident.reference,
                    CognitiveRef::Memory(_) | CognitiveRef::MemoryRevision(_)
                ) {
                    continue;
                }
                if seen.insert(resident.reference.clone()) {
                    result.results.push(reference_hit(
                        resident.reference,
                        EvidenceFamily::Runtime,
                        query,
                    ));
                }
            }
        }
        let (actions, degradation) = self.resource_actions_for_query(query, &plan).await?;
        result.resource_actions = actions;
        result.degradation.extend(degradation);
        result.results.sort_by(|a, b| {
            b.match_evidence
                .final_score
                .total_cmp(&a.match_evidence.final_score)
                .then_with(|| a.reference.to_string().cmp(&b.reference.to_string()))
        });
        result.results.truncate(query.result_need.limit);
        if !result.degradation.is_empty() {
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
        explain_plan(&mut result, query, &plan);
        Ok(result)
    }
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
        | CognitiveRef::NarrativeIdentityRevision(_) => AuthorityClass::SubjectCognition,
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
