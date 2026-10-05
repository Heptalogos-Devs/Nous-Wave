//! Bounded, separately granted concept and association maintenance.
mod commit;
mod context;
pub use commit::{CommitTopologyInput, TopologyOutcome};
mod plan;
mod proof;
use super::*;
use nous_configuration::*;
use std::collections::BTreeMap;
pub const TOPOLOGY_MAINTENANCE: ConfigKey<TopologyMaintenancePolicy> =
    ConfigKey::new("maintenance.topology");
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopologyMaintenancePolicy {
    pub max_actions: usize,
    pub max_cognition: usize,
    pub max_tags: usize,
    pub max_associations: usize,
    pub max_supports: usize,
    pub text_chars: usize,
}
impl Default for TopologyMaintenancePolicy {
    fn default() -> Self {
        Self {
            max_actions: 8,
            max_cognition: 16,
            max_tags: 32,
            max_associations: 32,
            max_supports: 64,
            text_chars: 16384,
        }
    }
}
pub(super) fn register_topology_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        TOPOLOGY_MAINTENANCE,
        "memory",
        "Bounded topology maintenance action, catalog and text envelope.",
        TopologyMaintenancePolicy::default(),
        ConfigExposure::Developer,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |p| {
            if !(1..=16).contains(&p.max_actions)
                || !(1..=32).contains(&p.max_cognition)
                || !(1..=64).contains(&p.max_tags)
                || !(1..=64).contains(&p.max_associations)
                || !(1..=128).contains(&p.max_supports)
                || !(1024..=32768).contains(&p.text_chars)
            {
                return Err(Error::Invalid(
                    "topology maintenance envelope exceeded".into(),
                ));
            }
            Ok(())
        },
    )
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyCognition {
    pub key: String,
    pub reference: CognitiveRef,
    pub epoch: i64,
    pub text: String,
    pub semantic_similarity: Option<f64>,
    pub use_summary: BTreeMap<String, u64>,
    pub provenance_roots: Vec<String>,
    pub context: TopologyContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyContext {
    pub cognitive_role: Option<String>,
    pub semantic_role: Option<String>,
    pub time: Option<TemporalExtent>,
    pub entities: Vec<EntityRef>,
    pub members: Vec<(CognitiveRef, String)>,
    pub source_support_keys: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyTag {
    pub key: String,
    pub target: TagExpectation,
    pub content: TagContent,
    pub aliases: Vec<String>,
    pub accretion: Option<AccretionSignal>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyAssociation {
    pub key: String,
    pub id: AssociationEvidenceId,
    pub from: String,
    pub to: String,
    pub relation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyPlan {
    pub subject: SubjectId,
    pub focus: CognitiveRef,
    pub authority_seq: i64,
    pub config_digest: String,
    pub policy: TopologyMaintenancePolicy,
    pub cognition: Vec<TopologyCognition>,
    pub tags: Vec<TopologyTag>,
    pub associations: Vec<TopologyAssociation>,
    pub entities: BTreeMap<String, (EntityRef, String)>,
    pub supports: BTreeMap<String, AssociationSupport>,
    pub source_context: BTreeMap<String, serde_json::Value>,
    pub partial: bool,
    pub merge_candidates: Vec<(String, String, f64)>,
    pub split_candidates: Vec<String>,
}
impl TopologyPlan {
    pub fn model_input(&self) -> serde_json::Value {
        let mut value = serde_json::json!({"focusKey":"c0","policy":self.policy,"partial":self.partial,
            "cognition":self.cognition.iter().map(|c|serde_json::json!({"key":c.key,"text":c.text,"semanticSimilarity":c.semantic_similarity,"use":c.use_summary,"provenanceRoots":c.provenance_roots,
                "kind":reference_parts(&c.reference).0,"context":{
                    "cognitiveRole":c.context.cognitive_role,"semanticRole":c.context.semantic_role,"time":c.context.time,
                    "sourceSupportKeys":c.context.source_support_keys,
                    "entityKeys":c.context.entities.iter().filter_map(|entity|self.entities.iter().find(|(_, (reference,_))|reference==entity).map(|(key,_)|key)).collect::<Vec<_>>(),
                    "members":c.context.members.iter().enumerate().map(|(ordinal,(reference,role))|serde_json::json!({"ordinal":ordinal,"targetKey":self.cognition.iter().find(|c|&c.reference==reference).map(|c|&c.key),"sourceKeys":self.supports.iter().filter_map(|(key,support)|match (reference,support) {(CognitiveRef::Occurrence(id),AssociationSupport::Revision(RevisionSupport::Evidence(e))) if *id==e.occurrence_id => Some(key), _=>None}).collect::<Vec<_>>(),"role":role})).collect::<Vec<_>>()
                }})).collect::<Vec<_>>(),
            "tags":self.tags.iter().map(|t|serde_json::json!({"key":t.key,"content":t.content,"aliases":t.aliases,"accretion":t.accretion.as_ref().map(|signal| {let mut signal=signal.clone();signal.member_keys=signal.member_keys.iter().filter_map(|reference|self.cognition.iter().find(|c|c.reference.to_string()==*reference).map(|c|c.key.clone())).collect();signal})})).collect::<Vec<_>>(),
            "entities":self.entities.iter().map(|(key,(_,text))|serde_json::json!({"key":key,"text":text})).collect::<Vec<_>>(),
            "associations":self.associations.iter().map(|a|serde_json::json!({"key":a.key,"from":a.from,"to":a.to,"relation":a.relation})).collect::<Vec<_>>(),
            "mergeCandidates":self.merge_candidates,"splitCandidates":self.split_candidates,
            "sourceContext":self.source_context,
            "supports":self.supports.iter().map(|(key,support)| {
                let (kind,target)=match support {
                    AssociationSupport::Revision(RevisionSupport::CognitionDependency(d))=>("exact_cognition",self.cognition.iter().find(|c|c.reference==d.target_revision).map(|c|c.key.clone())),
                    AssociationSupport::Revision(_)=>("source_evidence",None),
                    AssociationSupport::UseEvent(_)=>("meaningful_use",None),
                };serde_json::json!({"key":key,"kind":kind,"targetKey":target})
            }).collect::<Vec<_>>()});
        let mut remaining = self.policy.text_chars;
        let mut partial = self.partial;
        truncate_model_text(&mut value, &mut remaining, &mut partial);
        if let Some(object) = value.as_object_mut() {
            object.insert("partial".into(), serde_json::Value::Bool(partial));
        }
        value
    }
    pub(super) fn catalog_identity(&self) -> serde_json::Value {
        let mut plan = self.clone();
        plan.partial = false;
        plan.merge_candidates.clear();
        plan.split_candidates.clear();
        for tag in &mut plan.tags {
            tag.accretion = None;
        }
        for c in &mut plan.cognition {
            c.semantic_similarity = None;
        }
        serde_json::to_value(plan).expect("typed topology catalog serialization")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewTopologyTag {
    pub key: String,
    pub content: TagContent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TopologyAction {
    ReuseTag {
        tag_key: String,
    },
    CreateTag {
        key: String,
        content: TagContent,
        support_keys: Vec<String>,
        reason: String,
    },
    ReviseTag {
        tag_key: String,
        content: TagContent,
        support_keys: Vec<String>,
        reason: String,
    },
    AttachTag {
        cognition_key: String,
        tag_key: String,
        support_keys: Vec<String>,
        reason: String,
    },
    DetachTag {
        association_key: String,
        support_keys: Vec<String>,
        reason: String,
    },
    CreateAssociation {
        from_key: String,
        to_key: String,
        relation: TopologyRelation,
        support_keys: Vec<String>,
        reason: String,
    },
    RevokeAssociation {
        association_key: String,
        support_keys: Vec<String>,
        reason: String,
    },
    MergeTags {
        survivor_key: String,
        retired_keys: Vec<String>,
        support_keys: Vec<String>,
        reason: String,
    },
    SplitTag {
        tag_key: String,
        children: Vec<NewTopologyTag>,
        support_keys: Vec<String>,
        reason: String,
    },
    NoChange,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyProposal {
    pub actions: Vec<TopologyAction>,
}

impl MemoryService {
    pub(crate) async fn enqueue_topology_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        mut reference: CognitiveRef,
        sequence: i64,
    ) -> Result<()> {
        if let CognitiveRef::CognitiveSchema(id) = reference {
            let revision:Uuid=sqlx::query_scalar("SELECT current_revision_id FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(id.0).fetch_one(&mut **tx).await.map_err(db)?;
            reference = CognitiveRef::CognitiveSchemaRevision(CognitiveSchemaRevisionId(revision));
        }
        let (kind, value) = reference_parts(&reference);
        self.cognition
            .enqueue_maintenance_in(
                tx,
                &nous_runtime::MaintenanceRequest {
                    subject,
                    kind: "topology_maintenance".into(),
                    scope_kind: kind,
                    scope_ref: value,
                    trigger_authority_seq: sequence,
                    due_at: self.cognition.now(subject),
                    priority: 20,
                },
            )
            .await?;
        Ok(())
    }
    pub async fn request_topology_review(
        &self,
        subject: SubjectId,
        focus: CognitiveRef,
    ) -> Result<Uuid> {
        let (focus, _, _) = self.store.bind_exact_reference(subject, &focus).await?;
        self.plan_topology(subject, focus.clone()).await?;
        let (kind, value) = reference_parts(&focus);
        self.cognition
            .enqueue_maintenance(nous_runtime::MaintenanceRequest {
                subject,
                kind: "topology_maintenance".into(),
                scope_kind: kind,
                scope_ref: value,
                trigger_authority_seq: self.store.authority_seq(subject).await?,
                due_at: self.cognition.now(subject),
                priority: 30,
            })
            .await
    }
}

fn truncate_model_text(value: &mut serde_json::Value, remaining: &mut usize, partial: &mut bool) {
    match value {
        serde_json::Value::Object(object) => {
            for key in [
                "text",
                "label",
                "kind_hint",
                "description",
                "aliases",
                "role",
                "sourceClass",
                "semanticRole",
            ] {
                if let Some(value) = object.get_mut(key) {
                    truncate_text_value(value, remaining, partial);
                }
            }
            for (key, value) in object {
                if !matches!(
                    key.as_str(),
                    "text"
                        | "label"
                        | "description"
                        | "kind_hint"
                        | "aliases"
                        | "role"
                        | "sourceClass"
                        | "semanticRole"
                ) {
                    truncate_model_text(value, remaining, partial);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                truncate_model_text(value, remaining, partial);
            }
        }
        _ => {}
    }
}
fn truncate_text_value(value: &mut serde_json::Value, remaining: &mut usize, partial: &mut bool) {
    match value {
        serde_json::Value::String(text) => {
            let clipped = text.chars().take(*remaining).collect::<String>();
            *partial |= clipped != *text;
            *remaining = remaining.saturating_sub(clipped.chars().count());
            *text = clipped;
        }
        serde_json::Value::Array(values) => {
            for value in values {
                truncate_text_value(value, remaining, partial);
            }
        }
        _ => {}
    }
}
