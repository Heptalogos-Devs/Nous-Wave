use super::types::{
    AccessibilityQueryPolicy, BoundQuery, ExactBinding, RerankPolicy, RevisionPolicy,
};
use crate::CognitiveRuntimeService;
use super::resolve_retrieval_policy;
use nous_core::*;
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

pub fn planned_lanes(query: &CognitiveQuery) -> Vec<EvidenceFamily> {
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
    if query.cues.iter().any(|cue| matches!(cue, Cue::Entity(_)))
        || !query.constraints.entity_requirements.is_empty()
    {
        lanes.push(EvidenceFamily::Entity);
    }
    let has_text = query
        .cues
        .iter()
        .any(|cue| matches!(cue, Cue::Text(_) | Cue::Example(_)));
    if has_text {
        lanes.push(EvidenceFamily::Lexical);
        if query.capabilities.text_embedding != RequirementStrength::Forbidden {
            // The lane remains planned even when its provider is unavailable.
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
    if query.cues.iter().any(|cue| matches!(cue, Cue::Schema(_)))
        || query.targets.iter().any(|target| {
            matches!(
                target,
                QueryTarget::SchemaNeighborhood { .. }
                    | QueryTarget::Exact {
                        reference: CognitiveRef::CognitiveSchema(_)
                            | CognitiveRef::CognitiveSchemaRevision(_),
                    }
            )
        })
    {
        lanes.push(EvidenceFamily::SchemaDirect);
    }
    if explicit_topology(query) {
        lanes.push(EvidenceFamily::TopologyWave);
    }
    lanes.sort();
    lanes.dedup();
    lanes
}

pub fn explicit_topology(query: &CognitiveQuery) -> bool {
    matches!(
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
        })
}

fn validate_hard_constraints(query: &CognitiveQuery) -> Result<()> {
    for role in &query.constraints.cognitive_roles_include {
        if !matches!(
            role.as_str(),
            "experiential" | "declarative" | "procedural_experience"
        ) {
            return Err(Error::Invalid(format!(
                "unsupported cognitive role constraint: {role}"
            )));
        }
    }
    for mode in &query.constraints.formation_modes_include {
        if !matches!(mode.as_str(), "grounded" | "synthesized") {
            return Err(Error::Invalid(format!(
                "unsupported formation mode constraint: {mode}"
            )));
        }
    }
    for evidence_class in &query.constraints.evidence_classes {
        if !matches!(
            evidence_class.as_str(),
            "observed" | "reported" | "derived" | "inferred" | "narrative" | "simulated"
        ) {
            return Err(Error::Invalid(format!(
                "unsupported evidence class constraint: {evidence_class}"
            )));
        }
    }
    Ok(())
}

fn budget_values(
    query: &CognitiveQuery,
    lanes: &[EvidenceFamily],
    policy: &super::RetrievalPolicy,
) -> BTreeMap<EvidenceFamily, usize> {
    let index = match query.effort {
        CognitiveEffort::Light => 0,
        CognitiveEffort::Normal => 1,
        CognitiveEffort::Deep => 2,
        CognitiveEffort::Maximum => 3,
    };
    let budget = query
        .result_need
        .limit
        .saturating_mul(policy.effort_multipliers[index])
        .clamp(16, policy.per_lane_max[index]);
    lanes.iter().copied().map(|lane| (lane, budget)).collect()
}

impl CognitiveRuntimeService {
    #[expect(
        clippy::too_many_lines,
        reason = "query binding freezes identity, capabilities, config snapshot, lanes, and revision fences together"
    )]
    pub async fn bind_query(&self, query: CognitiveQuery) -> Result<BoundQuery> {
        query.validate()?;
        validate_hard_constraints(&query)?;
        self.require_subject(query.subject).await?;
        let config_snapshot = self.configuration.snapshot_for_subject(query.subject)?;
        let retrieval_policy = resolve_retrieval_policy(&config_snapshot)?;
        if let Some(session) = query.session {
            self.require_session(query.subject, session).await?;
        }

        let mut exact_bindings = Vec::new();
        let mut allowed_revision_refs = HashSet::new();
        for target in &query.targets {
            let QueryTarget::Exact { reference } = target else {
                continue;
            };
            let (bound_ref, epoch, mutable_object) = self
                .store
                .bind_exact_reference(query.subject, reference)
                .await?;
            if !mutable_object
                && matches!(
                    reference,
                    CognitiveRef::MemoryRevision(_)
                        | CognitiveRef::CognitiveSchemaRevision(_)
                )
            {
                allowed_revision_refs.insert(bound_ref.clone());
            }
            exact_bindings.push(ExactBinding {
                requested_ref: reference.clone(),
                bound_ref,
                bound_object_epoch: epoch,
                mutable_object,
            });
        }

        let mut runtime_refs = Vec::new();
        for reference in &query.situation.current_refs {
            let (bound_ref, _, _) = self
                .store
                .bind_exact_reference(query.subject, reference)
                .await?;
            runtime_refs.push(bound_ref);
        }

        let mut topology_seed_refs = Vec::new();
        for target in &query.targets {
            if let QueryTarget::SchemaNeighborhood { schema } = target {
                let (bound_ref, _, _) = self
                    .store
                    .bind_exact_reference(query.subject, &CognitiveRef::CognitiveSchema(*schema))
                    .await?;
                topology_seed_refs.push((bound_ref, "explicit_schema".into()));
            }
        }
        for cue in &query.cues {
            match cue {
                Cue::Schema(value) => {
                    let (bound_ref, _, _) = self
                        .store
                        .bind_exact_reference(
                            query.subject,
                            &CognitiveRef::CognitiveSchema(value.schema),
                        )
                        .await?;
                    topology_seed_refs.push((bound_ref, "explicit_schema".into()));
                }
                Cue::Relation(value) => {
                    for reference in [&value.from, &value.to] {
                        let (bound_ref, _, _) = self
                            .store
                            .bind_exact_reference(query.subject, reference)
                            .await?;
                        topology_seed_refs.push((bound_ref, "relation_cue".into()));
                    }
                }
                _ => {}
            }
        }

        let revision_policy = if allowed_revision_refs.is_empty() {
            RevisionPolicy::CurrentOnly
        } else {
            RevisionPolicy::ExactHistorical {
                allowed_revision_refs,
            }
        };
        let enabled_lanes = planned_lanes(&query);
        if enabled_lanes.is_empty() {
            return Err(Error::Invalid("query has no enabled retrieval lane".into()));
        }
        let lane_budgets = budget_values(&query, &enabled_lanes, &retrieval_policy);
        let bound_at_authority_seq = self.store.authority_seq(query.subject).await?;
        let exact_target_bypasses_auto_level = !exact_bindings.is_empty();
        Ok(BoundQuery {
            query_id: Uuid::now_v7(),
            source_query: query.clone(),
            bound_at_authority_seq,
            revision_policy,
            exact_bindings,
            runtime_refs,
            topology_seed_refs,
            enabled_lanes: enabled_lanes.clone(),
            lane_budgets,
            hard_constraints: query.constraints.clone(),
            accessibility_policy: AccessibilityQueryPolicy {
                effort: query.effort,
                exact_target_bypasses_auto_level,
            },
            selected_embedding_space: None,
            topology_required: explicit_topology(&query),
            fusion_version: "rrf-v1".into(),
            rerank_policy: RerankPolicy {
                strength: query.capabilities.text_rerank,
                enabled: query.capabilities.text_rerank == RequirementStrength::Required,
                top_n: (query.result_need.limit.saturating_mul(4)).clamp(12, 50),
            },
            config_snapshot,
            retrieval_policy,
        })
    }
}
