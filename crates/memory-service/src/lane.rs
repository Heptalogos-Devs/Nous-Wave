use async_trait::async_trait;
use nous_cognitive_runtime::{BoundQuery, QueryPlan};
use nous_core::{EvidenceFamily, Result};

pub(crate) use nous_cognitive_runtime::{LaneCandidate, LaneOutput, LaneStatus};

#[async_trait]
#[expect(
    dead_code,
    reason = "LaneProvider is the reserved owner seam for bounded lane implementations"
)]
pub(crate) trait LaneProvider: Send + Sync {
    fn family(&self) -> EvidenceFamily;

    async fn generate(
        &self,
        bound_query: &BoundQuery,
        plan: &QueryPlan,
        prior_outputs: &[LaneOutput],
    ) -> Result<LaneOutput>;
}
