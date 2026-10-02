//! Cognition-owner shared lifecycle, support, and provenance primitives.
//!
//! Domain-specific cognition types remain in their owning crates. These types
//! are shared because every cognition owner must apply the same lifecycle and
//! exact-support semantics without depending on another domain.

use crate::{
    CognitiveRef, CognitiveSeedVersionId, DerivedRegionId, DerivedRepresentationId, Error,
    OccurrenceId, Result, SourceRegionId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedSupportRef {
    pub seed_version_id: CognitiveSeedVersionId,
    pub semantic_path: String,
}

impl SeedSupportRef {
    pub fn new(
        seed_version_id: CognitiveSeedVersionId,
        semantic_path: impl Into<String>,
    ) -> Result<Self> {
        let semantic_path = semantic_path.into();
        if semantic_path.is_empty()
            || semantic_path.len() > 512
            || semantic_path.chars().any(char::is_whitespace)
        {
            return Err(Error::Invalid(
                "invalid Cognitive Seed semantic path".into(),
            ));
        }
        Ok(Self {
            seed_version_id,
            semantic_path,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceState {
    Accepted,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityState {
    Valid,
    RevalidationRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionState {
    Normal,
    Suppressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurgeState {
    Normal,
    Purging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceLocator {
    WholeOccurrence,
    SourceRegion(SourceRegionId),
    DerivedRepresentation(DerivedRepresentationId),
    DerivedRegion(DerivedRegionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportRole {
    Direct,
    Corroborating,
    Interpretation,
    Contradiction,
    Contextual,
}

impl SupportRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Corroborating => "corroborating",
            Self::Interpretation => "interpretation",
            Self::Contradiction => "contradiction",
            Self::Contextual => "contextual",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub occurrence_id: OccurrenceId,
    pub locator: EvidenceLocator,
    pub support_role: SupportRole,
}

impl EvidenceRef {
    pub fn cognitive_ref(&self) -> CognitiveRef {
        match self.locator {
            EvidenceLocator::WholeOccurrence => CognitiveRef::Occurrence(self.occurrence_id),
            EvidenceLocator::SourceRegion(id) => CognitiveRef::SourceRegion(id),
            EvidenceLocator::DerivedRepresentation(id) => CognitiveRef::DerivedRepresentation(id),
            EvidenceLocator::DerivedRegion(id) => CognitiveRef::DerivedRegion(id),
        }
    }

    pub fn canonical_key(&self) -> String {
        let locator = match self.locator {
            EvidenceLocator::WholeOccurrence => "whole_occurrence".to_owned(),
            EvidenceLocator::SourceRegion(id) => format!("source_region:{id:?}"),
            EvidenceLocator::DerivedRepresentation(id) => format!("derived_representation:{id:?}"),
            EvidenceLocator::DerivedRegion(id) => format!("derived_region:{id:?}"),
        };
        format!(
            "{}|{}|{}",
            self.occurrence_id.0,
            locator,
            self.support_role.as_str()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRootCertainty {
    Known,
    OccurrenceOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvidenceRoot {
    pub root_key: String,
    pub certainty: EvidenceRootCertainty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyRelation {
    SameRoot,
    PartiallyShared,
    Independent,
    UnknownDependency,
}

pub fn dependency_relation(left: &[EvidenceRoot], right: &[EvidenceRoot]) -> DependencyRelation {
    let left_keys = left
        .iter()
        .map(|root| root.root_key.as_str())
        .collect::<BTreeSet<_>>();
    let right_keys = right
        .iter()
        .map(|root| root.root_key.as_str())
        .collect::<BTreeSet<_>>();
    let intersection = left_keys
        .intersection(&right_keys)
        .copied()
        .collect::<BTreeSet<_>>();
    if !intersection.is_empty()
        && intersection.len() == left_keys.len()
        && intersection.len() == right_keys.len()
    {
        return DependencyRelation::SameRoot;
    }
    if !intersection.is_empty() {
        return DependencyRelation::PartiallyShared;
    }
    let all_known = left
        .iter()
        .chain(right.iter())
        .all(|root| matches!(root.certainty, EvidenceRootCertainty::Known));
    if all_known && !left_keys.is_empty() && !right_keys.is_empty() {
        DependencyRelation::Independent
    } else {
        DependencyRelation::UnknownDependency
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitionDependency {
    pub target_revision: CognitiveRef,
    pub support_role: SupportRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RevisionSupport {
    Evidence(EvidenceRef),
    CognitionDependency(CognitionDependency),
    Seed(SeedSupportRef),
}

impl RevisionSupport {
    pub fn canonical_key(&self) -> String {
        match self {
            Self::Evidence(value) => format!("evidence:{}", value.canonical_key()),
            Self::CognitionDependency(value) => {
                format!(
                    "dependency:{}|{}",
                    value.target_revision,
                    value.support_role.as_str()
                )
            }
            Self::Seed(value) => {
                format!("seed:{}:{}", value.seed_version_id.0, value.semantic_path)
            }
        }
    }
}

pub fn validate_exact_supports(supports: &[RevisionSupport]) -> Result<()> {
    if supports.is_empty() {
        return Err(Error::Invalid("a cognition revision needs support".into()));
    }
    let mut keys = BTreeSet::new();
    for support in supports {
        if !keys.insert(support.canonical_key()) {
            return Err(Error::Invalid("duplicate revision support".into()));
        }
        if let RevisionSupport::CognitionDependency(value) = support
            && !matches!(
                value.target_revision,
                CognitiveRef::MemoryRevision(_)
                    | CognitiveRef::CognitiveSchemaRevision(_)
                    | CognitiveRef::EpisodeRevision(_)
                    | CognitiveRef::JournalRevision(_)
            )
        {
            return Err(Error::Invalid(
                "cognition support must target an exact revision".into(),
            ));
        }
    }
    Ok(())
}
