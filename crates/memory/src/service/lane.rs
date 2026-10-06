// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nous_core::{EvidenceFamily, Result};
use nous_runtime::{BoundQuery, QueryPlan};

pub(crate) use nous_runtime::{LaneCandidate, LaneOutput, LaneStatus};

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
