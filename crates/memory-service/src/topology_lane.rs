use crate::*;
use nous_cognitive_runtime::{BoundQuery, QueryPlan};
use nous_memory_retrieval::{SourceSeed, propagate_with_budget};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

pub(crate) fn topology_ranks(
    snapshot: &nous_memory_retrieval::ServingSnapshot,
    bound: &BoundQuery,
    plan: &QueryPlan,
    lane_ranks: &BTreeMap<Uuid, HashMap<EvidenceFamily, usize>>,
) -> HashMap<CognitiveRef, usize> {
    let Some(graph) = snapshot.topology.as_ref() else {
        return HashMap::new();
    };
    let query = &bound.source_query;
    let mut seeds = Vec::new();
    for binding in &bound.exact_bindings {
        if let Some(node) = graph.node_id(&binding.bound_ref) {
            seeds.push((node, 1.0, "exact_target"));
        }
    }
    for reference in &bound.runtime_refs {
        if let Some(node) = graph.node_id(reference) {
            seeds.push((node, 0.85, "runtime_situation"));
        }
    }
    for (reference, family) in &bound.topology_seed_refs {
        if let Some(node) = graph.node_id(reference) {
            seeds.push((node, 1.0, family.as_str()));
        }
    }
    for cue in &query.cues {
        let (reference, weight, family) = match cue {
            Cue::Entity(value) => (
                CognitiveRef::Entity(value.entity_ref.clone()),
                0.90,
                "entity_cue",
            ),
            Cue::Tag(value) => (CognitiveRef::Tag(value.tag), 0.75, "tag_cue"),
            Cue::Schema(_) => continue,
            Cue::Relation(value) => (value.from.clone(), 1.0, "relation_cue"),
            _ => continue,
        };
        if let Some(node) = graph.node_id(&reference) {
            seeds.push((node, weight, family));
        }
    }
    for (revision, ranks) in lane_ranks {
        let promoted = ranks.contains_key(&EvidenceFamily::Lexical)
            || ranks.contains_key(&EvidenceFamily::Dense);
        if promoted {
            let reference = CognitiveRef::MemoryRevision(MemoryRevisionId(*revision));
            if let Some(node) = graph.node_id(&reference) {
                let (weight, family) = if ranks.contains_key(&EvidenceFamily::Lexical) {
                    (0.60, "lexical_promoted")
                } else {
                    (0.60, "dense_promoted")
                };
                seeds.push((node, weight, family));
            }
        }
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
    let seeds = merged.into_values().collect::<Vec<_>>();
    if seeds.is_empty() {
        return HashMap::new();
    }
    let river = propagate_with_budget(graph, &seeds, plan.topology_rounds, plan.topology_nodes);
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
