use crate::{ServingSnapshot, SourceSeed, propagate_with_budget};
use nous_core::{CognitiveRef, Cue, EvidenceFamily};
use nous_runtime::{BoundQuery, LaneCandidate, LaneOutput, LaneStatus, QueryPlan};
use std::collections::HashMap;

pub(crate) fn topology_lane(
    snapshot: &ServingSnapshot,
    bound: &BoundQuery,
    plan: &QueryPlan,
) -> LaneOutput {
    let mut output = LaneOutput::empty(EvidenceFamily::TopologyWave, LaneStatus::Ready);
    let Some(graph) = snapshot.topology.as_ref() else {
        output.status = LaneStatus::Unavailable;
        output
            .diagnostics
            .push("topology serving generation is unavailable".into());
        return output;
    };
    output.generation_ref = Some(graph.generation_id);
    let seed_weight = |name: &str, default: f64| {
        graph
            .config
            .seed_weights
            .get(name)
            .copied()
            .unwrap_or(default)
    };
    let mut seeds = Vec::new();
    for binding in &bound.exact_bindings {
        if let Some(node) = graph.node_id(&binding.bound_ref) {
            seeds.push((node, seed_weight("exact_target", 1.0), "exact_target"));
        }
    }
    for reference in &bound.runtime_refs {
        if let Some(node) = graph.node_id(reference) {
            seeds.push((
                node,
                seed_weight("runtime_situation", 0.85),
                "runtime_situation",
            ));
        }
    }
    for (reference, family) in &bound.topology_seed_refs {
        if let Some(node) = graph.node_id(reference) {
            seeds.push((node, seed_weight(family.as_str(), 1.0), family.as_str()));
        }
    }
    for cue in &bound.source_query.cues {
        let (reference, weight, family) = match cue {
            Cue::Entity(value) => (
                CognitiveRef::Entity(value.entity_ref.clone()),
                seed_weight("entity_cue", 0.90),
                "entity_cue",
            ),
            Cue::Tag(value) => (
                CognitiveRef::Tag(value.tag),
                seed_weight("tag_cue", 0.75),
                "tag_cue",
            ),
            Cue::Relation(value) => (
                value.from.clone(),
                seed_weight("relation_cue", 1.0),
                "relation_cue",
            ),
            _ => continue,
        };
        if let Some(node) = graph.node_id(&reference) {
            seeds.push((node, weight, family));
        }
    }
    if seeds.is_empty() {
        return output;
    }
    let mut merged = HashMap::<u32, SourceSeed>::new();
    for (node, weight, family) in seeds {
        merged
            .entry(node)
            .and_modify(|seed| seed.weight += weight)
            .or_insert(SourceSeed {
                node,
                weight,
                seed_family: family.into(),
                origin_cue: family.into(),
                hop_zero: true,
            });
    }
    let river = propagate_with_budget(
        graph,
        &merged.into_values().collect::<Vec<_>>(),
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
    output.candidates = values
        .into_iter()
        .take(plan.lane_budget(EvidenceFamily::TopologyWave))
        .enumerate()
        .map(|(index, (reference, _))| LaneCandidate {
            reference,
            rank: (index + 1) as u32,
            variants: vec!["topology:wave".into()],
            provider_metadata: serde_json::Value::Null,
        })
        .collect();
    output
}
