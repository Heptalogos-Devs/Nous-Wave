use nous_core::*;

#[derive(Debug, Clone)]
pub struct QueryPlan {
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
    pub fn for_query(query: &CognitiveQuery) -> Self {
        let (multiplier, per_lane_max, validation, hops, states, resources): (
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
        ) = match query.effort {
            CognitiveEffort::Light => (2, 128, 4, 1, 128, 1),
            CognitiveEffort::Normal => (4, 512, 6, 2, 512, 4),
            CognitiveEffort::Deep => (8, 2048, 8, 3, 2048, 8),
            CognitiveEffort::Maximum => (16, 8192, 12, 4, 4096, 16),
        };
        let limit = query.result_need.limit;
        let explicit_topology = matches!(
            query.exploration,
            ExplorationIntent::BoundedAssociative
                | ExplorationIntent::AroundTag
                | ExplorationIntent::AroundSchema
                | ExplorationIntent::ExplainAssociation
        ) || query.cues.iter().any(|cue| matches!(cue, Cue::Relation(_)))
            || query.targets.iter().any(|target| {
                matches!(
                    target,
                    QueryTarget::EntityNeighborhood { .. } | QueryTarget::SchemaNeighborhood { .. }
                )
            });
        Self {
            candidate_limit: limit.saturating_mul(multiplier).clamp(16, per_lane_max),
            final_validation_budget: validation.saturating_mul(limit).max(match query.effort {
                CognitiveEffort::Light => 32,
                CognitiveEffort::Normal => 48,
                CognitiveEffort::Deep => 64,
                CognitiveEffort::Maximum => 128,
            }),
            sense_cues: query.capabilities.residual_sensing == RequirementStrength::Required
                && query.capabilities.residual_sensing != RequirementStrength::Forbidden,
            expand_topology: explicit_topology,
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

    pub fn serving_need(&self, query: &CognitiveQuery) -> ServingNeed {
        let has_text = query
            .cues
            .iter()
            .any(|cue| matches!(cue, Cue::Text(_) | Cue::Example(_)));
        let exact = query.targets.iter().any(|target| {
            matches!(
                target,
                QueryTarget::Exact { .. }
                    | QueryTarget::EntityNeighborhood { .. }
                    | QueryTarget::SchemaNeighborhood { .. }
            )
        }) || query
            .cues
            .iter()
            .any(|cue| matches!(cue, Cue::Entity(_) | Cue::Tag(_) | Cue::Schema(_)));
        ServingNeed {
            exact,
            lexical: has_text,
            dense: has_text && query.capabilities.text_embedding != RequirementStrength::Forbidden,
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
}
