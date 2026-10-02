use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMetadata {
    pub media_type: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
