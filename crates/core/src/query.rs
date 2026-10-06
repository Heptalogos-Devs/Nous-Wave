// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueryTarget {
    EntityNeighborhood { entity_ref: EntityRef },
    SchemaNeighborhood { schema: CognitiveSchemaId },
    Exact { reference: CognitiveRef },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultDomain {
    Memory,
    Schema,
    Episode,
    Journal,
    Evidence,
    Resource,
}
impl ResultDomain {
    pub fn name(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Schema => "schema",
            Self::Episode => "episode",
            Self::Journal => "journal",
            Self::Evidence => "evidence",
            Self::Resource => "resource",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultProjection {
    pub domains: Vec<ResultDomain>,
}
impl Default for ResultProjection {
    fn default() -> Self {
        Self {
            domains: vec![
                ResultDomain::Memory,
                ResultDomain::Schema,
                ResultDomain::Episode,
                ResultDomain::Journal,
            ],
        }
    }
}
impl ResultProjection {
    pub fn domain_names(&self) -> Vec<&'static str> {
        self.domains.iter().map(|domain| domain.name()).collect()
    }
    pub fn allows_reference(&self, reference: &CognitiveRef) -> bool {
        self.domain_names()
            .contains(&reference_query_domain(reference))
    }
    pub fn has_cognition(&self) -> bool {
        self.domains.iter().any(|domain| {
            matches!(
                domain,
                ResultDomain::Memory
                    | ResultDomain::Schema
                    | ResultDomain::Episode
                    | ResultDomain::Journal
            )
        })
    }
    pub fn validate(&self) -> Result<()> {
        let unique: std::collections::BTreeSet<_> = self.domains.iter().collect();
        if self.domains.is_empty() || unique.len() != self.domains.len() {
            return Err(Error::Invalid("invalid result projection".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", content = "at", rename_all = "snake_case")]
pub enum AuthorityView {
    #[default]
    Current,
    AsOf(DateTime<Utc>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RevisionView {
    #[default]
    Current,
    History,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFrame {
    pub clock_now: DateTime<Utc>,
    pub authority_view: AuthorityView,
    pub revision_view: RevisionView,
    #[serde(default)]
    pub original_expressions: Vec<String>,
}
impl Default for TemporalFrame {
    fn default() -> Self {
        Self {
            clock_now: DateTime::<Utc>::UNIX_EPOCH,
            authority_view: AuthorityView::Current,
            revision_view: RevisionView::Current,
            original_expressions: Vec::new(),
        }
    }
}

pub fn reference_query_domain(reference: &CognitiveRef) -> &'static str {
    match reference {
        CognitiveRef::Memory(_) | CognitiveRef::MemoryRevision(_) => "memory",
        CognitiveRef::CognitiveSchema(_) | CognitiveRef::CognitiveSchemaRevision(_) => "schema",
        CognitiveRef::Episode(_) | CognitiveRef::EpisodeRevision(_) => "episode",
        CognitiveRef::Journal(_) | CognitiveRef::JournalRevision(_) => "journal",
        CognitiveRef::Resource(_) => "resource",
        CognitiveRef::Tag(_) => "concept",
        _ => "evidence",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCue {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptCue {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCue {
    pub entity_ref: EntityRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectCue {
    pub object_ref: ObjectRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactCue {
    pub artifact: ArtifactId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaRegionCue {
    pub region: CognitiveRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCue {
    pub tag: TagId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCue {
    pub schema: CognitiveSchemaId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationCue {
    pub from: CognitiveRef,
    pub to: CognitiveRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleCue {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCue {
    pub resource: ResourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cue {
    Text(TextCue),
    Concept(ConceptCue),
    Entity(EntityCue),
    Object(ObjectCue),
    Artifact(ArtifactCue),
    MediaRegion(MediaRegionCue),
    Tag(TagCue),
    Schema(SchemaCue),
    Relation(RelationCue),
    Example(ExampleCue),
    Resource(ResourceCue),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryConstraints {
    #[serde(default)]
    pub current_authority: CurrentAuthorityNeed,
    #[serde(default)]
    pub source_classes_include: Vec<SourceClass>,
    #[serde(default)]
    pub source_classes_exclude: Vec<SourceClass>,
    #[serde(default)]
    pub cognitive_roles_include: Vec<String>,
    #[serde(default)]
    pub formation_modes_include: Vec<String>,
    #[serde(default)]
    pub entity_requirements: Vec<EntityRef>,
    pub occurred: Option<TimeInterval>,
    pub observed: Option<TimeInterval>,
    pub valid: Option<TimeInterval>,
    pub formed: Option<TimeInterval>,
    pub recorded: Option<TimeInterval>,
    #[serde(default)]
    pub include_suppressed: bool,
    pub authority: Option<AuthorityClass>,
    #[serde(default)]
    pub modalities: Vec<Modality>,
    #[serde(default)]
    pub evidence_classes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExplorationIntent {
    #[default]
    None,
    BoundedAssociative,
    AroundTag,
    AroundSchema,
    ExplainAssociation,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CurrentAuthorityNeed {
    #[default]
    None,
    Prefer,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceIntent {
    #[serde(default)]
    pub synopsis_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultNeed {
    #[serde(default = "default_result_limit")]
    pub limit: usize,
    #[serde(default = "default_true")]
    pub need_evidence: bool,
    #[serde(default)]
    pub need_materialization_handles: bool,
}

pub const MAX_QUERY_RESULT_ITEMS: usize = 2048;

fn default_result_limit() -> usize {
    // Explicit Rust callers may select the reference default; public requests
    // resolve omitted limits through the Runtime Configuration snapshot.
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../config/reference/retrieval-ranking-v1.json"
    ))
    .expect("retrieval reference profile");
    reference["values"]["retrieval.query.default_result_limit"]
        .as_u64()
        .expect("reference result limit") as usize
}

fn default_true() -> bool {
    true
}

impl Default for ResultNeed {
    fn default() -> Self {
        Self {
            limit: default_result_limit(),
            need_evidence: true,
            need_materialization_handles: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveEffort {
    Light,
    #[default]
    Normal,
    Deep,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPolicy {
    #[serde(default)]
    pub rerank: RequirementStrength,
    #[serde(default)]
    pub text_embedding: RequirementStrength,
    #[serde(default)]
    pub multimodal_interpretation: RequirementStrength,
    #[serde(default)]
    pub residual_sensing: RequirementStrength,
    #[serde(default)]
    pub query_concept_enrichment: RequirementStrength,
}

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self {
            rerank: RequirementStrength::Optional,
            text_embedding: RequirementStrength::Optional,
            multimodal_interpretation: RequirementStrength::Optional,
            residual_sensing: RequirementStrength::Optional,
            query_concept_enrichment: RequirementStrength::Optional,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsRequest {
    None,
    #[default]
    Summary,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveQueryExpr {
    pub operation: QueryOperation,
    #[serde(default)]
    pub targets: Vec<QueryTarget>,
    #[serde(default)]
    pub cues: Vec<Cue>,
    #[serde(default)]
    pub constraints: QueryConstraints,
    #[serde(default)]
    pub children: Vec<CognitiveQueryExpr>,
    #[serde(default)]
    pub preferences: Vec<QueryPreference>,
}

impl Default for CognitiveQueryExpr {
    fn default() -> Self {
        Self {
            operation: QueryOperation::Atom,
            targets: Vec::new(),
            cues: Vec::new(),
            constraints: QueryConstraints::default(),
            children: Vec::new(),
            preferences: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPreference {
    pub negative: bool,
    pub operand: PreferenceOperand,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreferenceOperand {
    Cue(Cue),
    Exact(CognitiveRef),
    Recent(TimeAxis),
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimeAxis {
    Occurred,
    Observed,
    Valid,
    Formed,
    Recorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOperation {
    Atom,
    All,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveQuery {
    #[serde(default)]
    pub text_only_compatibility: bool,
    #[serde(default)]
    pub projection: ResultProjection,
    #[serde(default)]
    pub temporal_frame: TemporalFrame,
    #[serde(default)]
    pub work_context: Option<uuid::Uuid>,
    pub api_version: u32,
    #[serde(default)]
    pub subject: SubjectId,
    pub session: Option<SessionId>,
    #[serde(default)]
    pub situation: SituationDescriptor,
    pub expression: CognitiveQueryExpr,
    #[serde(default)]
    pub exploration: ExplorationIntent,
    #[serde(default)]
    pub resources: ResourceIntent,
    #[serde(default)]
    pub result_need: ResultNeed,
    #[serde(default)]
    pub effort: CognitiveEffort,
    #[serde(default)]
    pub capabilities: CapabilityPolicy,
    #[serde(default)]
    pub diagnostics: DiagnosticsRequest,
}

impl CognitiveQuery {
    pub fn requests_resources(&self) -> bool {
        self.resources.synopsis_only
            || self.exploration == ExplorationIntent::Global
            || self.scopes().iter().any(|scope| {
                scope.constraints.current_authority != CurrentAuthorityNeed::None
                    || scope.cues.iter().any(|cue| matches!(cue, Cue::Resource(_)))
            })
    }
    pub fn scopes(&self) -> Vec<&CognitiveQueryExpr> {
        let mut pending = vec![&self.expression];
        let mut scopes = Vec::new();
        while let Some(node) = pending.pop() {
            scopes.push(node);
            pending.extend(node.children.iter().rev());
        }
        scopes
    }
    pub fn validate(&self) -> Result<()> {
        self.projection.validate()?;
        if self.api_version != API_VERSION {
            return Err(Error::Invalid(format!(
                "unsupported cognitive query api_version {}",
                self.api_version
            )));
        }
        if self.result_need.limit == 0 || self.result_need.limit > MAX_QUERY_RESULT_ITEMS {
            return Err(Error::Invalid(
                "result_need.limit must be between 1 and 2048".into(),
            ));
        }
        let mut nodes = vec![(&self.expression, 0)];
        let mut count = 0;
        while let Some((node, depth)) = nodes.pop() {
            count += 1;
            if count > 64
                || depth > 16
                || node.cues.len() > 256
                || node.targets.len() > 128
                || node.preferences.len() > 16
            {
                return Err(Error::Invalid(
                    "query tree/cue/target bound exceeded".into(),
                ));
            }
            if (node.operation == QueryOperation::Atom && !node.children.is_empty())
                || (node.operation != QueryOperation::Atom
                    && (node.children.len() < 2 || !node.cues.is_empty()))
            {
                return Err(Error::Invalid("invalid query expression shape".into()));
            }
            for interval in [
                node.constraints.occurred,
                node.constraints.observed,
                node.constraints.valid,
                node.constraints.formed,
                node.constraints.recorded,
            ]
            .into_iter()
            .flatten()
            {
                interval.validate()?;
            }
            nodes.extend(node.children.iter().map(|child| (child, depth + 1)));
        }
        if self.capabilities.text_embedding == RequirementStrength::Forbidden
            && self.capabilities.residual_sensing == RequirementStrength::Required
        {
            return Err(Error::Invalid(
                "required residual_sensing conflicts with forbidden text_embedding".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryStatus {
    Complete,
    Degraded,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Degradation {
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryGenerationTrace {
    pub lexical: Option<ServingGenerationId>,
    #[serde(default)]
    pub dense: Vec<ServingGenerationId>,
    pub topology: Option<ServingGenerationId>,
    pub epa_basis: Option<ServingGenerationId>,
    pub postings: Option<ServingGenerationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFamily {
    Exact,
    Runtime,
    Entity,
    Lexical,
    Dense,
    Temporal,
    SchemaDirect,
    TagDirect,
    TopologyWave,
    Resource,
    LanguageRerank,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchEvidence {
    #[serde(default)]
    pub families: Vec<EvidenceFamily>,
    pub base_rank_score: f64,
    pub preference_score: f64,
    pub rerank_score: Option<f64>,
    pub baseline_rank: u32,
    pub final_rank: u32,
    pub best_lane_rank: u32,
    pub enabled_lane_count: u32,
    pub final_score: f64,
    #[serde(default)]
    pub variants: Vec<String>,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveHit {
    pub authority_epoch: Option<i64>,
    #[serde(default)]
    pub preference_refs: Vec<CognitiveRef>,
    pub reference: CognitiveRef,
    pub revision: Option<CognitiveRef>,
    pub semantic_role: Option<String>,
    pub cognitive_role: Option<String>,
    pub formation_mode: Option<String>,
    pub representation: Option<String>,
    pub authority: AuthorityClass,
    pub freshness: FreshnessDescriptor,
    #[serde(default)]
    pub entity_refs: Vec<EntityRef>,
    #[serde(default)]
    pub evidence: Vec<EvidenceHandle>,
    pub match_evidence: MatchEvidence,
    #[serde(default)]
    pub materialization: Vec<MaterializationHandle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceActionSuggestion {
    pub action_id: Uuid,
    pub query_text: String,
    pub limit: usize,
    pub materialize: bool,
    pub adapter_kind: String,
    pub provider_profile: String,
    pub provider_locator: String,
    pub descriptor_digest: String,
    pub resource: ResourceRef,
    pub action: String,
    pub reason: String,
    #[serde(default)]
    pub current_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StableExternalRef {
    pub provider_kind: String,
    pub provider_profile: String,
    pub profile_digest: String,
    pub resource_ref: ResourceRef,
    pub provider_resource_id: String,
    pub entry_id: String,
    pub entry_version: Option<String>,
    pub content_digest: String,
    pub source_locator: String,
    pub retrieved_at: String,
    pub access_scope: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalResourceRecord {
    pub resource_ref: ResourceRef,
    pub reference: StableExternalRef,
    pub title: Option<String>,
    pub content: String,
    pub provider_rank: u32,
    pub provider_score: Option<f64>,
    pub version_status: String,
    pub access_status: String,
}
#[derive(Debug, Clone)]
pub struct ExternalResourceResult {
    pub action_id: Uuid,
    pub resource_ref: ResourceRef,
    pub status: String,
    pub records: Vec<ExternalResourceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDiagnostics {
    pub candidate_counts: std::collections::BTreeMap<String, usize>,
    pub lane_status: std::collections::BTreeMap<String, String>,
    pub topology_complete: Option<bool>,
    pub topology_discarded_mass: Option<f64>,
    pub trace: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveQueryResult {
    pub query_id: Uuid,
    pub generation: QueryGenerationTrace,
    pub status: QueryStatus,
    pub results: Vec<CognitiveHit>,
    #[serde(default)]
    pub resource_actions: Vec<ResourceActionSuggestion>,
    pub resource_records: Vec<ExternalResourceRecord>,
    #[serde(default)]
    pub degradation: Vec<Degradation>,
    pub diagnostics: Option<QueryDiagnostics>,
}

/// Semantic serving families required by one query.  This stays independent
/// of the concrete index implementation used to serve each family.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServingNeed {
    pub exact: bool,
    pub lexical: bool,
    pub dense: bool,
    pub topology: bool,
    pub concept: bool,
    pub concept_vectors: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Readiness {
    Ready,
    Degraded,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityStatus {
    pub capability_id: String,
    pub status: Readiness,
    pub reason: Option<String>,
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn default_projection_admits_only_cognition_even_for_exact_refs() {
        let projection = ResultProjection::default();
        for reference in [
            CognitiveRef::MemoryRevision(MemoryRevisionId::new()),
            CognitiveRef::CognitiveSchemaRevision(CognitiveSchemaRevisionId::new()),
            CognitiveRef::EpisodeRevision(EpisodeRevisionId::new()),
            CognitiveRef::JournalRevision(JournalRevisionId::new()),
        ] {
            assert!(projection.allows_reference(&reference));
        }
        assert!(!projection.allows_reference(&CognitiveRef::Artifact(ArtifactId::new())));
        assert!(!projection.allows_reference(&CognitiveRef::Resource(
            ResourceRef::new("resource:test").unwrap()
        )));
        let evidence = ResultProjection {
            domains: vec![ResultDomain::Evidence],
        };
        assert!(evidence.allows_reference(&CognitiveRef::Artifact(ArtifactId::new())));
        assert!(!evidence.allows_reference(&CognitiveRef::MemoryRevision(MemoryRevisionId::new())));
        assert!(ResultProjection { domains: vec![] }.validate().is_err());
        assert!(
            ResultProjection {
                domains: vec![ResultDomain::Memory, ResultDomain::Memory]
            }
            .validate()
            .is_err()
        );
    }
}
