// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MemoryService {
    pub async fn ready_status(&self) -> RuntimeStatus {
        self.status().await
    }
}
