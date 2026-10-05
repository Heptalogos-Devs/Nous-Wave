use crate::*;
use nous_configuration::{
    ConfigApplyMode, ConfigExposure, ConfigKey, ConfigRegistryBuilder, ConfigScopePolicy,
    ConfigSemanticEffect, ConfigSnapshot,
};
use nous_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveNodeKind {
    Memory,
    Tag,
    Schema,
    Entity,
    Resource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveNode {
    pub serving_id: u32,
    pub reference: CognitiveRef,
    pub node_kind: WaveNodeKind,
    pub embedding_key: Option<u64>,
    pub posting_key: Option<u64>,
    pub intrinsic_residual_gain: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveEdgeEvidence {
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub support_class: String,
    pub association_kind: String,
    pub polarity: String,
    pub support_mass: f64,
    #[serde(default)]
    pub provenance_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveConfig {
    pub hub_beta: f64,
    pub hub_penalty_min: f64,
    pub hub_penalty_max: f64,
    pub outbound_budget: f64,
    pub max_hops: usize,
    pub max_states: usize,
    pub max_neighbors_per_node: usize,
    pub minimum_state_energy: f64,
    pub immediate_return_multiplier: f64,
    pub initial_budget_steps: u32,
    pub normal_edge_cost: u32,
    pub fir_gamma: f64,
    #[serde(default)]
    pub class_quality: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub seed_weights: std::collections::BTreeMap<String, f64>,
}

pub const HUB_BETA_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.hub_beta");
pub const HUB_PENALTY_MIN_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.hub_penalty_min");
pub const HUB_PENALTY_MAX_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.hub_penalty_max");
pub const OUTBOUND_BUDGET_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.outbound_budget");
pub const MAX_HOPS_KEY: ConfigKey<usize> = ConfigKey::new("topology.wave.max_hops");
pub const MAX_STATES_KEY: ConfigKey<usize> = ConfigKey::new("topology.wave.max_states");
pub const MAX_NEIGHBORS_KEY: ConfigKey<usize> =
    ConfigKey::new("topology.wave.max_neighbors_per_node");
pub const MINIMUM_STATE_ENERGY_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.minimum_state_energy");
pub const IMMEDIATE_RETURN_MULTIPLIER_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.immediate_return_multiplier");
pub const INITIAL_BUDGET_STEPS_KEY: ConfigKey<u32> =
    ConfigKey::new("topology.wave.initial_budget_steps");
pub const NORMAL_EDGE_COST_KEY: ConfigKey<u32> = ConfigKey::new("topology.wave.normal_edge_cost");
pub const FIR_GAMMA_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.fir_gamma");
pub const QUALITY_HOST_EXPLICIT_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.quality.host_explicit");
pub const QUALITY_SOURCE_EVIDENCE_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.quality.source_evidence");
pub const QUALITY_COGNITIVE_DERIVATION_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.quality.cognitive_derivation");
pub const QUALITY_DERIVED_STRUCTURE_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.quality.derived_structure");
pub const QUALITY_MEANINGFUL_USE_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.quality.meaningful_use");
pub const SEED_EXACT_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.seed_weights.exact_target");
pub const SEED_RUNTIME_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.seed_weights.runtime_situation");
pub const SEED_RELATION_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.seed_weights.relation_cue");
pub const SEED_ENTITY_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.seed_weights.entity_cue");
pub const SEED_TAG_KEY: ConfigKey<f64> = ConfigKey::new("topology.wave.seed_weights.tag_cue");
pub const SEED_LEXICAL_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.seed_weights.lexical_promoted");
pub const SEED_DENSE_KEY: ConfigKey<f64> =
    ConfigKey::new("topology.wave.seed_weights.dense_promoted");

#[expect(
    clippy::too_many_lines,
    reason = "Wave registration keeps the complete reference policy catalog in one owner boundary"
)]
pub fn register_wave_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    let reference = nous_configuration::ReferenceProfile::parse(include_str!(
        "../../../config/reference/topology-wave-v1.json"
    ))?;
    let finite = |value: &f64| {
        if value.is_finite() {
            Ok(())
        } else {
            Err(Error::Invalid("Wave value must be finite".into()))
        }
    };
    let positive_usize = |value: &usize| {
        if *value > 0 {
            Ok(())
        } else {
            Err(Error::Invalid("Wave bound must be positive".into()))
        }
    };
    let positive_u32 = |value: &u32| {
        if *value > 0 {
            Ok(())
        } else {
            Err(Error::Invalid("Wave bound must be positive".into()))
        }
    };
    macro_rules! float {
        ($key:expr, $default:expr, $description:expr) => {
            registry.register(
                $key,
                "cognitive-retrieval",
                $description,
                $default,
                ConfigExposure::Developer,
                ConfigScopePolicy::SystemOnly,
                ConfigApplyMode::ServingRebuild,
                ConfigSemanticEffect::ServingProjection,
                finite,
            )?;
        };
    }
    macro_rules! usize_key {
        ($key:expr, $default:expr, $description:expr) => {
            registry.register(
                $key,
                "cognitive-retrieval",
                $description,
                $default,
                ConfigExposure::Developer,
                ConfigScopePolicy::SystemOnly,
                ConfigApplyMode::ServingRebuild,
                ConfigSemanticEffect::ServingProjection,
                positive_usize,
            )?;
        };
    }
    macro_rules! u32_key {
        ($key:expr, $default:expr, $description:expr) => {
            registry.register(
                $key,
                "cognitive-retrieval",
                $description,
                $default,
                ConfigExposure::Developer,
                ConfigScopePolicy::SystemOnly,
                ConfigApplyMode::ServingRebuild,
                ConfigSemanticEffect::ServingProjection,
                positive_u32,
            )?;
        };
    }
    float!(
        HUB_BETA_KEY,
        reference.get(HUB_BETA_KEY)?,
        "Wave hub penalty exponent."
    );
    float!(
        HUB_PENALTY_MIN_KEY,
        reference.get(HUB_PENALTY_MIN_KEY)?,
        "Wave hub penalty minimum."
    );
    float!(
        HUB_PENALTY_MAX_KEY,
        reference.get(HUB_PENALTY_MAX_KEY)?,
        "Wave hub penalty maximum."
    );
    float!(
        OUTBOUND_BUDGET_KEY,
        reference.get(OUTBOUND_BUDGET_KEY)?,
        "Wave outbound conductance budget."
    );
    usize_key!(
        MAX_HOPS_KEY,
        reference.get(MAX_HOPS_KEY)?,
        "Wave maximum hops."
    );
    usize_key!(
        MAX_STATES_KEY,
        reference.get(MAX_STATES_KEY)?,
        "Wave maximum states."
    );
    usize_key!(
        MAX_NEIGHBORS_KEY,
        reference.get(MAX_NEIGHBORS_KEY)?,
        "Wave maximum neighbors per node."
    );
    float!(
        MINIMUM_STATE_ENERGY_KEY,
        reference.get(MINIMUM_STATE_ENERGY_KEY)?,
        "Wave minimum state energy."
    );
    float!(
        IMMEDIATE_RETURN_MULTIPLIER_KEY,
        reference.get(IMMEDIATE_RETURN_MULTIPLIER_KEY)?,
        "Wave immediate return multiplier."
    );
    u32_key!(
        INITIAL_BUDGET_STEPS_KEY,
        reference.get(INITIAL_BUDGET_STEPS_KEY)?,
        "Wave initial budget steps."
    );
    u32_key!(
        NORMAL_EDGE_COST_KEY,
        reference.get(NORMAL_EDGE_COST_KEY)?,
        "Wave normal edge cost."
    );
    float!(
        FIR_GAMMA_KEY,
        reference.get(FIR_GAMMA_KEY)?,
        "Wave finite impulse response gamma."
    );
    float!(
        QUALITY_HOST_EXPLICIT_KEY,
        reference.get(QUALITY_HOST_EXPLICIT_KEY)?,
        "Wave host-explicit edge quality."
    );
    float!(
        QUALITY_SOURCE_EVIDENCE_KEY,
        reference.get(QUALITY_SOURCE_EVIDENCE_KEY)?,
        "Wave source-evidence edge quality."
    );
    float!(
        QUALITY_COGNITIVE_DERIVATION_KEY,
        reference.get(QUALITY_COGNITIVE_DERIVATION_KEY)?,
        "Wave cognitive-derivation edge quality."
    );
    float!(
        QUALITY_DERIVED_STRUCTURE_KEY,
        reference.get(QUALITY_DERIVED_STRUCTURE_KEY)?,
        "Wave derived-structure edge quality."
    );
    float!(
        QUALITY_MEANINGFUL_USE_KEY,
        reference.get(QUALITY_MEANINGFUL_USE_KEY)?,
        "Wave meaningful-use edge quality."
    );
    float!(
        SEED_EXACT_KEY,
        reference.get(SEED_EXACT_KEY)?,
        "Wave exact-target seed weight."
    );
    float!(
        SEED_RUNTIME_KEY,
        reference.get(SEED_RUNTIME_KEY)?,
        "Wave runtime seed weight."
    );
    float!(
        SEED_RELATION_KEY,
        reference.get(SEED_RELATION_KEY)?,
        "Wave relation-cue seed weight."
    );
    float!(
        SEED_ENTITY_KEY,
        reference.get(SEED_ENTITY_KEY)?,
        "Wave entity-cue seed weight."
    );
    float!(
        SEED_TAG_KEY,
        reference.get(SEED_TAG_KEY)?,
        "Wave tag-cue seed weight."
    );
    float!(
        SEED_LEXICAL_KEY,
        reference.get(SEED_LEXICAL_KEY)?,
        "Wave lexical-promotion seed weight."
    );
    float!(
        SEED_DENSE_KEY,
        reference.get(SEED_DENSE_KEY)?,
        "Wave dense-promotion seed weight."
    );
    reference.describe(registry)?;
    Ok(())
}

pub fn resolve_wave_config(snapshot: &ConfigSnapshot) -> Result<WaveConfig> {
    let config = WaveConfig {
        hub_beta: snapshot.get(HUB_BETA_KEY)?,
        hub_penalty_min: snapshot.get(HUB_PENALTY_MIN_KEY)?,
        hub_penalty_max: snapshot.get(HUB_PENALTY_MAX_KEY)?,
        outbound_budget: snapshot.get(OUTBOUND_BUDGET_KEY)?,
        max_hops: snapshot.get(MAX_HOPS_KEY)?,
        max_states: snapshot.get(MAX_STATES_KEY)?,
        max_neighbors_per_node: snapshot.get(MAX_NEIGHBORS_KEY)?,
        minimum_state_energy: snapshot.get(MINIMUM_STATE_ENERGY_KEY)?,
        immediate_return_multiplier: snapshot.get(IMMEDIATE_RETURN_MULTIPLIER_KEY)?,
        initial_budget_steps: snapshot.get(INITIAL_BUDGET_STEPS_KEY)?,
        normal_edge_cost: snapshot.get(NORMAL_EDGE_COST_KEY)?,
        fir_gamma: snapshot.get(FIR_GAMMA_KEY)?,
        class_quality: std::collections::BTreeMap::from([
            (
                "host_explicit".into(),
                snapshot.get(QUALITY_HOST_EXPLICIT_KEY)?,
            ),
            (
                "source_evidence".into(),
                snapshot.get(QUALITY_SOURCE_EVIDENCE_KEY)?,
            ),
            (
                "cognitive_derivation".into(),
                snapshot.get(QUALITY_COGNITIVE_DERIVATION_KEY)?,
            ),
            (
                "derived_structure".into(),
                snapshot.get(QUALITY_DERIVED_STRUCTURE_KEY)?,
            ),
            (
                "meaningful_use".into(),
                snapshot.get(QUALITY_MEANINGFUL_USE_KEY)?,
            ),
        ]),
        seed_weights: std::collections::BTreeMap::from([
            ("exact_target".into(), snapshot.get(SEED_EXACT_KEY)?),
            ("runtime_situation".into(), snapshot.get(SEED_RUNTIME_KEY)?),
            ("relation_cue".into(), snapshot.get(SEED_RELATION_KEY)?),
            ("entity_cue".into(), snapshot.get(SEED_ENTITY_KEY)?),
            ("tag_cue".into(), snapshot.get(SEED_TAG_KEY)?),
            ("lexical_promoted".into(), snapshot.get(SEED_LEXICAL_KEY)?),
            ("dense_promoted".into(), snapshot.get(SEED_DENSE_KEY)?),
        ]),
    };
    config.validate()?;
    if config.hub_penalty_min > config.hub_penalty_max {
        return Err(Error::Invalid(
            "Wave hub penalty bounds are inverted".into(),
        ));
    }
    Ok(config)
}

impl Default for WaveConfig {
    fn default() -> Self {
        let mut registry = ConfigRegistryBuilder::new();
        register_wave_configuration(&mut registry).expect("Wave reference catalog");
        resolve_wave_config(
            &registry
                .finish()
                .expect("Wave catalog")
                .reference_snapshot()
                .expect("Wave reference snapshot"),
        )
        .expect("Wave reference policy")
    }
}

impl WaveConfig {
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.outbound_budget)
            || self.max_hops == 0
            || self.max_states == 0
            || self.max_neighbors_per_node == 0
            || !(0.0..1.0).contains(&self.immediate_return_multiplier)
            || !(0.0..1.0).contains(&self.fir_gamma)
            || self.normal_edge_cost == 0
            || self.initial_budget_steps == 0
            || self
                .class_quality
                .values()
                .any(|value| !value.is_finite() || *value < 0.0)
            || self
                .seed_weights
                .values()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(Error::Invalid("invalid WaveConfig bounds".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WaveGraphGeneration {
    pub generation_id: ServingGenerationId,
    pub cognitive_profile: nous_runtime::CognitiveProfile,
    pub nodes: Vec<WaveNode>,
    node_by_ref: std::collections::HashMap<CognitiveRef, u32>,
    adjacency: Vec<Vec<(u32, f64)>>,
    raw_support: std::collections::HashMap<(u32, u32), f64>,
    evidence: std::collections::BTreeMap<(u32, u32), Vec<WaveEdgeEvidence>>,
    pub config: WaveConfig,
}

impl WaveGraphGeneration {
    pub fn artifact(&self) -> TopologyArtifact {
        let mut edges = Vec::new();
        for (from, row) in self.adjacency.iter().enumerate() {
            for (to, weight) in row {
                edges.push((
                    from as u32,
                    *to,
                    *weight,
                    *self.raw_support.get(&(from as u32, *to)).unwrap_or(&0.0),
                ));
            }
        }
        TopologyArtifact {
            generation_id: self.generation_id,
            cognitive_profile: self.cognitive_profile,
            nodes: self.nodes.clone(),
            evidence: self.evidence.values().flatten().cloned().collect(),
            edges,
            config: self.config.clone(),
        }
    }

    pub fn from_artifact(artifact: TopologyArtifact) -> Result<Self> {
        artifact.config.validate()?;
        let mut adjacency = vec![Vec::new(); artifact.nodes.len()];
        let mut raw_support = std::collections::HashMap::new();
        for (from, to, weight, raw) in artifact.edges {
            if from as usize >= artifact.nodes.len()
                || to as usize >= artifact.nodes.len()
                || !weight.is_finite()
                || weight < 0.0
                || !raw.is_finite()
            {
                return Err(Error::Infrastructure("topology artifact is corrupt".into()));
            }
            adjacency[from as usize].push((to, weight));
            raw_support.insert((from, to), raw);
        }
        let node_by_ref = artifact
            .nodes
            .iter()
            .map(|node| (node.reference.clone(), node.serving_id))
            .collect::<std::collections::HashMap<_, _>>();
        let evidence = index_evidence(artifact.evidence, &node_by_ref);
        Ok(Self {
            generation_id: artifact.generation_id,
            cognitive_profile: artifact.cognitive_profile,
            nodes: artifact.nodes,
            evidence,
            node_by_ref,
            adjacency,
            raw_support,
            config: artifact.config,
        })
    }

    pub fn build(
        mut nodes: Vec<WaveNode>,
        evidence: &[WaveEdgeEvidence],
        config: WaveConfig,
    ) -> Result<Self> {
        config.validate()?;
        nodes.sort_by_key(|node| node.serving_id);
        for (index, node) in nodes.iter_mut().enumerate() {
            node.serving_id = index as u32;
        }
        let node_by_ref = nodes
            .iter()
            .map(|node| (node.reference.clone(), node.serving_id))
            .collect::<std::collections::HashMap<_, _>>();
        let mut by_root = std::collections::HashMap::<(u32, u32, Option<String>), f64>::new();
        for item in evidence {
            if item.polarity.eq_ignore_ascii_case("negative")
                || !supported_relation(&item.association_kind)
                || !item.support_mass.is_finite()
                || item.support_mass <= 0.0
            {
                continue;
            }
            let (Some(&from), Some(&to)) = (node_by_ref.get(&item.from), node_by_ref.get(&item.to))
            else {
                continue;
            };
            let quality =
                class_quality(&item.support_class, &config.class_quality) * item.support_mass;
            let entry = by_root
                .entry((from, to, item.provenance_root.clone()))
                .or_default();
            *entry = entry.max(quality);
        }
        let mut aggregate = std::collections::HashMap::<(u32, u32), f64>::new();
        for ((from, to, _root), mass) in by_root {
            *aggregate.entry((from, to)).or_default() += mass;
        }
        let mut inflow = vec![0.0; nodes.len()];
        for (&(_, to), mass) in &aggregate {
            inflow[to as usize] += *mass;
        }
        let mut positive = inflow
            .iter()
            .copied()
            .filter(|value| *value > 0.0)
            .collect::<Vec<_>>();
        positive.sort_by(f64::total_cmp);
        let median = positive
            .get(positive.len() / 2)
            .copied()
            .unwrap_or(1.0)
            .max(f64::EPSILON);
        let mut rows = vec![Vec::new(); nodes.len()];
        let mut raw_support = std::collections::HashMap::new();
        for (&(from, to), &raw) in &aggregate {
            let penalty = (inflow[to as usize] / median)
                .powf(-config.hub_beta)
                .clamp(config.hub_penalty_min, config.hub_penalty_max);
            rows[from as usize].push((to, raw * penalty, raw));
        }
        let mut adjacency = vec![Vec::new(); nodes.len()];
        for (from, row) in rows.into_iter().enumerate() {
            let mut row = row;
            row.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            row.truncate(config.max_neighbors_per_node);
            let total: f64 = row.iter().map(|(_, weight, _)| *weight).sum();
            for (to, weight, raw) in row {
                let conductance = if total > 0.0 {
                    config.outbound_budget * weight / total
                } else {
                    0.0
                };
                if conductance > 0.0 {
                    adjacency[from].push((to, conductance));
                    raw_support.insert((from as u32, to), raw);
                }
            }
        }
        let retained_evidence = evidence
            .iter()
            .filter(|item| {
                let (Some(from), Some(to)) =
                    (node_by_ref.get(&item.from), node_by_ref.get(&item.to))
                else {
                    return false;
                };
                raw_support.contains_key(&(*from, *to))
                    && !item.polarity.eq_ignore_ascii_case("negative")
                    && supported_relation(&item.association_kind)
                    && item.support_mass.is_finite()
                    && item.support_mass > 0.0
            })
            .cloned()
            .collect();
        Ok(Self {
            generation_id: ServingGenerationId::new(),
            evidence: index_evidence(retained_evidence, &node_by_ref),
            cognitive_profile: nous_runtime::CognitiveProfile::default(),
            nodes,
            node_by_ref,
            adjacency,
            raw_support,
            config,
        })
    }

    pub fn edge_evidence(&self, from: u32, to: u32) -> impl Iterator<Item = &WaveEdgeEvidence> {
        self.evidence.get(&(from, to)).into_iter().flatten()
    }
    pub fn node_id(&self, reference: &CognitiveRef) -> Option<u32> {
        self.node_by_ref.get(reference).copied()
    }
    pub fn outgoing(&self, node: u32) -> Vec<(u32, f64)> {
        self.adjacency
            .get(node as usize)
            .cloned()
            .unwrap_or_default()
    }
    pub fn outbound_mass(&self, node: u32) -> f64 {
        self.outgoing(node).iter().map(|(_, mass)| *mass).sum()
    }
    pub fn edge_raw_support(&self, from: u32, to: u32) -> Option<f64> {
        self.raw_support.get(&(from, to)).copied()
    }
}

fn index_evidence(
    evidence: Vec<WaveEdgeEvidence>,
    nodes: &std::collections::HashMap<CognitiveRef, u32>,
) -> std::collections::BTreeMap<(u32, u32), Vec<WaveEdgeEvidence>> {
    let mut indexed = std::collections::BTreeMap::<_, Vec<_>>::new();
    for item in evidence {
        if let (Some(from), Some(to)) = (nodes.get(&item.from), nodes.get(&item.to)) {
            indexed.entry((*from, *to)).or_default().push(item);
        }
    }
    indexed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyArtifact {
    pub generation_id: ServingGenerationId,
    pub cognitive_profile: nous_runtime::CognitiveProfile,
    pub nodes: Vec<WaveNode>,
    pub edges: Vec<(u32, u32, f64, f64)>,
    pub evidence: Vec<WaveEdgeEvidence>,
    pub config: WaveConfig,
}

fn supported_relation(kind: &str) -> bool {
    matches!(
        kind,
        "aboutness"
            | "tag_attachment"
            | "schema_support"
            | "derived_from"
            | "temporal_successor"
            | "elaborates"
            | "assoc.related"
            | "assoc.co_occurs"
            | "assoc.sequence"
            | "assoc.procedural"
            | "assoc.shared_outcome"
    )
}
fn class_quality(class: &str, values: &std::collections::BTreeMap<String, f64>) -> f64 {
    values.get(class).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_core::MemoryRevisionId;

    fn node(id: CognitiveRef, serving_id: u32) -> WaveNode {
        WaveNode {
            serving_id,
            reference: id,
            node_kind: WaveNodeKind::Memory,
            embedding_key: None,
            posting_key: None,
            intrinsic_residual_gain: None,
        }
    }

    #[test]
    fn unknown_negative_and_contradictory_edges_do_not_enter_nonnegative_graph() {
        let a = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let b = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let graph = WaveGraphGeneration::build(
            vec![node(a.clone(), 0), node(b.clone(), 1)],
            &[
                WaveEdgeEvidence {
                    from: a.clone(),
                    to: b.clone(),
                    support_class: "host_explicit".into(),
                    association_kind: "contradicts".into(),
                    polarity: "positive".into(),
                    support_mass: 1.0,
                    provenance_root: None,
                },
                WaveEdgeEvidence {
                    from: a.clone(),
                    to: b.clone(),
                    support_class: "host_explicit".into(),
                    association_kind: "assoc.related".into(),
                    polarity: "negative".into(),
                    support_mass: 1.0,
                    provenance_root: None,
                },
            ],
            WaveConfig::default(),
        )
        .expect("graph");
        assert!(graph.outgoing(0).is_empty());
    }

    #[test]
    fn provenance_root_duplicates_keep_only_the_best_support_per_root() {
        let a = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let b = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let graph = WaveGraphGeneration::build(
            vec![node(a.clone(), 0), node(b.clone(), 1)],
            &[
                WaveEdgeEvidence {
                    from: a.clone(),
                    to: b.clone(),
                    support_class: "host_explicit".into(),
                    association_kind: "assoc.related".into(),
                    polarity: "positive".into(),
                    support_mass: 0.6,
                    provenance_root: Some("artifact:one".into()),
                },
                WaveEdgeEvidence {
                    from: a.clone(),
                    to: b.clone(),
                    support_class: "host_explicit".into(),
                    association_kind: "assoc.related".into(),
                    polarity: "positive".into(),
                    support_mass: 0.9,
                    provenance_root: Some("artifact:one".into()),
                },
                WaveEdgeEvidence {
                    from: a,
                    to: b,
                    support_class: "host_explicit".into(),
                    association_kind: "assoc.related".into(),
                    polarity: "positive".into(),
                    support_mass: 0.4,
                    provenance_root: Some("artifact:two".into()),
                },
            ],
            WaveConfig::default(),
        )
        .expect("graph");
        assert_eq!(graph.edge_raw_support(0, 1), Some(1.3));
    }

    #[test]
    fn outbound_mass_is_bounded() {
        let a = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let b = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let c = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let graph = WaveGraphGeneration::build(
            vec![node(a.clone(), 0), node(b.clone(), 1), node(c.clone(), 2)],
            &[
                WaveEdgeEvidence {
                    from: a.clone(),
                    to: b,
                    support_class: "host_explicit".into(),
                    association_kind: "assoc.related".into(),
                    polarity: "positive".into(),
                    support_mass: 1.0,
                    provenance_root: None,
                },
                WaveEdgeEvidence {
                    from: a,
                    to: c,
                    support_class: "host_explicit".into(),
                    association_kind: "assoc.related".into(),
                    polarity: "positive".into(),
                    support_mass: 1.0,
                    provenance_root: None,
                },
            ],
            WaveConfig::default(),
        )
        .expect("graph");
        assert!(graph.outbound_mass(0) <= 0.90 + f64::EPSILON);
    }
}
