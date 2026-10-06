use crate::{CognitiveRuntimeService, WorkContextState, WorkContextView};
use nous_configuration::*;
use nous_core::*;
use nous_persistence::QueryDescriptor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryRepresentationLimits {
    pub total_chars: usize,
    pub max_context_items: usize,
}

impl Default for QueryRepresentationLimits {
    fn default() -> Self {
        Self {
            total_chars: 8192,
            max_context_items: 16,
        }
    }
}
pub const QUERY_REPRESENTATION: ConfigKey<QueryRepresentationLimits> =
    ConfigKey::new("retrieval.query.representation");
pub(super) fn register(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        QUERY_REPRESENTATION,
        "runtime",
        "Bounded semantic Query Representation inputs.",
        QueryRepresentationLimits::default(),
        ConfigExposure::Developer,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::QueryPolicy,
        |v| {
            if !(256..=32768).contains(&v.total_chars) || v.max_context_items > 64 {
                return Err(Error::Invalid("invalid query representation bounds".into()));
            }
            Ok(())
        },
    )
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryRepresentation {
    pub representation_version: String,
    pub text: String,
    pub sha256: String,
    pub source_refs: Vec<(CognitiveRef, String)>,
    pub truncation_flags: Vec<String>,
    pub work_context: Option<WorkContextView>,
}

struct Builder {
    sections: Vec<(&'static str, String)>,
    remaining: usize,
    flags: BTreeSet<String>,
}
impl Builder {
    fn section(&mut self, label: &'static str, value: &str, limit: usize) {
        if value.trim().is_empty() {
            return;
        }
        if value.trim().chars().count() > limit {
            self.flags.insert(label.into());
        }
        self.sections
            .push((label, value.trim().chars().take(limit).collect()));
    }
    fn render(&mut self) -> String {
        // Allocation priority is independent of the fixed semantic section order.
        let priority = |label| match label {
            "Intent" => 0,
            "Temporal orientation" => 1,
            "Entities" | "Concepts" => 2,
            "Schemas" | "Current cognition" => 3,
            "Current work" => 4,
            _ => 5,
        };
        let mut indices = (0..self.sections.len()).collect::<Vec<_>>();
        indices.sort_by_key(|i| priority(self.sections[*i].0));
        for index in indices {
            let (label, value) = &mut self.sections[index];
            let header = label.chars().count() + 4; // label, colon, newline and section separation
            let available = self.remaining.saturating_sub(header);
            if value.chars().count() > available {
                self.flags.insert((*label).into());
                *value = value.chars().take(available).collect();
            }
            if !value.is_empty() {
                self.remaining -= header + value.chars().count();
            }
        }
        self.sections
            .iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(label, value)| format!("{label}:\n{value}"))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn intent(node: &CognitiveQueryExpr) -> String {
    let own = node
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Text(v) => Some(v.text.as_str()),
            Cue::Example(v) => Some(v.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let parts = node
        .children
        .iter()
        .map(intent)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return own;
    }
    format!(
        "{}{}: {}",
        if own.is_empty() {
            String::new()
        } else {
            format!("{own}; ")
        },
        if node.operation == QueryOperation::All {
            "ALL OF"
        } else {
            "ANY OF"
        },
        parts.join("; ")
    )
}

pub fn build_query_representation(
    query: &CognitiveQuery,
    descriptors: &[QueryDescriptor],
    context: Option<WorkContextView>,
    sources: Vec<(CognitiveRef, String)>,
    limits: &QueryRepresentationLimits,
) -> QueryRepresentation {
    let raw = intent(&query.expression);
    if query.text_only_compatibility {
        return finish(raw, sources, BTreeSet::new(), None, "text-compatibility-v1");
    }
    let mut builder = Builder {
        sections: Vec::new(),
        remaining: limits.total_chars,
        flags: BTreeSet::new(),
    };
    builder.section("Intent", &raw, 2048);
    let temporal = query.scopes().iter().filter_map(|scope| {
        let c=&scope.constraints;
        let recent=scope.preferences.iter().filter_map(|p| match p.operand { PreferenceOperand::Recent(axis) => Some(format!("{}recent({axis:?})",if p.negative { "avoid " } else { "prefer " })), _ => None }).collect::<Vec<_>>();
        if [c.occurred,c.observed,c.valid,c.formed,c.recorded].iter().all(Option::is_none) && recent.is_empty() { return None; }
        Some(serde_json::json!({"occurred":c.occurred,"observed":c.observed,"valid":c.valid,"formed":c.formed,"recorded":c.recorded,"preferences":recent}).to_string())
    }).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>().join("\n");
    builder.section("Temporal orientation", &temporal, 1024);
    for (label, select, maximum, chars) in [
        ("Entities", "entity", limits.max_context_items, 256),
        ("Concepts", "tag", limits.max_context_items, 256),
        ("Schemas", "cognitive_schema", limits.max_context_items, 512),
        (
            "Current cognition",
            "current",
            limits.max_context_items,
            512,
        ),
        ("Resources", "resource", limits.max_context_items, 512),
    ] {
        let values = descriptors
            .iter()
            .filter(|d| {
                let kind = reference_parts(&d.reference).0;
                if select == "current" {
                    !matches!(
                        kind.as_str(),
                        "entity" | "tag" | "cognitive_schema" | "resource" | "external_object"
                    )
                } else {
                    kind == select
                }
            })
            .map(|d| d.text.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect::<BTreeSet<_>>();
        if values.len() > maximum {
            builder.flags.insert(label.into());
        }
        let text = values
            .into_iter()
            .take(maximum)
            .map(|text| {
                if text.chars().count() > chars {
                    builder.flags.insert(label.into());
                }
                format!("- {}", text.chars().take(chars).collect::<String>())
            })
            .collect::<Vec<_>>()
            .join("\n");
        builder.section(label, &text, limits.total_chars);
    }
    let objects = query
        .situation
        .object_descriptions
        .values()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n");
    builder.section("Current objects", &objects, 512 * limits.max_context_items);
    if let Some(context) = &context {
        builder.section(
            "Current work",
            &format!(
                "{}\n{}",
                context.purpose,
                context.unresolved_questions.join("\n")
            ),
            1024,
        );
    }
    if let Some(consumer) = &query.situation.consumer {
        builder.section("Consumer/task", consumer, 256);
    }
    let text = builder.render();
    finish(text, sources, builder.flags, context, "cognitive-query-v2")
}
fn finish(
    text: String,
    mut sources: Vec<(CognitiveRef, String)>,
    flags: BTreeSet<String>,
    context: Option<WorkContextView>,
    version: &str,
) -> QueryRepresentation {
    sources.sort_by_key(|(reference, source)| (reference.to_string(), source.clone()));
    sources.dedup();
    QueryRepresentation {
        representation_version: version.into(),
        sha256: Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        text,
        source_refs: sources,
        truncation_flags: flags.into_iter().collect(),
        work_context: context,
    }
}

impl CognitiveRuntimeService {
    pub(super) async fn resolve_query_representation(
        &self,
        query: &CognitiveQuery,
        work_context: Option<WorkContextView>,
        mut representation_sources: Vec<(CognitiveRef, String)>,
        config_snapshot: &ConfigSnapshot,
    ) -> Result<QueryRepresentation> {
        for cue in query.scopes().iter().flat_map(|node| &node.cues) {
            let reference = match cue {
                Cue::Entity(v) => Some(CognitiveRef::Entity(v.entity_ref.clone())),
                Cue::Tag(v) => Some(CognitiveRef::Tag(v.tag)),
                Cue::Schema(v) => Some(CognitiveRef::CognitiveSchema(v.schema)),
                Cue::Resource(v) => Some(CognitiveRef::Resource(v.resource.clone())),
                Cue::Object(v) => Some(CognitiveRef::ExternalObject(v.object_ref.clone())),
                Cue::Artifact(v) => Some(CognitiveRef::Artifact(v.artifact)),
                Cue::MediaRegion(v) => Some(v.region.clone()),
                Cue::Relation(v) => {
                    representation_sources.push((v.from.clone(), "explicit_relation".into()));
                    Some(v.to.clone())
                }
                _ => None,
            };
            if let Some(reference) = reference {
                representation_sources.push((reference, "explicit_cue".into()));
            }
        }
        let limits = config_snapshot.get(QUERY_REPRESENTATION)?;
        let mut selected = representation_sources
            .iter()
            .filter(|(_, source)| source.starts_with("explicit"))
            .map(|(reference, _)| reference.clone())
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        selected.retain(|reference| seen.insert(reference.clone()));
        let current_count = representation_sources
            .iter()
            .filter(|(_, source)| !source.starts_with("explicit"))
            .map(|(reference, _)| reference.clone())
            .collect::<HashSet<_>>()
            .len();
        selected.extend(
            representation_sources
                .iter()
                .filter(|(_, source)| !source.starts_with("explicit"))
                .map(|(reference, _)| reference.clone())
                .filter(|reference| seen.insert(reference.clone()))
                .take(limits.max_context_items),
        );
        let catalog_truncated = selected.len() > 256;
        selected.truncate(256);
        let descriptor_refs = selected;
        let descriptors = self
            .store
            .query_descriptors(query.subject, &descriptor_refs)
            .await?;
        let mut representation = build_query_representation(
            query,
            &descriptors,
            work_context,
            representation_sources,
            &config_snapshot.get(QUERY_REPRESENTATION)?,
        );
        if current_count > limits.max_context_items {
            representation
                .truncation_flags
                .push("current_descriptor_catalog".into());
        }
        if catalog_truncated {
            representation
                .truncation_flags
                .push("descriptor_catalog".into());
        }
        for reference in &descriptor_refs {
            if !descriptors
                .iter()
                .any(|descriptor| &descriptor.reference == reference)
            {
                representation
                    .truncation_flags
                    .push(format!("missing_descriptor:{reference}"));
            }
        }
        representation.truncation_flags.sort();
        representation.truncation_flags.dedup();
        Ok(representation)
    }

    pub(super) async fn query_context(
        &self,
        query: &mut CognitiveQuery,
    ) -> Result<(Option<WorkContextView>, Vec<(CognitiveRef, String)>)> {
        let mut sources = query
            .situation
            .current_refs
            .iter()
            .cloned()
            .map(|reference| (reference, "caller_situation".into()))
            .collect::<Vec<_>>();
        let session = if let Some(id) = query.session {
            Some(self.session(query.subject, id).await?)
        } else {
            None
        };
        let context_id = query.work_context.or_else(|| {
            session
                .as_ref()
                .and_then(|session| session.active_work_context_id)
        });
        let context = if let Some(id) = context_id {
            let context = self.work_context(query.subject, id).await?;
            if context.state != WorkContextState::Open {
                return Err(Error::FailedPrecondition("STALE_CONTEXT".into()));
            }
            sources.extend(
                context
                    .references
                    .iter()
                    .cloned()
                    .map(|reference| (reference, "active_work_context".into())),
            );
            query
                .situation
                .current_refs
                .extend(context.references.clone());
            query.work_context = Some(id);
            Some(context)
        } else {
            None
        };
        if let Some(mut session) = session {
            session.resident.sort_by(|a, b| {
                b.last_meaningful_use_at
                    .cmp(&a.last_meaningful_use_at)
                    .then_with(|| b.entered_at.cmp(&a.entered_at))
                    .then_with(|| a.reference.to_string().cmp(&b.reference.to_string()))
            });
            sources.extend(
                session
                    .resident
                    .iter()
                    .map(|resident| (resident.reference.clone(), "runtime_resident".into())),
            );
            query.situation.current_refs.extend(
                session
                    .resident
                    .into_iter()
                    .map(|resident| resident.reference),
            );
        }
        query
            .situation
            .current_refs
            .sort_by_key(ToString::to_string);
        query.situation.current_refs.dedup();
        Ok((context, sources))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> CognitiveQuery {
        CognitiveQuery {
            api_version: API_VERSION,
            subject: SubjectId::new(),
            text_only_compatibility: false,
            work_context: None,
            session: None,
            situation: Default::default(),
            expression: CognitiveQueryExpr {
                cues: vec![Cue::Text(TextCue {
                    text: "Alice develops Nous Wave".into(),
                })],
                ..Default::default()
            },
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        }
    }
    #[test]
    fn allocation_preserves_context_priority_and_fixed_section_order() {
        let mut builder = Builder {
            sections: Vec::new(),
            remaining: 128,
            flags: BTreeSet::new(),
        };
        builder.section("Intent", "Alice reviews deployment readiness", 2048);
        builder.section("Resources", &"r".repeat(128), 512);
        builder.section("Current work", "Review Tide approval", 1024);
        let text = builder.render();
        assert!(text.contains("Review Tide approval"));
        assert!(builder.flags.contains("Resources"));
        assert!(text.find("Resources").unwrap() < text.find("Current work").unwrap());
        assert!(text.chars().count() <= 128);
        assert!(!text.contains("r".repeat(128).as_str()));
    }

    #[test]
    fn representation_is_semantic_deterministic_bounded_and_text_track_is_raw() {
        let mut query = query();
        let a = QueryDescriptor {
            reference: CognitiveRef::Tag(TagId::new()),
            text: "Nous Wave: cognition project".into(),
        };
        let b = QueryDescriptor {
            reference: CognitiveRef::Entity(EntityRef::new("entity:opaque").unwrap()),
            text: "Alice (A)".into(),
        };
        let limits = QueryRepresentationLimits::default();
        let first =
            build_query_representation(&query, &[a.clone(), b.clone()], None, vec![], &limits);
        let reversed = build_query_representation(&query, &[b, a], None, vec![], &limits);
        assert_eq!(first.text, reversed.text);
        assert_eq!(first.sha256, reversed.sha256);
        assert!(first.text.contains("Alice (A)"));
        assert!(!first.text.contains("entity:opaque"));
        query
            .situation
            .object_descriptions
            .insert("object:opaque".into(), "Nous Wave issue triage".into());
        let changed = build_query_representation(&query, &[], None, vec![], &limits);
        assert!(changed.text.contains("Nous Wave issue triage"));
        assert!(!changed.text.contains("object:opaque"));
        assert_ne!(changed.sha256, first.sha256);
        query.expression.cues = vec![Cue::Text(TextCue {
            text: "a".repeat(10000),
        })];
        let bounded = build_query_representation(&query, &[], None, vec![], &limits);
        assert!(bounded.text.chars().count() <= limits.total_chars);
        assert!(bounded.truncation_flags.contains(&"Intent".into()));
        query.text_only_compatibility = true;
        query.expression.cues = vec![Cue::Text(TextCue {
            text: "What did I discuss yesterday?".into(),
        })];
        let raw = build_query_representation(&query, &[], None, vec![], &limits);
        assert_eq!(raw.text, "What did I discuss yesterday?");
        assert_eq!(raw.representation_version, "text-compatibility-v1");
    }
}
