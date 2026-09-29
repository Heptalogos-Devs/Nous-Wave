use crate::*;
use nous_configuration::{
    ConfigApplyMode, ConfigExposure, ConfigKey, ConfigRegistryBuilder, ConfigScopePolicy,
    ConfigSemanticEffect, ConfigSnapshot,
};
use nous_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;

/// The only production owner of the reference RRF constants and formula.
pub const RRF_K: f64 = 60.0;

pub const RRF_K_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.k");
pub const RRF_EXACT_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.exact");
pub const RRF_RUNTIME_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.runtime");
pub const RRF_ENTITY_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.entity");
pub const RRF_LEXICAL_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.lexical");
pub const RRF_DENSE_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.dense");
pub const RRF_TEMPORAL_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.temporal");
pub const RRF_SCHEMA_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.schema_direct");
pub const RRF_TOPOLOGY_KEY: ConfigKey<f64> = ConfigKey::new("retrieval.rrf.weights.topology_wave");

pub const QUERY_LIGHT_MULTIPLIER: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.effort_multiplier.light");
pub const QUERY_NORMAL_MULTIPLIER: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.effort_multiplier.normal");
pub const QUERY_DEEP_MULTIPLIER: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.effort_multiplier.deep");
pub const QUERY_MAXIMUM_MULTIPLIER: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.effort_multiplier.maximum");
pub const QUERY_LIGHT_MAX: ConfigKey<usize> = ConfigKey::new("retrieval.query.per_lane_max.light");
pub const QUERY_NORMAL_MAX: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.per_lane_max.normal");
pub const QUERY_DEEP_MAX: ConfigKey<usize> = ConfigKey::new("retrieval.query.per_lane_max.deep");
pub const QUERY_MAXIMUM_MAX: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.per_lane_max.maximum");
pub const QUERY_VALIDATION_MULTIPLIER: ConfigKey<usize> =
    ConfigKey::new("retrieval.query.validation_multiplier");
pub const QUERY_VALIDATION_MIN: ConfigKey<usize> = ConfigKey::new("retrieval.query.validation_min");
pub const QUERY_VALIDATION_MAX: ConfigKey<usize> = ConfigKey::new("retrieval.query.validation_max");

#[derive(Debug, Clone)]
pub struct RetrievalPolicy {
    pub rrf_k: f64,
    pub weights: BTreeMap<EvidenceFamily, f64>,
    pub effort_multipliers: [usize; 4],
    pub per_lane_max: [usize; 4],
    pub validation_multiplier: usize,
    pub validation_min: usize,
    pub validation_max: usize,
}

impl RetrievalPolicy {
    pub fn reference() -> Self {
        Self {
            rrf_k: RRF_K,
            weights: BTreeMap::from([
                (EvidenceFamily::Exact, 4.0),
                (EvidenceFamily::Runtime, 2.0),
                (EvidenceFamily::Entity, 2.5),
                (EvidenceFamily::Lexical, 1.5),
                (EvidenceFamily::Dense, 1.5),
                (EvidenceFamily::Temporal, 1.0),
                (EvidenceFamily::SchemaDirect, 1.5),
                (EvidenceFamily::TopologyWave, 1.0),
            ]),
            effort_multipliers: [2, 4, 8, 16],
            per_lane_max: [128, 512, 2048, 8192],
            validation_multiplier: 4,
            validation_min: 32,
            validation_max: 4096,
        }
    }

    pub fn weight(&self, family: EvidenceFamily) -> Option<f64> {
        self.weights.get(&family).copied()
    }

    pub fn validate(&self) -> Result<()> {
        if !self.rrf_k.is_finite()
            || self.rrf_k <= 0.0
            || self
                .weights
                .values()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || self.effort_multipliers.contains(&0)
            || self.per_lane_max.contains(&0)
            || self.validation_multiplier == 0
            || self.validation_min == 0
            || self.validation_max < self.validation_min
        {
            return Err(Error::Invalid("invalid retrieval policy bounds".into()));
        }
        Ok(())
    }
}

pub fn register_retrieval_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    let positive_float = |value: &f64| {
        if value.is_finite() && *value > 0.0 {
            Ok(())
        } else {
            Err(Error::Invalid(
                "retrieval value must be finite and positive".into(),
            ))
        }
    };
    let positive_usize = |value: &usize| {
        if *value > 0 {
            Ok(())
        } else {
            Err(Error::Invalid("retrieval budget must be positive".into()))
        }
    };
    macro_rules! float {
        ($key:expr, $default:expr, $description:expr) => {
            registry.register(
                $key,
                "runtime",
                $description,
                $default,
                ConfigExposure::Developer,
                ConfigScopePolicy::SystemOnly,
                ConfigApplyMode::Live,
                ConfigSemanticEffect::QueryPolicy,
                positive_float,
            )?;
        };
    }
    macro_rules! budget {
        ($key:expr, $default:expr, $description:expr) => {
            registry.register(
                $key,
                "runtime",
                $description,
                $default,
                ConfigExposure::Developer,
                ConfigScopePolicy::SystemOnly,
                ConfigApplyMode::Live,
                ConfigSemanticEffect::QueryPolicy,
                positive_usize,
            )?;
        };
    }
    float!(RRF_K_KEY, 60.0, "RRF denominator constant.");
    float!(RRF_EXACT_KEY, 4.0, "Exact lane RRF weight.");
    float!(RRF_RUNTIME_KEY, 2.0, "Runtime lane RRF weight.");
    float!(RRF_ENTITY_KEY, 2.5, "Entity lane RRF weight.");
    float!(RRF_LEXICAL_KEY, 1.5, "Lexical lane RRF weight.");
    float!(RRF_DENSE_KEY, 1.5, "Dense lane RRF weight.");
    float!(RRF_TEMPORAL_KEY, 1.0, "Temporal lane RRF weight.");
    float!(RRF_SCHEMA_KEY, 1.5, "Schema lane RRF weight.");
    float!(RRF_TOPOLOGY_KEY, 1.0, "Topology lane RRF weight.");
    budget!(QUERY_LIGHT_MULTIPLIER, 2, "Light query effort multiplier.");
    budget!(
        QUERY_NORMAL_MULTIPLIER,
        4,
        "Normal query effort multiplier."
    );
    budget!(QUERY_DEEP_MULTIPLIER, 8, "Deep query effort multiplier.");
    budget!(
        QUERY_MAXIMUM_MULTIPLIER,
        16,
        "Maximum query effort multiplier."
    );
    budget!(QUERY_LIGHT_MAX, 128, "Light per-lane candidate maximum.");
    budget!(QUERY_NORMAL_MAX, 512, "Normal per-lane candidate maximum.");
    budget!(QUERY_DEEP_MAX, 2048, "Deep per-lane candidate maximum.");
    budget!(
        QUERY_MAXIMUM_MAX,
        8192,
        "Maximum per-lane candidate maximum."
    );
    budget!(
        QUERY_VALIDATION_MULTIPLIER,
        4,
        "Final validation budget multiplier."
    );
    budget!(QUERY_VALIDATION_MIN, 32, "Final validation budget minimum.");
    budget!(
        QUERY_VALIDATION_MAX,
        4096,
        "Final validation budget maximum."
    );
    Ok(())
}

pub fn resolve_retrieval_policy(snapshot: &ConfigSnapshot) -> Result<RetrievalPolicy> {
    let policy = RetrievalPolicy {
        rrf_k: snapshot.get(RRF_K_KEY)?,
        weights: BTreeMap::from([
            (EvidenceFamily::Exact, snapshot.get(RRF_EXACT_KEY)?),
            (EvidenceFamily::Runtime, snapshot.get(RRF_RUNTIME_KEY)?),
            (EvidenceFamily::Entity, snapshot.get(RRF_ENTITY_KEY)?),
            (EvidenceFamily::Lexical, snapshot.get(RRF_LEXICAL_KEY)?),
            (EvidenceFamily::Dense, snapshot.get(RRF_DENSE_KEY)?),
            (EvidenceFamily::Temporal, snapshot.get(RRF_TEMPORAL_KEY)?),
            (EvidenceFamily::SchemaDirect, snapshot.get(RRF_SCHEMA_KEY)?),
            (
                EvidenceFamily::TopologyWave,
                snapshot.get(RRF_TOPOLOGY_KEY)?,
            ),
        ]),
        effort_multipliers: [
            snapshot.get(QUERY_LIGHT_MULTIPLIER)?,
            snapshot.get(QUERY_NORMAL_MULTIPLIER)?,
            snapshot.get(QUERY_DEEP_MULTIPLIER)?,
            snapshot.get(QUERY_MAXIMUM_MULTIPLIER)?,
        ],
        per_lane_max: [
            snapshot.get(QUERY_LIGHT_MAX)?,
            snapshot.get(QUERY_NORMAL_MAX)?,
            snapshot.get(QUERY_DEEP_MAX)?,
            snapshot.get(QUERY_MAXIMUM_MAX)?,
        ],
        validation_multiplier: snapshot.get(QUERY_VALIDATION_MULTIPLIER)?,
        validation_min: snapshot.get(QUERY_VALIDATION_MIN)?,
        validation_max: snapshot.get(QUERY_VALIDATION_MAX)?,
    };
    policy.validate()?;
    Ok(policy)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FusionLaneSpec {
    pub family: EvidenceFamily,
    pub weight: f64,
}

pub fn default_rrf_plan(enabled: &[EvidenceFamily]) -> Vec<FusionLaneSpec> {
    default_rrf_plan_with_policy(enabled, &RetrievalPolicy::reference())
}

pub fn default_rrf_plan_with_policy(
    enabled: &[EvidenceFamily],
    policy: &RetrievalPolicy,
) -> Vec<FusionLaneSpec> {
    enabled
        .iter()
        .filter_map(|family| {
            let weight = policy.weight(*family)?;
            Some(FusionLaneSpec {
                family: *family,
                weight,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRankInput {
    pub reference: CognitiveRef,
    pub family_ranks: HashMap<EvidenceFamily, usize>,
    #[serde(default)]
    pub family_view_ranks: HashMap<EvidenceFamily, Vec<usize>>,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedCandidate {
    pub reference: CognitiveRef,
    pub baseline_score: f64,
    pub final_score: f64,
    pub best_lane_rank: usize,
    pub exact_match: bool,
    pub families: Vec<EvidenceFamily>,
    pub variants: Vec<String>,
}

pub fn rank_candidates(
    candidates: &[CandidateRankInput],
    enabled_lanes: &[EvidenceFamily],
) -> Vec<RankedCandidate> {
    rank_candidates_with_policy(candidates, enabled_lanes, &RetrievalPolicy::reference())
}

pub fn rank_candidates_with_policy(
    candidates: &[CandidateRankInput],
    enabled_lanes: &[EvidenceFamily],
    policy: &RetrievalPolicy,
) -> Vec<RankedCandidate> {
    let enabled = default_rrf_plan_with_policy(enabled_lanes, policy);
    let denominator = enabled
        .iter()
        .map(|lane| lane.weight / (policy.rrf_k + 1.0))
        .sum::<f64>();
    let mut ranked = candidates
        .iter()
        .map(|candidate| {
            let raw = enabled
                .iter()
                .filter_map(|lane| {
                    let rank = candidate
                        .family_view_ranks
                        .get(&lane.family)
                        .and_then(|ranks| ranks.iter().copied().min())
                        .or_else(|| candidate.family_ranks.get(&lane.family).copied())?;
                    Some(lane.weight / (policy.rrf_k + rank as f64))
                })
                .sum::<f64>();
            let mut families = candidate.family_ranks.keys().copied().collect::<Vec<_>>();
            families.sort();
            let best_lane_rank = candidate
                .family_ranks
                .values()
                .copied()
                .min()
                .unwrap_or(usize::MAX);
            let score = if denominator > 0.0 {
                raw / denominator
            } else {
                0.0
            };
            RankedCandidate {
                reference: candidate.reference.clone(),
                baseline_score: score,
                final_score: score,
                best_lane_rank,
                exact_match: candidate.family_ranks.contains_key(&EvidenceFamily::Exact),
                families,
                variants: candidate.variants.clone(),
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .final_score
            .total_cmp(&left.final_score)
            .then_with(|| left.best_lane_rank.cmp(&right.best_lane_rank))
            .then_with(|| right.exact_match.cmp(&left.exact_match))
            .then_with(|| left.reference.to_string().cmp(&right.reference.to_string()))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_core::MemoryId;

    #[test]
    fn denominator_is_plan_family_based() {
        let candidate = CandidateRankInput {
            reference: CognitiveRef::Memory(MemoryId::new()),
            family_ranks: HashMap::from([(EvidenceFamily::Lexical, 1)]),
            family_view_ranks: HashMap::new(),
            variants: Vec::new(),
        };
        let reference = candidate.reference.clone();
        let mut extra = candidate.clone();
        extra.reference = CognitiveRef::Memory(MemoryId::new());
        extra.family_ranks.insert(EvidenceFamily::Dense, 1);
        let first = rank_candidates(std::slice::from_ref(&candidate), &[EvidenceFamily::Lexical])
            [0]
        .baseline_score;
        let second = rank_candidates(&[candidate, extra], &[EvidenceFamily::Lexical])
            .into_iter()
            .find(|value| value.reference == reference)
            .unwrap()
            .baseline_score;
        assert_eq!(first, second);
    }

    #[test]
    fn multi_view_family_uses_one_best_rank_and_tie_order_is_stable() {
        let left = CandidateRankInput {
            reference: CognitiveRef::Memory(MemoryId::new()),
            family_ranks: HashMap::new(),
            family_view_ranks: HashMap::from([(EvidenceFamily::Dense, vec![7, 2, 5])]),
            variants: vec!["dense:space-a".into()],
        };
        let right = CandidateRankInput {
            reference: CognitiveRef::Memory(MemoryId::new()),
            family_ranks: HashMap::from([(EvidenceFamily::Dense, 2)]),
            family_view_ranks: HashMap::new(),
            variants: Vec::new(),
        };
        let reverse = rank_candidates(&[right.clone(), left.clone()], &[EvidenceFamily::Dense]);
        let forward = rank_candidates(&[left, right], &[EvidenceFamily::Dense]);
        let reverse_refs = reverse
            .iter()
            .map(|candidate| candidate.reference.to_string())
            .collect::<Vec<_>>();
        let forward_refs = forward
            .iter()
            .map(|candidate| candidate.reference.to_string())
            .collect::<Vec<_>>();
        assert_eq!(reverse_refs, forward_refs);
    }
}
