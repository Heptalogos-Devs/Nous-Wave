//! Self cognition value objects and state-transition validation.
//!
//! Persistence and query mechanics belong to `nous-self-service`; this crate
//! deliberately has no dependency on Memory or the Authority Store.

use chrono::{DateTime, Utc};
use nous_core::{
    AcceptanceState, CognitiveRef, EpistemicClass, Error, EvidenceRef, IntegrityState, OperationId,
    PurgeState, Result, RevisionSupport, SubjectId, SuppressionState, TemporalExtent,
    validate_exact_supports,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_KEY_BYTES: usize = 256;
const MAX_SCOPE_BYTES: usize = 256;
const MAX_STATEMENT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfFacetKind {
    Identity,
    Role,
    Capability,
    Limitation,
    Tendency,
    Value,
    Preference,
}

impl SelfFacetKind {
    pub const ORDER: [Self; 7] = [
        Self::Identity,
        Self::Role,
        Self::Limitation,
        Self::Capability,
        Self::Value,
        Self::Preference,
        Self::Tendency,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Role => "role",
            Self::Capability => "capability",
            Self::Limitation => "limitation",
            Self::Tendency => "tendency",
            Self::Value => "value",
            Self::Preference => "preference",
        }
    }
}

impl TryFrom<&str> for SelfFacetKind {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "identity" => Ok(Self::Identity),
            "role" => Ok(Self::Role),
            "capability" => Ok(Self::Capability),
            "limitation" => Ok(Self::Limitation),
            "tendency" => Ok(Self::Tendency),
            "value" => Ok(Self::Value),
            "preference" => Ok(Self::Preference),
            _ => Err(Error::Invalid(format!("unknown Self facet kind: {value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfRevisionIntent {
    Correct,
    Refine,
    Reinterpret,
    Evolve,
}

impl SelfRevisionIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correct => "correct",
            Self::Refine => "refine",
            Self::Reinterpret => "reinterpret",
            Self::Evolve => "evolve",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfFacet {
    pub self_facet_id: nous_core::SelfFacetId,
    pub subject_id: SubjectId,
    pub kind: SelfFacetKind,
    pub key: String,
    pub current_revision_id: nous_core::SelfFacetRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: AcceptanceState,
    pub integrity_state: IntegrityState,
    pub suppression_state: SuppressionState,
    pub purge_state: PurgeState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfFacetRevision {
    pub self_facet_revision_id: nous_core::SelfFacetRevisionId,
    pub self_facet_id: nous_core::SelfFacetId,
    pub subject_id: SubjectId,
    pub revision_no: i32,
    pub parent_revision_id: Option<nous_core::SelfFacetRevisionId>,
    pub revision_intent: Option<SelfRevisionIntent>,
    pub statement: String,
    pub scope: String,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub producer_signature_id: Option<Uuid>,
    pub supports: Vec<RevisionSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeIdentity {
    pub narrative_identity_id: nous_core::NarrativeIdentityId,
    pub subject_id: SubjectId,
    pub key: String,
    pub current_revision_id: nous_core::NarrativeIdentityRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: AcceptanceState,
    pub integrity_state: IntegrityState,
    pub suppression_state: SuppressionState,
    pub purge_state: PurgeState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeIdentityRevision {
    pub narrative_identity_revision_id: nous_core::NarrativeIdentityRevisionId,
    pub narrative_identity_id: nous_core::NarrativeIdentityId,
    pub subject_id: SubjectId,
    pub revision_no: i32,
    pub parent_revision_id: Option<nous_core::NarrativeIdentityRevisionId>,
    pub revision_intent: Option<SelfRevisionIntent>,
    pub text: String,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub producer_signature_id: Option<Uuid>,
    pub supports: Vec<RevisionSupport>,
    pub references: Vec<NarrativeReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeReference {
    pub narrative_revision_id: nous_core::NarrativeIdentityRevisionId,
    pub position: i32,
    pub target_exact_ref: CognitiveRef,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSelfFacet {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub kind: SelfFacetKind,
    pub key: String,
    pub statement: String,
    pub scope: String,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseSelfFacet {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub self_facet_id: nous_core::SelfFacetId,
    pub expected_object_epoch: i64,
    pub parent_revision_id: nous_core::SelfFacetRevisionId,
    pub revision_intent: SelfRevisionIntent,
    pub statement: String,
    pub scope: String,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNarrativeIdentity {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub key: String,
    pub text: String,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub references: Vec<NarrativeReferenceInput>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeReferenceInput {
    pub target_exact_ref: CognitiveRef,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseNarrativeIdentity {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub narrative_identity_id: nous_core::NarrativeIdentityId,
    pub expected_object_epoch: i64,
    pub parent_revision_id: nous_core::NarrativeIdentityRevisionId,
    pub revision_intent: SelfRevisionIntent,
    pub text: String,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub references: Vec<NarrativeReferenceInput>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfLifecycleOperation {
    Withdraw,
    Reaccept,
    Suppress,
    Restore,
    MarkRevalidationRequired,
    RestoreValid,
    Purge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateSelfLifecycle {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub reference: CognitiveRef,
    pub expected_object_epoch: i64,
    pub operation: SelfLifecycleOperation,
}

pub fn validate_key(key: &str) -> Result<()> {
    if key.trim().is_empty() || key.len() > MAX_KEY_BYTES || key.chars().any(char::is_whitespace) {
        return Err(Error::Invalid(
            "Self key must be non-empty, bounded, and whitespace-free".into(),
        ));
    }
    Ok(())
}

pub fn validate_scope(scope: &str) -> Result<()> {
    if scope.trim().is_empty() || scope.len() > MAX_SCOPE_BYTES {
        return Err(Error::Invalid(
            "Self scope must be non-empty and bounded".into(),
        ));
    }
    Ok(())
}

pub fn validate_statement(statement: &str) -> Result<()> {
    if statement.trim().is_empty() || statement.len() > MAX_STATEMENT_BYTES {
        return Err(Error::Invalid(
            "Self statement must be non-empty and at most 64 KiB".into(),
        ));
    }
    Ok(())
}

fn producer_required(epistemic_class: EpistemicClass) -> bool {
    matches!(
        epistemic_class,
        EpistemicClass::Derived | EpistemicClass::Inferred | EpistemicClass::Simulated
    )
}

pub fn validate_facet_create(input: &CreateSelfFacet) -> Result<()> {
    if input.operation_id.0.is_nil() {
        return Err(Error::Invalid("operation_id is required".into()));
    }
    validate_key(&input.key)?;
    validate_scope(&input.scope)?;
    validate_statement(&input.statement)?;
    input.valid_time.validate()?;
    validate_exact_supports(&input.supports)?;
    if producer_required(input.epistemic_class) && input.producer_signature_id.is_none() {
        return Err(Error::Invalid(
            "derived Self cognition requires producer_signature_id".into(),
        ));
    }
    Ok(())
}

pub fn validate_facet_revision(input: &ReviseSelfFacet) -> Result<()> {
    if input.operation_id.0.is_nil() {
        return Err(Error::Invalid("operation_id is required".into()));
    }
    if input.expected_object_epoch <= 0 {
        return Err(Error::Invalid(
            "expected_object_epoch must be positive".into(),
        ));
    }
    validate_scope(&input.scope)?;
    validate_statement(&input.statement)?;
    input.valid_time.validate()?;
    validate_exact_supports(&input.supports)?;
    if producer_required(input.epistemic_class) && input.producer_signature_id.is_none() {
        return Err(Error::Invalid(
            "derived Self cognition requires producer_signature_id".into(),
        ));
    }
    Ok(())
}

pub fn validate_narrative_references(references: &[NarrativeReferenceInput]) -> Result<()> {
    for (position, reference) in references.iter().enumerate() {
        if !matches!(
            reference.target_exact_ref,
            CognitiveRef::MemoryRevision(_) | CognitiveRef::SelfFacetRevision(_)
        ) {
            return Err(Error::Invalid(
                "NarrativeReference must target an exact Memory or Self revision".into(),
            ));
        }
        if reference.role.trim().is_empty() || reference.role.len() > MAX_KEY_BYTES {
            return Err(Error::Invalid("NarrativeReference role is invalid".into()));
        }
        validate_narrative_position(position)?;
    }
    Ok(())
}

fn validate_narrative_position(position: usize) -> Result<()> {
    if position > i32::MAX as usize {
        return Err(Error::Invalid("too many NarrativeReference entries".into()));
    }
    Ok(())
}

pub fn validate_narrative_create(input: &CreateNarrativeIdentity) -> Result<()> {
    if input.operation_id.0.is_nil() {
        return Err(Error::Invalid("operation_id is required".into()));
    }
    validate_key(&input.key)?;
    validate_statement(&input.text)?;
    input.valid_time.validate()?;
    validate_exact_supports(&input.supports)?;
    validate_narrative_references(&input.references)?;
    Ok(())
}

pub fn validate_narrative_revision(input: &ReviseNarrativeIdentity) -> Result<()> {
    if input.operation_id.0.is_nil() || input.expected_object_epoch <= 0 {
        return Err(Error::Invalid(
            "operation_id and positive expected_object_epoch are required".into(),
        ));
    }
    validate_statement(&input.text)?;
    input.valid_time.validate()?;
    validate_exact_supports(&input.supports)?;
    validate_narrative_references(&input.references)?;
    Ok(())
}

pub fn support_evidence(evidence: EvidenceRef) -> RevisionSupport {
    RevisionSupport::Evidence(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facet_kind_has_fixed_order_without_other() {
        assert_eq!(SelfFacetKind::ORDER.len(), 7);
        assert_eq!(SelfFacetKind::ORDER[0].as_str(), "identity");
        assert_eq!(SelfFacetKind::ORDER[6].as_str(), "tendency");
        assert!(SelfFacetKind::try_from("other").is_err());
    }

    #[test]
    fn statement_limit_is_64_kibibytes() {
        assert!(validate_statement(&"x".repeat(MAX_STATEMENT_BYTES)).is_ok());
        assert!(validate_statement(&"x".repeat(64 * 1024)).is_ok());
        assert!(validate_statement(&"x".repeat(MAX_STATEMENT_BYTES + 1)).is_err());
        assert!(validate_statement(&"x".repeat(64 * 1024 + 1)).is_err());
    }

    fn seed_support() -> RevisionSupport {
        RevisionSupport::Seed(nous_core::CognitiveSeedVersionId::new())
    }

    fn facet_create() -> CreateSelfFacet {
        CreateSelfFacet {
            operation_id: OperationId::new(),
            subject: SubjectId::new(),
            kind: SelfFacetKind::Identity,
            key: "primary-name".into(),
            statement: "Nous".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![seed_support()],
            producer_signature_id: None,
        }
    }

    #[test]
    fn kind_and_revision_intent_vocabulary_is_frozen() {
        for value in [
            "identity",
            "role",
            "capability",
            "limitation",
            "tendency",
            "value",
            "preference",
        ] {
            assert!(SelfFacetKind::try_from(value).is_ok());
        }
        assert!(SelfFacetKind::try_from("other").is_err());
        assert_eq!(SelfRevisionIntent::Correct.as_str(), "correct");
        assert_eq!(SelfRevisionIntent::Refine.as_str(), "refine");
        assert_eq!(SelfRevisionIntent::Reinterpret.as_str(), "reinterpret");
        assert_eq!(SelfRevisionIntent::Evolve.as_str(), "evolve");
    }

    #[test]
    fn bounded_values_reject_empty_whitespace_and_overflow() {
        assert!(validate_key("primary-name").is_ok());
        assert!(validate_key(&"k".repeat(MAX_KEY_BYTES)).is_ok());
        assert!(validate_key("").is_err());
        assert!(validate_key("has whitespace").is_err());
        assert!(validate_key(&"k".repeat(MAX_KEY_BYTES + 1)).is_err());
        assert!(validate_scope("global").is_ok());
        assert!(validate_scope(&"s".repeat(MAX_SCOPE_BYTES)).is_ok());
        assert!(validate_scope(" ").is_err());
        assert!(validate_scope(&"s".repeat(MAX_SCOPE_BYTES + 1)).is_err());
        assert!(validate_statement("Nous").is_ok());
        assert!(validate_statement(" \n").is_err());
    }

    #[test]
    fn facet_validation_protects_operation_support_and_producer_rules() {
        let mut input = facet_create();
        assert!(validate_facet_create(&input).is_ok());
        input.operation_id = OperationId(uuid::Uuid::nil());
        assert!(validate_facet_create(&input).is_err());
        input = facet_create();
        input.supports.clear();
        assert!(validate_facet_create(&input).is_err());
        input = facet_create();
        input.epistemic_class = EpistemicClass::Derived;
        assert!(validate_facet_create(&input).is_err());
        input.producer_signature_id = Some(uuid::Uuid::now_v7());
        assert!(validate_facet_create(&input).is_ok());

        let mut revision = ReviseSelfFacet {
            operation_id: OperationId::new(),
            subject: input.subject,
            self_facet_id: nous_core::SelfFacetId::new(),
            expected_object_epoch: 1,
            parent_revision_id: nous_core::SelfFacetRevisionId::new(),
            revision_intent: SelfRevisionIntent::Refine,
            statement: "Nous Wave".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![seed_support()],
            producer_signature_id: None,
        };
        assert!(validate_facet_revision(&revision).is_ok());
        revision.expected_object_epoch = 0;
        assert!(validate_facet_revision(&revision).is_err());
    }

    #[test]
    fn narrative_references_are_exact_and_roles_are_bounded() {
        let valid = NarrativeReferenceInput {
            target_exact_ref: CognitiveRef::MemoryRevision(nous_core::MemoryRevisionId::new()),
            role: "identity".into(),
        };
        assert!(validate_narrative_references(std::slice::from_ref(&valid)).is_ok());
        let self_valid = NarrativeReferenceInput {
            target_exact_ref: CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId::new()),
            role: "identity".into(),
        };
        assert!(validate_narrative_references(&[self_valid]).is_ok());
        let mut mutable = valid.clone();
        mutable.target_exact_ref = CognitiveRef::Memory(nous_core::MemoryId::new());
        assert!(validate_narrative_references(&[mutable]).is_err());
        let mut empty_role = valid.clone();
        empty_role.role.clear();
        assert!(validate_narrative_references(&[empty_role]).is_err());
        let mut long_role = valid;
        long_role.role = "r".repeat(MAX_KEY_BYTES);
        assert!(validate_narrative_references(&[long_role.clone()]).is_ok());
        long_role.role = "r".repeat(MAX_KEY_BYTES + 1);
        assert!(validate_narrative_references(&[long_role]).is_err());
        assert!(validate_narrative_position(i32::MAX as usize).is_ok());
        assert!(validate_narrative_position(i32::MAX as usize + 1).is_err());
    }

    #[test]
    fn narrative_validation_protects_operation_and_text() {
        let mut input = CreateNarrativeIdentity {
            operation_id: OperationId::new(),
            subject: SubjectId::new(),
            key: "primary".into(),
            text: "I am Nous".into(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![seed_support()],
            references: Vec::new(),
            producer_signature_id: None,
        };
        assert!(validate_narrative_create(&input).is_ok());
        input.operation_id = OperationId(uuid::Uuid::nil());
        assert!(validate_narrative_create(&input).is_err());
        input = CreateNarrativeIdentity {
            operation_id: OperationId::new(),
            text: "".into(),
            ..input
        };
        assert!(validate_narrative_create(&input).is_err());
        let mut revision = ReviseNarrativeIdentity {
            operation_id: OperationId::new(),
            subject: input.subject,
            narrative_identity_id: nous_core::NarrativeIdentityId::new(),
            expected_object_epoch: 1,
            parent_revision_id: nous_core::NarrativeIdentityRevisionId::new(),
            revision_intent: SelfRevisionIntent::Refine,
            text: "I am Nous Wave".into(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![seed_support()],
            references: Vec::new(),
            producer_signature_id: None,
        };
        assert!(validate_narrative_revision(&revision).is_ok());
        revision.expected_object_epoch = 0;
        assert!(validate_narrative_revision(&revision).is_err());
    }
}
