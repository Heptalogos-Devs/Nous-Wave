//! Supported association relation identity and adjacency interpretation.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyRelation {
    TagAttachment,
    #[serde(rename = "assoc.related")]
    Related,
    #[serde(rename = "assoc.co_occurs")]
    CoOccurs,
    #[serde(rename = "assoc.sequence")]
    Sequence,
    #[serde(rename = "assoc.procedural")]
    Procedural,
    #[serde(rename = "assoc.shared_outcome")]
    SharedOutcome,
}
impl TopologyRelation {
    pub fn is_symmetric(&self) -> bool {
        matches!(
            self,
            Self::TagAttachment | Self::Related | Self::CoOccurs | Self::SharedOutcome
        )
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TagAttachment => "tag_attachment",
            Self::Related => "assoc.related",
            Self::CoOccurs => "assoc.co_occurs",
            Self::Sequence => "assoc.sequence",
            Self::Procedural => "assoc.procedural",
            Self::SharedOutcome => "assoc.shared_outcome",
        }
    }
}
