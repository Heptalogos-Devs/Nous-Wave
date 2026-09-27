use crate::*;
use std::collections::HashMap;

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
    pub families: Vec<EvidenceFamily>,
    pub variants: Vec<String>,
}

const FAMILY_WEIGHTS: &[(EvidenceFamily, f64)] = &[
    (EvidenceFamily::Exact, 4.0),
    (EvidenceFamily::Runtime, 2.0),
    (EvidenceFamily::Entity, 2.5),
    (EvidenceFamily::Lexical, 1.5),
    (EvidenceFamily::Dense, 1.5),
    (EvidenceFamily::Temporal, 1.0),
    (EvidenceFamily::TopologyWave, 1.0),
];

pub fn rank_candidates(
    candidates: &[CandidateRankInput],
    _diagnostic_topology_value: f64,
) -> Vec<RankedCandidate> {
    let enabled = FAMILY_WEIGHTS.to_vec();
    let denominator = enabled
        .iter()
        .map(|(_, weight)| weight / 61.0)
        .sum::<f64>()
        .max(f64::EPSILON);
    let mut ranked = candidates
        .iter()
        .map(|candidate| {
            let raw = enabled
                .iter()
                .filter_map(|(family, weight)| {
                    candidate
                        .family_view_ranks
                        .get(family)
                        .and_then(|ranks| ranks.iter().copied().min())
                        .or_else(|| candidate.family_ranks.get(family).copied())
                        .as_ref()
                        .map(|rank| weight / (60.0 + *rank as f64))
                })
                .sum::<f64>();
            let mut families = candidate.family_ranks.keys().copied().collect::<Vec<_>>();
            families.sort();
            let score = raw / denominator;
            RankedCandidate {
                reference: candidate.reference.clone(),
                baseline_score: score,
                final_score: score,
                families,
                variants: candidate.variants.clone(),
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .final_score
            .total_cmp(&left.final_score)
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
        let first = rank_candidates(std::slice::from_ref(&candidate), 0.0)[0].baseline_score;
        let second = rank_candidates(&[candidate, extra], 0.0)
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
        let reverse = rank_candidates(&[right.clone(), left.clone()], 0.0);
        let forward = rank_candidates(&[left, right], 0.0);
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
