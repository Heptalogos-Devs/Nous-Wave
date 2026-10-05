use crate::observation::NATIVE_MECHANISM_ID;
use crate::{QueryObservation, ServingSnapshot, SourceSeed};
use nous_core::{CognitiveRef, Cue, EvidenceFamily};
use nous_runtime::{
    BoundQuery, LaneCandidate, LaneOutput, LaneStatus, QueryPlan, TopologyWorkSummary,
};
use std::collections::{BTreeMap, VecDeque};

pub(crate) fn topology_lane(
    snapshot: &ServingSnapshot,
    bound: &BoundQuery,
    plan: &QueryPlan,
    signals: &crate::PreparedQuerySignals,
) -> nous_core::Result<LaneOutput> {
    let mut output = LaneOutput::empty(EvidenceFamily::TopologyWave, LaneStatus::Ready);
    if !plan.expand_topology {
        output.status = LaneStatus::Disabled;
        return Ok(output);
    }
    let Some(graph) = snapshot.topology.as_ref() else {
        output.status = LaneStatus::Unavailable;
        output
            .diagnostics
            .push("topology serving generation is unavailable".into());
        return Ok(output);
    };
    if graph.cognitive_profile != plan.cognitive_profile {
        output.status = LaneStatus::Unavailable;
        output
            .diagnostics
            .push("topology generation does not match bound cognitive profile".into());
        return Ok(output);
    }
    if plan.cognitive_profile.requirements().query_embedding && signals.embedding().is_none() {
        output.status = LaneStatus::Unavailable;
        output
            .diagnostics
            .push("bound cognitive profile requires an available permitted query embedding".into());
        return Ok(output);
    }
    if plan.cognitive_profile != nous_runtime::CognitiveProfile::NousNodePotential {
        output.status = LaneStatus::Unavailable;
        output.diagnostics.push(format!(
            "cognitive kernel not implemented: {}",
            plan.cognitive_profile.id()
        ));
        return Ok(output);
    }
    output.generation_ref = Some(graph.generation_id);
    let observation = QueryObservation::native(graph, bound, plan, source_seeds(graph, bound))?;
    let river = observation.river();
    output.topology_work = Some(TopologyWorkSummary {
        mechanism: NATIVE_MECHANISM_ID.into(),
        profile_id: observation.profile_id().into(),
        profile_digest: observation.profile_digest().into(),
        activated_edges: river.edges.len(),
        max_hop_observed: river.max_hop_observed,
        seed_count: observation.source_seeds().len(),
        visited_nodes: river.node_potential.len(),
        complete: river.complete,
        discarded_mass: Some(river.discarded_state_mass),
    });
    if !river.complete {
        output.status = LaneStatus::Truncated;
    }
    let routes = activated_routes(river, observation.source_seeds());
    let mut values = river
        .node_potential
        .iter()
        .filter_map(|(node, potential)| {
            graph
                .nodes
                .get(*node as usize)
                .map(|value| (value.reference.clone(), *potential, *node))
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
        .map(|(index, (reference, potential, node))| LaneCandidate {
            reference,
            rank: (index + 1) as u32,
            variants: vec!["topology:wave".into()],
            provider_metadata: serde_json::json!({
                "mechanism": NATIVE_MECHANISM_ID,
                "node_potential": potential,
                "complete": river.complete,
                "activated_route": routes.get(&node).map(|path| path.iter().map(|id|
                    graph.nodes[*id as usize].reference.to_string()).collect::<Vec<_>>()),
                "route_hop_limit": river.max_hops.min(32),
                "route_seed": routes.get(&node).and_then(|path| path.first()).map(|seed_node|
                    observation.source_seeds().iter().filter(|seed| seed.node == *seed_node).collect::<Vec<_>>()),
                "route_evidence": routes.get(&node).map(|path| path.windows(2).map(|pair| {
                    let support = graph.edge_evidence(pair[0], pair[1]).take(16).collect::<Vec<_>>();
                    serde_json::json!({
                        "from": graph.nodes[pair[0] as usize].reference,
                        "to": graph.nodes[pair[1] as usize].reference,
                        "flow": river.edges.iter().find(|edge| edge.from == pair[0] && edge.to == pair[1]).map(|edge| edge.flow),
                        "support": support,
                        "support_truncated": graph.edge_evidence(pair[0], pair[1]).count() > 16,
                    })
                }).collect::<Vec<_>>()),
                "route_semantics": "shortest witnessed route through activated edges; not score attribution",
                "provenance": river.provenance.iter().find(|value| value.node == node),
            }),
        })
        .collect();
    Ok(output)
}

fn source_seeds(graph: &crate::WaveGraphGeneration, bound: &BoundQuery) -> Vec<SourceSeed> {
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
    for cue in &bound.source_query.expression.cues {
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
    let mut merged = BTreeMap::<(u32, String), SourceSeed>::new();
    for (node, weight, family) in seeds {
        merged
            .entry((node, family.into()))
            .and_modify(|seed| seed.weight += weight)
            .or_insert(SourceSeed {
                node,
                weight,
                seed_family: family.into(),
                origin_cue: format!("{family}:{}", graph.nodes[node as usize].reference),
                hop_zero: true,
            });
    }
    merged.into_values().collect()
}

// Multi-source breadth-first traversal visits each activated node once. A route
// witnesses connectivity in this observation; it does not explain every unit of
// accumulated potential or invent relation provenance absent from the artifact.
fn activated_routes(river: &crate::QueryRiver, seeds: &[SourceSeed]) -> BTreeMap<u32, Vec<u32>> {
    let mut outgoing = BTreeMap::<u32, Vec<u32>>::new();
    for edge in &river.edges {
        if edge.flow > 0.0 {
            outgoing.entry(edge.from).or_default().push(edge.to);
        }
    }
    for targets in outgoing.values_mut() {
        targets.sort_unstable();
        targets.dedup();
    }
    let mut routes = BTreeMap::new();
    let mut queue = VecDeque::new();
    for seed in seeds {
        if seed.weight > 0.0 && !routes.contains_key(&seed.node) {
            routes.insert(seed.node, vec![seed.node]);
            queue.push_back(seed.node);
        }
    }
    while let Some(node) = queue.pop_front() {
        let path = routes[&node].clone();
        if path.len() > river.max_hops.min(32) {
            continue;
        }
        for target in outgoing.get(&node).into_iter().flatten() {
            if !routes.contains_key(target) {
                let mut next = path.clone();
                next.push(*target);
                routes.insert(*target, next);
                queue.push_back(*target);
            }
        }
    }
    routes
}
