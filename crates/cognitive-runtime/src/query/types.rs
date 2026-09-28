use nous_core::*;
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

/// The immutable query contract consumed by candidate providers.
///
/// Binding happens before any serving or Authority candidate is inspected.  A
/// provider may therefore report only lane-local relevance and cannot change
/// the enabled lane set or its budgets while it is running.
#[derive(Debug, Clone)]
pub struct BoundQuery {
    pub query_id: Uuid,
    pub source_query: CognitiveQuery,
    pub bound_at_authority_seq: i64,
    pub revision_policy: RevisionPolicy,
    pub exact_bindings: Vec<ExactBinding>,
    pub runtime_refs: Vec<CognitiveRef>,
    pub topology_seed_refs: Vec<(CognitiveRef, String)>,
    pub enabled_lanes: Vec<EvidenceFamily>,
    pub lane_budgets: BTreeMap<EvidenceFamily, usize>,
    pub hard_constraints: QueryConstraints,
    pub accessibility_policy: AccessibilityQueryPolicy,
    pub selected_embedding_space: Option<EmbeddingSpaceSignature>,
    pub topology_required: bool,
    pub fusion_version: String,
    pub rerank_policy: RerankPolicy,
    pub config_snapshot: nous_configuration_service::ConfigSnapshot,
    pub retrieval_policy: nous_cognitive_retrieval::RetrievalPolicy,
}

#[derive(Debug, Clone)]
pub enum RevisionPolicy {
    CurrentOnly,
    ExactHistorical {
        allowed_revision_refs: HashSet<CognitiveRef>,
    },
}

impl RevisionPolicy {
    pub fn allows_historical(&self, reference: &CognitiveRef) -> bool {
        matches!(
            self,
            Self::ExactHistorical {
                allowed_revision_refs
            } if allowed_revision_refs.contains(reference)
        )
    }
}

#[derive(Debug, Clone)]
pub struct ExactBinding {
    pub requested_ref: CognitiveRef,
    pub bound_ref: CognitiveRef,
    pub bound_object_epoch: Option<i64>,
    pub mutable_object: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct AccessibilityQueryPolicy {
    pub effort: CognitiveEffort,
    pub exact_target_bypasses_auto_level: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RerankPolicy {
    pub strength: RequirementStrength,
    pub enabled: bool,
    pub top_n: usize,
}

impl BoundQuery {
    pub fn query(&self) -> &CognitiveQuery {
        &self.source_query
    }

    pub fn lane_enabled(&self, family: EvidenceFamily) -> bool {
        self.enabled_lanes.contains(&family)
    }

    pub fn lane_budget(&self, family: EvidenceFamily) -> usize {
        self.lane_budgets.get(&family).copied().unwrap_or(0)
    }

    pub fn exact_binding_for(&self, reference: &CognitiveRef) -> Option<&ExactBinding> {
        self.exact_bindings
            .iter()
            .find(|binding| &binding.requested_ref == reference)
    }
}
