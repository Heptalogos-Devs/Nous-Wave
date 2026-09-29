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
pub struct LaneOutput {
    pub family: EvidenceFamily,
    pub status: LaneStatus,
    pub generation_ref: Option<ServingGenerationId>,
    pub authority_watermark: Option<i64>,
    pub candidates: Vec<LaneCandidate>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
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
        }
    }
}
