//! Owner-direct lanes start from the frozen revision selection, never current heads.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
use super::*;
use nous_runtime::{
    BoundQuery, CognitiveContributor, LaneCandidate, LaneOutput, LaneStatus, QueryPlan,
};
use std::collections::{BTreeSet, HashSet};

pub(super) async fn direct_lanes(
    service: &MemoryService,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> Result<Vec<LaneOutput>> {
    let view = bound
        .historical_authority
        .as_deref()
        .ok_or_else(|| Error::Invalid("historical Authority view required".into()))?;
    let references: Vec<_> = view
        .cognition
        .iter()
        .flat_map(|state| state.revisions.iter().cloned())
        .collect();
    let (mut hits, _) = service
        .validate_and_materialize(view.subject, &references, bound)
        .await?;
    hits.sort_by(|a, b| {
        b.freshness
            .recorded_at
            .cmp(&a.freshness.recorded_at)
            .then_with(|| a.reference.to_string().cmp(&b.reference.to_string()))
    });
    let query = &bound.source_query;
    let entities: HashSet<_> = query
        .expression
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Entity(cue) => Some(cue.entity_ref.clone()),
            _ => None,
        })
        .chain(
            query
                .expression
                .constraints
                .entity_requirements
                .iter()
                .cloned(),
        )
        .chain(
            query
                .expression
                .targets
                .iter()
                .filter_map(|target| match target {
                    QueryTarget::EntityNeighborhood { entity_ref } => Some(entity_ref.clone()),
                    _ => None,
                }),
        )
        .collect();
    let mut outputs = Vec::new();
    if bound.lane_enabled(EvidenceFamily::Entity) {
        let candidates = hits
            .iter()
            .filter(|hit| {
                hit.entity_refs
                    .iter()
                    .any(|entity| entities.contains(entity))
            })
            .map(|hit| hit.reference.clone())
            .collect();
        outputs.push(lane(
            EvidenceFamily::Entity,
            candidates,
            plan,
            "entity:historical_aboutness",
        ));
    }
    if bound.lane_enabled(EvidenceFamily::Temporal) {
        let c = &query.expression.constraints;
        let candidates = if [c.occurred, c.observed, c.valid, c.formed, c.recorded]
            .iter()
            .any(Option::is_some)
        {
            hits.iter().map(|hit| hit.reference.clone()).collect()
        } else {
            vec![]
        };
        outputs.push(lane(
            EvidenceFamily::Temporal,
            candidates,
            plan,
            "temporal:historical_revision",
        ));
    }
    if bound.lane_enabled(EvidenceFamily::SchemaDirect) {
        let candidates = schema_candidates(service, bound, view, &hits).await?;
        outputs.push(lane(
            EvidenceFamily::SchemaDirect,
            candidates,
            plan,
            "schema:historical_support",
        ));
    }
    Ok(outputs)
}
async fn schema_candidates(
    service: &MemoryService,
    bound: &BoundQuery,
    view: &HistoricalAuthoritySnapshot,
    hits: &[CognitiveHit],
) -> Result<Vec<CognitiveRef>> {
    let query = &bound.source_query;
    let schemas: HashSet<_> = query
        .expression
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Schema(cue) => Some(CognitiveRef::CognitiveSchema(cue.schema)),
            _ => None,
        })
        .chain(
            query
                .expression
                .targets
                .iter()
                .filter_map(|target| match target {
                    QueryTarget::SchemaNeighborhood { schema } => {
                        Some(CognitiveRef::CognitiveSchema(*schema))
                    }
                    _ => None,
                }),
        )
        .collect();
    let roots: HashSet<_> = view
        .cognition
        .iter()
        .filter(|state| schemas.contains(&state.object))
        .flat_map(|state| state.revisions.iter().cloned())
        .chain(
            bound
                .exact_bindings
                .iter()
                .filter_map(|binding| match binding.bound_ref {
                    CognitiveRef::CognitiveSchemaRevision(_) => Some(binding.bound_ref.clone()),
                    _ => None,
                }),
        )
        .collect();
    let mut candidates: BTreeSet<_> = hits
        .iter()
        .filter(|hit| roots.contains(&hit.reference))
        .map(|hit| hit.reference.to_string())
        .collect();
    if !roots.is_empty() {
        let input = service
            .store
            .historical_projection_input(view, bound.config_snapshot.get(EPISODE_SYNOPSIS)?)
            .await?;
        for edge in input.topology.edges {
            if edge.association_kind == "schema_support" && roots.contains(&edge.from) {
                candidates.insert(edge.to.to_string());
            }
        }
    }
    let candidates = candidates
        .into_iter()
        .map(|text| {
            let (kind, value) = text
                .split_once(':')
                .ok_or_else(|| Error::Infrastructure("invalid candidate reference".into()))?;
            parse_reference(kind, value)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(candidates)
}

fn lane(
    family: EvidenceFamily,
    references: Vec<CognitiveRef>,
    plan: &QueryPlan,
    variant: &str,
) -> LaneOutput {
    let budget = plan.lane_budget(family);
    let mut output = LaneOutput::empty(
        family,
        if references.len() > budget {
            LaneStatus::Truncated
        } else {
            LaneStatus::Ready
        },
    );
    output.candidates = references
        .into_iter()
        .take(budget)
        .enumerate()
        .map(|(rank, reference)| LaneCandidate {
            reference,
            rank: (rank + 1) as u32,
            variants: vec![variant.into()],
            provider_metadata: serde_json::Value::Null,
        })
        .collect();
    output
}
