// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMetadata {
    pub media_type: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
