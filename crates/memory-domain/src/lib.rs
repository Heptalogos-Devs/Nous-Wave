//! Memory-domain value objects and validation rules.
//!
//! The domain owns cognitive meaning. SQL, transport, retrieval, and provider
//! mechanics remain in their respective owners.

use chrono::{DateTime, Utc};
use nous_core::{
    AssociationEvidenceId, CognitiveRef, CognitiveSchemaId, CognitiveSchemaRevisionId, EntityRef,
    EpistemicClass, Error, MemoryId, MemoryRevisionId, OccurrenceId, OperationId, Result,
    SchemaEvidenceLinkId, SubjectId, TagId, TemporalExtent, UseEventId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveRole {
    Experiential,
    Declarative,
    ProceduralExperience,
}

impl CognitiveRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experiential => "experiential",
            Self::Declarative => "declarative",
            Self::ProceduralExperience => "procedural_experience",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationMode {
    Grounded,
    Synthesized,
}

impl FormationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grounded => "grounded",
            Self::Synthesized => "synthesized",
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityMode {
    #[default]
    Auto,
    Normal,
    Deep,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityLevel {
    Normal,
    Deep,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionIntent {
    Correct,
    Rephrase,
    Reinterpret,
}

impl RevisionIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correct => "correct",
            Self::Rephrase => "rephrase",
            Self::Reinterpret => "reinterpret",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryObject {
    pub memory_id: MemoryId,
    pub subject_id: SubjectId,
    pub cognitive_role: CognitiveRole,
    pub current_revision_id: MemoryRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: AcceptanceState,
    pub integrity_state: IntegrityState,
    pub suppression_state: SuppressionState,
    pub purge_state: PurgeState,
    pub accessibility_mode: AccessibilityMode,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRevision {
    pub memory_revision_id: MemoryRevisionId,
    pub memory_id: MemoryId,
    pub subject_id: SubjectId,
    pub revision_no: i32,
    pub parent_revision_id: Option<MemoryRevisionId>,
    pub revision_intent: Option<RevisionIntent>,
    pub formation_mode: FormationMode,
    pub grounding_occurrence_id: Option<OccurrenceId>,
    pub semantic_role: String,
    pub title: Option<String>,
    pub representation_text: String,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceLocator {
    WholeOccurrence,
    SourceRegion(nous_core::SourceRegionId),
    DerivedRepresentation(nous_core::DerivedRepresentationId),
    DerivedRegion(nous_core::DerivedRegionId),
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

/// The stable source identity used when judging provenance independence.
///
/// An encounter remains an occurrence identity.  This value deliberately
/// collapses repeated encounters and derived representations that originate
/// from the same artifact or external source.
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

/// Compare two recursively resolved support root sets using the R1 rules.
pub fn dependency_relation(left: &[EvidenceRoot], right: &[EvidenceRoot]) -> DependencyRelation {
    let left_keys = left
        .iter()
        .map(|root| root.root_key.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let right_keys = right
        .iter()
        .map(|root| root.root_key.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let intersection = left_keys
        .intersection(&right_keys)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
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
}

impl RevisionSupport {
    pub fn canonical_key(&self) -> String {
        match self {
            Self::Evidence(value) => format!("evidence:{}", value.canonical_key()),
            Self::CognitionDependency(value) => format!(
                "dependency:{}|{}",
                value.target_revision,
                value.support_role.as_str()
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRevisionEvidence {
    pub evidence_no: i32,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRelation {
    DerivedFrom,
    Contradicts,
    TemporalSuccessor,
    ReplacesBasis,
    Elaborates,
}

impl MemoryRelation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DerivedFrom => "derived_from",
            Self::Contradicts => "contradicts",
            Self::TemporalSuccessor => "temporal_successor",
            Self::ReplacesBasis => "replaces_basis",
            Self::Elaborates => "elaborates",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRevisionRelation {
    pub from_revision_id: MemoryRevisionId,
    pub to_revision_id: MemoryRevisionId,
    pub relation: MemoryRelation,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub tag_id: TagId,
    pub subject_id: SubjectId,
    pub current_revision_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRevision {
    pub tag_revision_id: Uuid,
    pub tag_id: TagId,
    pub revision_no: i32,
    pub label: String,
    pub description: Option<String>,
    pub kind_hint: Option<String>,
    pub origin: String,
    pub producer_signature_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssociationPolarity {
    Positive,
    Negative,
}

impl AssociationPolarity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssociationSupportClass {
    HostExplicit,
    SourceEvidence,
    CognitiveDerivation,
    MeaningfulUse,
    DerivedStructure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseEventRef {
    pub subject_id: SubjectId,
    pub consumer_ref: String,
    pub event_id: UseEventId,
}

impl UseEventRef {
    pub fn canonical_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.subject_id.0, self.consumer_ref, self.event_id.0
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssociationSupport {
    Revision(RevisionSupport),
    UseEvent(UseEventRef),
}

impl AssociationSupportClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HostExplicit => "host_explicit",
            Self::SourceEvidence => "source_evidence",
            Self::CognitiveDerivation => "cognitive_derivation",
            Self::MeaningfulUse => "meaningful_use",
            Self::DerivedStructure => "derived_structure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationEvidence {
    pub association_evidence_id: AssociationEvidenceId,
    pub subject_id: SubjectId,
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub relation_kind: String,
    pub polarity: AssociationPolarity,
    pub support_class: AssociationSupportClass,
    pub supports: Vec<AssociationSupport>,
    pub producer_signature_id: Option<Uuid>,
    pub valid_time: TemporalExtent,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitMemoryInput {
    pub operation_id: OperationId,
    #[serde(default)]
    pub subject: SubjectId,
    pub cognitive_role: CognitiveRole,
    pub formation_mode: FormationMode,
    pub grounding_occurrence_id: Option<OccurrenceId>,
    pub semantic_role: String,
    pub representation_text: String,
    pub title: Option<String>,
    #[serde(default)]
    pub supports: Vec<RevisionSupport>,
    #[serde(default)]
    pub aboutness: Vec<EntityRef>,
    #[serde(default)]
    pub tags: Vec<TagId>,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub epistemic_class: EpistemicClass,
}

impl ExplicitMemoryInput {
    pub fn validate(&self) -> Result<()> {
        if self.operation_id.0.is_nil() {
            return Err(Error::Invalid("operation_id is required".into()));
        }
        validate_content(&self.semantic_role, &self.representation_text)?;
        self.valid_time.validate()?;
        if self.supports.is_empty() {
            return Err(Error::Invalid("a Memory revision needs support".into()));
        }
        validate_supports(
            self.formation_mode,
            self.grounding_occurrence_id,
            &self.supports,
        )?;
        let mut keys = std::collections::BTreeSet::new();
        for support in &self.supports {
            if !keys.insert(support.canonical_key()) {
                return Err(Error::Invalid("duplicate revision support".into()));
            }
        }
        let mut aboutness = std::collections::BTreeSet::new();
        for entity in &self.aboutness {
            EntityRef::new(entity.as_str())?;
            aboutness.insert(entity.as_str());
        }
        if aboutness.len() != self.aboutness.len() {
            return Err(Error::Invalid("duplicate aboutness entity".into()));
        }
        Ok(())
    }
}

pub fn validate_content(semantic_role: &str, representation_text: &str) -> Result<()> {
    if semantic_role.trim().is_empty() || semantic_role.len() > 128 {
        return Err(Error::Invalid(
            "semantic_role is required and bounded".into(),
        ));
    }
    if representation_text.trim().is_empty() || representation_text.len() > 1_000_000 {
        return Err(Error::Invalid(
            "representation_text is required and bounded".into(),
        ));
    }
    Ok(())
}

fn validate_supports(
    mode: FormationMode,
    grounding_occurrence: Option<OccurrenceId>,
    supports: &[RevisionSupport],
) -> Result<()> {
    match mode {
        FormationMode::Grounded => {
            let occurrence = grounding_occurrence.ok_or_else(|| {
                Error::Invalid("grounded formation requires grounding_occurrence_id".into())
            })?;
            if !supports.iter().any(|support| {
                matches!(support, RevisionSupport::Evidence(evidence) if evidence.occurrence_id == occurrence)
            }) {
                return Err(Error::Invalid(
                    "grounding occurrence must be in the evidence support set".into(),
                ));
            }
        }
        FormationMode::Synthesized => {
            if grounding_occurrence.is_some() {
                return Err(Error::Invalid(
                    "synthesized formation cannot have grounding_occurrence_id".into(),
                ));
            }
            if supports.len() < 2 {
                return Err(Error::Invalid(
                    "synthesized formation needs at least two distinct inputs".into(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFormationProposal {
    pub operation_id: OperationId,
    pub cognitive_role: CognitiveRole,
    pub formation_mode: FormationMode,
    pub grounding_occurrence_id: Option<OccurrenceId>,
    pub semantic_role: String,
    pub representation_text: String,
    pub title: Option<String>,
    pub supports: Vec<RevisionSupport>,
    pub aboutness: Vec<EntityRef>,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub epistemic_class: EpistemicClass,
}

impl MemoryFormationProposal {
    pub fn into_input(self, subject: SubjectId) -> ExplicitMemoryInput {
        ExplicitMemoryInput {
            operation_id: self.operation_id,
            subject,
            cognitive_role: self.cognitive_role,
            formation_mode: self.formation_mode,
            grounding_occurrence_id: self.grounding_occurrence_id,
            semantic_role: self.semantic_role,
            representation_text: self.representation_text,
            title: self.title,
            supports: self.supports,
            aboutness: self.aboutness,
            tags: Vec::new(),
            valid_time: self.valid_time,
            formed_at: self.formed_at,
            epistemic_class: self.epistemic_class,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TagProposal {
    Existing {
        tag_id: TagId,
    },
    New {
        label: String,
        description: Option<String>,
        kind_hint: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaScope {
    pub description: String,
    #[serde(default)]
    pub aboutness: Vec<EntityRef>,
    #[serde(default)]
    pub tags: Vec<TagId>,
    pub valid_time: TemporalExtent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSchema {
    pub schema_id: CognitiveSchemaId,
    pub subject_id: SubjectId,
    pub current_revision_id: CognitiveSchemaRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: AcceptanceState,
    pub integrity_state: IntegrityState,
    pub suppression_state: SuppressionState,
    pub purge_state: PurgeState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSchemaRevision {
    pub schema_revision_id: CognitiveSchemaRevisionId,
    pub schema_id: CognitiveSchemaId,
    pub revision_no: i32,
    pub parent_revision_id: Option<CognitiveSchemaRevisionId>,
    pub revision_intent: Option<RevisionIntent>,
    pub title: Option<String>,
    pub structural_claim: String,
    pub applicability_scope: SchemaScope,
    pub boundary_definition: String,
    pub formation_kind: SchemaFormationKind,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFormationKind {
    #[default]
    ExplicitImport,
    Synthesized,
}

impl SchemaFormationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitImport => "explicit_import",
            Self::Synthesized => "synthesized",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaEvidenceRole {
    Support,
    Counterexample,
    BoundaryCase,
}

impl SchemaEvidenceRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Support => "support",
            Self::Counterexample => "counterexample",
            Self::BoundaryCase => "boundary_case",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEvidenceLink {
    pub link_id: SchemaEvidenceLinkId,
    pub subject_id: SubjectId,
    pub schema_revision_id: CognitiveSchemaRevisionId,
    pub role: SchemaEvidenceRole,
    pub support: RevisionSupport,
    pub producer_signature_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaLineage {
    pub from_revision_id: CognitiveSchemaRevisionId,
    pub to_revision_id: CognitiveSchemaRevisionId,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSchemaInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub title: Option<String>,
    pub structural_claim: String,
    pub applicability_scope: SchemaScope,
    pub boundary_definition: String,
    pub formed_at: DateTime<Utc>,
    #[serde(default)]
    pub formation_kind: SchemaFormationKind,
    pub evidence_links: Vec<SchemaEvidenceLinkInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEvidenceLinkInput {
    pub role: SchemaEvidenceRole,
    pub support: RevisionSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseSchemaInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub schema_id: CognitiveSchemaId,
    pub expected_object_epoch: i64,
    pub intent: RevisionIntent,
    pub title: Option<String>,
    pub structural_claim: String,
    pub applicability_scope: SchemaScope,
    pub boundary_definition: String,
    pub formed_at: DateTime<Utc>,
    pub copy_link_ids: Vec<SchemaEvidenceLinkId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssociationInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub relation_kind: String,
    pub polarity: AssociationPolarity,
    pub support_class: AssociationSupportClass,
    pub supports: Vec<AssociationSupport>,
    pub valid_time: TemporalExtent,
}

// Consolidation remains an owner-level proposal envelope. It is translated
// into the explicit Memory/Schema operations before Authority commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationTarget {
    Synthesized,
    TopologyOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationRequest {
    pub operation_id: OperationId,
    #[serde(default)]
    pub subject: SubjectId,
    pub source_memories: Vec<MemoryRevisionId>,
    pub target: ConsolidationTarget,
    pub representation_text: Option<String>,
    pub semantic_role: Option<String>,
    pub formed_at: DateTime<Utc>,
    #[serde(default)]
    pub topology: Option<TopologyConsolidationProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologyConsolidationProposal {
    #[serde(default)]
    pub tags: Vec<TopologyTagProposal>,
    #[serde(default)]
    pub associations: Vec<TopologyAssociationProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyTagProposal {
    pub label: String,
    pub description: Option<String>,
    pub kind_hint: Option<String>,
    pub tag_id: Option<TagId>,
    #[serde(default)]
    pub attach_to: Vec<MemoryRevisionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyAssociationProposal {
    pub from: CognitiveRef,
    pub to: CognitiveRef,
    pub relation_kind: String,
    pub polarity: AssociationPolarity,
    pub support_class: AssociationSupportClass,
    #[serde(default)]
    pub supports: Vec<AssociationSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalEvidence {
    pub occurred: Vec<TemporalExtent>,
    pub observed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(occurrence: OccurrenceId) -> RevisionSupport {
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: occurrence,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        })
    }

    #[test]
    fn formation_mode_is_revision_level() {
        let occurrence = OccurrenceId::new();
        let input = ExplicitMemoryInput {
            operation_id: OperationId::new(),
            subject: SubjectId::new(),
            cognitive_role: CognitiveRole::Declarative,
            formation_mode: FormationMode::Grounded,
            grounding_occurrence_id: Some(occurrence),
            semantic_role: "fact".into(),
            representation_text: "bounded fact".into(),
            title: None,
            supports: vec![evidence(occurrence)],
            aboutness: Vec::new(),
            tags: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Reported,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn synthesized_requires_two_inputs() {
        let input = ExplicitMemoryInput {
            operation_id: OperationId::new(),
            subject: SubjectId::new(),
            cognitive_role: CognitiveRole::Declarative,
            formation_mode: FormationMode::Synthesized,
            grounding_occurrence_id: None,
            semantic_role: "fact".into(),
            representation_text: "bounded fact".into(),
            title: None,
            supports: vec![evidence(OccurrenceId::new())],
            aboutness: Vec::new(),
            tags: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Inferred,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn provenance_relation_is_conservative_for_unknown_encounters() {
        let known = EvidenceRoot {
            root_key: "artifact:a".into(),
            certainty: EvidenceRootCertainty::Known,
        };
        let same = known.clone();
        assert_eq!(
            dependency_relation(std::slice::from_ref(&known), std::slice::from_ref(&same)),
            DependencyRelation::SameRoot
        );
        assert_eq!(
            dependency_relation(
                std::slice::from_ref(&known),
                &[EvidenceRoot {
                    root_key: "artifact:b".into(),
                    certainty: EvidenceRootCertainty::Known,
                }]
            ),
            DependencyRelation::Independent
        );
        assert_eq!(
            dependency_relation(
                &[EvidenceRoot {
                    root_key: "occurrence:a".into(),
                    certainty: EvidenceRootCertainty::OccurrenceOnly,
                }],
                &[EvidenceRoot {
                    root_key: "occurrence:b".into(),
                    certainty: EvidenceRootCertainty::OccurrenceOnly,
                }]
            ),
            DependencyRelation::UnknownDependency
        );
    }
}
