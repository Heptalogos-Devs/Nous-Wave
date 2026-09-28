use super::{BoundQuery, planned_lanes};
use nous_core::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub enabled_lanes: Vec<EvidenceFamily>,
    pub lane_budgets: BTreeMap<EvidenceFamily, usize>,
    /// Compatibility accessor for bounded non-exact providers. The plan
    /// itself is lane-specific; providers must use `lane_budgets`.
    pub candidate_limit: usize,
    pub final_validation_budget: usize,
    pub sense_cues: bool,
    pub expand_topology: bool,
    pub topology_rounds: usize,
    pub topology_nodes: usize,
    pub resource_limit: usize,
    pub materialize_evidence: bool,
    pub prefer_resource_synopsis: bool,
}

impl QueryPlan {
    pub fn for_bound_query(bound: &BoundQuery) -> Self {
        Self::from_parts(
            &bound.source_query,
            bound.enabled_lanes.clone(),
            bound.lane_budgets.clone(),
            &bound.retrieval_policy,
        )
    }

    /// Retained for local plan tests and non-authority callers. Production
    /// query execution uses `BoundQuery::bind_query` and `for_bound_query`.
    pub fn for_query(query: &CognitiveQuery) -> Self {
        let lanes = planned_lanes(query);
        let (multiplier, per_lane_max) = match query.effort {
            CognitiveEffort::Light => (2, 128),
            CognitiveEffort::Normal => (4, 512),
            CognitiveEffort::Deep => (8, 2048),
            CognitiveEffort::Maximum => (16, 8192),
        };
        let budget = query
            .result_need
            .limit
            .saturating_mul(multiplier)
            .clamp(16, per_lane_max);
        let budgets = lanes.iter().copied().map(|lane| (lane, budget)).collect();
        Self::from_parts(
            query,
            lanes,
            budgets,
            &nous_cognitive_retrieval::RetrievalPolicy::reference(),
        )
    }

    fn from_parts(
        query: &CognitiveQuery,
        enabled_lanes: Vec<EvidenceFamily>,
        lane_budgets: BTreeMap<EvidenceFamily, usize>,
        retrieval_policy: &nous_cognitive_retrieval::RetrievalPolicy,
    ) -> Self {
        let (hops, states, resources) = match query.effort {
            CognitiveEffort::Light => (1, 128, 1),
            CognitiveEffort::Normal => (2, 512, 4),
            CognitiveEffort::Deep => (3, 2048, 8),
            CognitiveEffort::Maximum => (4, 4096, 16),
        };
        let candidate_limit = lane_budgets.values().copied().max().unwrap_or(0);
        Self {
            enabled_lanes: enabled_lanes.clone(),
            lane_budgets,
            candidate_limit,
            final_validation_budget: query
                .result_need
                .limit
                .saturating_mul(retrieval_policy.validation_multiplier)
                .clamp(
                    retrieval_policy.validation_min,
                    retrieval_policy.validation_max,
                ),
            sense_cues: query.capabilities.residual_sensing == RequirementStrength::Required,
            expand_topology: enabled_lanes.contains(&EvidenceFamily::TopologyWave),
            topology_rounds: hops,
            topology_nodes: states,
            resource_limit: resources,
            materialize_evidence: query.result_need.need_evidence
                && matches!(
                    query.effort,
                    CognitiveEffort::Deep | CognitiveEffort::Maximum
                ),
            prefer_resource_synopsis: query.resources.synopsis_only
                || matches!(query.exploration, ExplorationIntent::Global),
        }
    }

    pub fn lane_budget(&self, family: EvidenceFamily) -> usize {
        self.lane_budgets.get(&family).copied().unwrap_or(0)
    }

    pub fn serving_need(&self, query: &CognitiveQuery) -> ServingNeed {
        let has_text = query
            .cues
            .iter()
            .any(|cue| matches!(cue, Cue::Text(_) | Cue::Example(_)));
        ServingNeed {
            exact: self.enabled_lanes.contains(&EvidenceFamily::Exact)
                || self.enabled_lanes.contains(&EvidenceFamily::SchemaDirect),
            lexical: has_text && self.enabled_lanes.contains(&EvidenceFamily::Lexical),
            dense: has_text && self.enabled_lanes.contains(&EvidenceFamily::Dense),
            topology: self.expand_topology,
        }
    }
}

#[derive(Debug, Default)]
pub struct WorkCycle {
    pub inspected: std::collections::HashSet<CognitiveRef>,
    pub frontier: Vec<CognitiveRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(effort: CognitiveEffort) -> CognitiveQuery {
        CognitiveQuery {
            api_version: API_VERSION,
            subject: SubjectId::new(),
            session: None,
            situation: Default::default(),
            targets: Vec::new(),
            cues: vec![Cue::Text(TextCue {
                text: "query".into(),
            })],
            constraints: Default::default(),
            exploration: ExplorationIntent::None,
            resources: Default::default(),
            result_need: Default::default(),
            effort,
            capabilities: Default::default(),
            diagnostics: DiagnosticsRequest::Summary,
        }
    }

    #[test]
    fn deep_text_does_not_enable_topology_without_explicit_intent() {
        assert!(!QueryPlan::for_query(&query(CognitiveEffort::Deep)).expand_topology);
    }

    #[test]
    fn validation_budget_is_the_frozen_reference_formula() {
        let mut value = query(CognitiveEffort::Normal);
        value.result_need.limit = 12;
        assert_eq!(QueryPlan::for_query(&value).final_validation_budget, 48);
    }
}
