//! Social Cognition value objects and semantic validation.

use chrono::{DateTime, Utc};
use nous_core::{
    CognitiveRef, EntityRef, EpistemicClass, Error, LanguageConventionId,
    LanguageConventionRevisionId, RelationTypeId, RelationshipAssertionId, RelationshipRevisionId,
    Result, RevisionSupport, SubjectId, TemporalExtent,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialEntityKind {
    Person,
    Group,
    Community,
    Channel,
}

impl SocialEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Group => "group",
            Self::Community => "community",
            Self::Channel => "channel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SocialParty {
    SubjectSelf,
    Entity {
        entity_kind: SocialEntityKind,
        entity_ref: EntityRef,
    },
}

impl SocialParty {
    pub fn canonical(&self) -> String {
        match self {
            Self::SubjectSelf => "subject".into(),
            Self::Entity {
                entity_kind,
                entity_ref,
            } => format!("{}:{}", entity_kind.as_str(), entity_ref.as_str()),
        }
    }

    pub fn entity_ref(&self) -> Option<&EntityRef> {
        match self {
            Self::SubjectSelf => None,
            Self::Entity { entity_ref, .. } => Some(entity_ref),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ViewSemantics {
    Directed,
    SymmetricView,
    InverseView { inverse_key: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DegreeSemantics {
    None,
    Ordinal { levels: Vec<String> },
    BoundedScalar { min: f64, max: f64 },
    TypedState { states: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalSemantics {
    State,
    Interval,
    Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationTypeDefinition {
    pub relation_type_id: RelationTypeId,
    pub subject_id: SubjectId,
    pub key: String,
    pub allowed_from_kinds: Vec<String>,
    pub allowed_to_kinds: Vec<String>,
    pub view_semantics: ViewSemantics,
    pub degree_semantics: DegreeSemantics,
    pub temporal_semantics: TemporalSemantics,
    pub created_at: DateTime<Utc>,
}

impl RelationTypeDefinition {
    pub fn validate(&self) -> Result<()> {
        validate_social_key(&self.key, "relation type key")?;
        validate_allowed_kinds(&self.allowed_from_kinds)?;
        validate_allowed_kinds(&self.allowed_to_kinds)?;
        if self.allowed_from_kinds.is_empty() || self.allowed_to_kinds.is_empty() {
            return Err(Error::Invalid(
                "relation type endpoint kinds cannot be empty".into(),
            ));
        }
        if let ViewSemantics::InverseView { inverse_key } = &self.view_semantics {
            validate_social_key(inverse_key, "inverse relation type key")?;
        }
        validate_degree(&self.degree_semantics)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipAssertion {
    pub relationship_id: RelationshipAssertionId,
    pub subject_id: SubjectId,
    pub relation_type_id: RelationTypeId,
    pub from: SocialParty,
    pub to: SocialParty,
    pub current_revision_id: RelationshipRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: String,
    pub integrity_state: String,
    pub suppression_state: String,
    pub purge_state: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRevision {
    pub relationship_revision_id: RelationshipRevisionId,
    pub relationship_id: RelationshipAssertionId,
    pub subject_id: SubjectId,
    pub revision_no: i32,
    pub parent_revision_id: Option<RelationshipRevisionId>,
    pub revision_intent: Option<String>,
    pub degree: Option<serde_json::Value>,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub producer_signature_id: Option<uuid::Uuid>,
    pub supports: Vec<RevisionSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialScope {
    Person(EntityRef),
    Dyad(SocialParty, SocialParty),
    Group(EntityRef),
    Community(EntityRef),
    Channel(EntityRef),
}

impl SocialScope {
    pub fn canonicalize(self) -> Result<Self> {
        match self {
            Self::Dyad(left, right) if left.canonical() == right.canonical() => Err(
                Error::Invalid("SocialScope dyad parties must differ".into()),
            ),
            Self::Dyad(left, right) if left.canonical() > right.canonical() => {
                Ok(Self::Dyad(right, left))
            }
            other => Ok(other),
        }
    }

    pub fn canonical(&self) -> String {
        match self {
            Self::Person(value) => format!("person:{}", value.as_str()),
            Self::Dyad(left, right) => format!("dyad:{}|{}", left.canonical(), right.canonical()),
            Self::Group(value) => format!("group:{}", value.as_str()),
            Self::Community(value) => format!("community:{}", value.as_str()),
            Self::Channel(value) => format!("channel:{}", value.as_str()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConvention {
    pub convention_id: LanguageConventionId,
    pub subject_id: SubjectId,
    pub key: String,
    pub expression: String,
    pub scope: SocialScope,
    pub context_scope: Option<String>,
    pub topic_scope: Option<String>,
    pub current_revision_id: LanguageConventionRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: String,
    pub integrity_state: String,
    pub suppression_state: String,
    pub purge_state: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConventionRevision {
    pub convention_revision_id: LanguageConventionRevisionId,
    pub convention_id: LanguageConventionId,
    pub subject_id: SubjectId,
    pub revision_no: i32,
    pub parent_revision_id: Option<LanguageConventionRevisionId>,
    pub revision_intent: Option<String>,
    pub meaning: String,
    pub pragmatic_role: Option<String>,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub producer_signature_id: Option<uuid::Uuid>,
    pub formation_policy_digest: String,
    pub supports: Vec<RevisionSupport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationEvidenceKind {
    SeedDirect,
    ExplicitExplanation,
    ExplicitConfirmation,
    ExternalConsistentUse,
    SuccessfulUnderstanding,
    RepairSequence,
    Contextual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConventionFormationEvidence {
    pub support_index: i32,
    pub kind: FormationEvidenceKind,
    pub external_actor: Option<EntityRef>,
}

pub fn validate_relationship_identity(from: &SocialParty, to: &SocialParty) -> Result<()> {
    if from == to {
        return Err(Error::Invalid("relationship endpoints must differ".into()));
    }
    Ok(())
}

pub fn validate_social_key(value: &str, label: &str) -> Result<()> {
    if (1..=128).contains(&value.len())
        && value.bytes().enumerate().all(|(index, byte)| {
            (index == 0 && byte.is_ascii_lowercase())
                || (index > 0
                    && (byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || b"._:-".contains(&byte)))
        })
    {
        Ok(())
    } else {
        Err(Error::Invalid(format!("invalid {label}")))
    }
}

fn validate_allowed_kinds(values: &[String]) -> Result<()> {
    if values.iter().all(|value| {
        matches!(
            value.as_str(),
            "subject" | "person" | "group" | "community" | "channel"
        )
    }) {
        Ok(())
    } else {
        Err(Error::Invalid("invalid allowed social party kind".into()))
    }
}

fn validate_degree(value: &DegreeSemantics) -> Result<()> {
    match value {
        DegreeSemantics::None => Ok(()),
        DegreeSemantics::Ordinal { levels } if (2..=16).contains(&levels.len()) => Ok(()),
        DegreeSemantics::BoundedScalar { min, max }
            if min.is_finite() && max.is_finite() && min < max =>
        {
            Ok(())
        }
        DegreeSemantics::TypedState { states } if (1..=32).contains(&states.len()) => Ok(()),
        _ => Err(Error::Invalid("invalid social degree semantics".into())),
    }
}

pub fn exact_ref(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::RelationshipRevision(_) | CognitiveRef::LanguageConventionRevision(_)
    )
}
