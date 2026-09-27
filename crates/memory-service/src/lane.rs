use async_trait::async_trait;
use nous_cognitive_runtime::{BoundQuery, QueryPlan};
use nous_core::{CognitiveRef, EvidenceFamily, Result, ServingGenerationId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LaneStatus {
    Disabled,
    Ready,
    Stale,
    Unavailable,
    Truncated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LaneCandidate {
    pub reference: CognitiveRef,
    pub rank: u32,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub provider_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LaneOutput {
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

#[async_trait]
#[allow(dead_code)]
pub(crate) trait LaneProvider: Send + Sync {
    fn family(&self) -> EvidenceFamily;

    async fn generate(
        &self,
        bound_query: &BoundQuery,
        plan: &QueryPlan,
        prior_outputs: &[LaneOutput],
    ) -> Result<LaneOutput>;
}
