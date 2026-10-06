// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use nous_core::{CognitiveRef, EvidenceFamily, ServingGenerationId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneStatus {
    Disabled,
    Ready,
    Stale,
    Unavailable,
    Truncated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneCandidate {
    pub reference: CognitiveRef,
    pub rank: u32,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub provider_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyWorkSummary {
    pub profile_id: String,
    pub profile_digest: String,
    pub activated_edges: usize,
    pub max_hop_observed: usize,
    pub mechanism: String,
    pub seed_count: usize,
    pub visited_nodes: usize,
    pub complete: bool,
    /// None means the engine does not measure discarded probability mass.
    pub discarded_mass: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneOutput {
    pub family: EvidenceFamily,
    pub status: LaneStatus,
    pub generation_ref: Option<ServingGenerationId>,
    pub authority_watermark: Option<i64>,
    pub candidates: Vec<LaneCandidate>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
    pub topology_work: Option<TopologyWorkSummary>,
}

impl LaneOutput {
    pub fn empty(family: EvidenceFamily, status: LaneStatus) -> Self {
        Self {
            family,
            status,
            generation_ref: None,
            authority_watermark: None,
            candidates: Vec::new(),
            diagnostics: Vec::new(),
            topology_work: None,
        }
    }
}
