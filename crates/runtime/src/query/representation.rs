// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use crate::{CognitiveRuntimeService, SessionView, WorkContextState, WorkContextView};
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
/// Query-local frozen input, never a second durable context authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContextSnapshot {
    pub work_context: Option<WorkContextView>,
    pub session: Option<SessionView>,
    pub situation_refs: Vec<CognitiveRef>,
    pub source_refs: Vec<(CognitiveRef, String)>,
    pub history_exclusions: Vec<Degradation>,
    pub digest: String,
}
impl QueryContextSnapshot {
    pub(super) fn freeze(&mut self) -> Result<()> {
        self.digest.clear();
        let bytes = serde_json::to_vec(self).map_err(|e| Error::Internal(e.to_string()))?;
        self.digest = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Ok(())
    }
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
    sections: Vec<(&'static str, String, u8)>,
    remaining: usize,
    flags: BTreeSet<String>,
}
impl Builder {
    fn section(&mut self, label: &'static str, value: &str, limit: usize) {
        let priority = match label {
            "Intent" => 0,
            "Temporal orientation" | "Semantic concepts" => 1,
            "Current work" => 3,
            _ => 5,
        };
        self.prioritized_section(label, value, limit, priority);
    }
    fn prioritized_section(
        &mut self,
        label: &'static str,
        value: &str,
        limit: usize,
        priority: u8,
    ) {
        if value.trim().is_empty() {
            return;
        }
        if value.trim().chars().count() > limit {
            self.flags.insert(label.into());
        }
        self.sections
            .push((label, value.trim().chars().take(limit).collect(), priority));
    }
    fn render(&mut self) -> String {
        // Allocation priority is independent of the fixed semantic section order.
        let mut indices = (0..self.sections.len()).collect::<Vec<_>>();
        indices.sort_by_key(|i| self.sections[*i].2);
        for index in indices {
            let (label, value, priority) = &mut self.sections[index];
            let header = label.chars().count() + 4; // label, colon, newline and section separation
            let available = self.remaining.saturating_sub(header);
            if value.chars().count() > available {
                self.flags.insert((*label).into());
                self.flags.insert(format!(
                    "{label}:{}",
                    match priority {
                        1 => "explicit_query",
                        2 => "work_context_anchor",
                        3 => "current_work",
                        4 => "work_context_cognition",
                        _ => "runtime_background",
                    }
                ));
                *value = value.chars().take(available).collect();
            }
            if !value.is_empty() {
                self.remaining -= header + value.chars().count();
            }
        }
        let mut rendered: Vec<(&str, String)> = Vec::new();
        for (label, value, _) in &self.sections {
            if value.is_empty() {
                continue;
            }
            if let Some((_, text)) = rendered.iter_mut().find(|(name, _)| name == label) {
                text.push('\n');
                text.push_str(value);
            } else {
                rendered.push((label, value.clone()));
            }
        }
        rendered
            .into_iter()
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

fn descriptor_priority(
    reference: &CognitiveRef,
    sources: &[(CognitiveRef, String)],
    semantic_anchor: bool,
) -> u8 {
    sources
        .iter()
        .filter(|(candidate, _)| candidate == reference)
        .map(|(_, source)| match source.as_str() {
            explicit if explicit.starts_with("explicit") => 1,
            "work_context" if semantic_anchor => 2,
            "work_context" => 4,
            _ => 5,
        })
        .min()
        .unwrap_or(5)
}

pub fn build_query_representation(
    query: &CognitiveQuery,
    descriptors: &[QueryDescriptor],
    context: Option<WorkContextView>,
    sources: Vec<(CognitiveRef, String)>,
    limits: &QueryRepresentationLimits,
) -> QueryRepresentation {
    let raw = intent(&query.expression);
    let mut builder = Builder {
        sections: Vec::new(),
        remaining: limits.total_chars,
        flags: BTreeSet::new(),
    };
    builder.section("Intent", &raw, limits.total_chars);
    let semantic_concepts = query
        .scopes()
        .into_iter()
        .flat_map(|node| &node.cues)
        .filter_map(|cue| {
            if let Cue::Concept(concept) = cue {
                Some(concept.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("; ");
    let temporal = query.scopes().iter().filter_map(|scope| {
        let c=&scope.constraints;
        let recent=scope.preferences.iter().filter_map(|p| match p.operand { PreferenceOperand::Recent(axis) => Some(format!("{}recent({axis:?})",if p.negative { "avoid " } else { "prefer " })), _ => None }).collect::<Vec<_>>();
        if [c.occurred,c.observed,c.valid,c.formed,c.recorded].iter().all(Option::is_none) && recent.is_empty() { return None; }
        Some(serde_json::json!({"occurred":c.occurred,"observed":c.observed,"valid":c.valid,"formed":c.formed,"recorded":c.recorded,"preferences":recent}).to_string())
    }).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>().join("\n");
    let temporal = match (query.temporal_frame.authority_view, query.temporal_frame.revision_view) {
        (AuthorityView::Current, RevisionView::Current) => temporal,
        _ => format!("{}\n{}", serde_json::json!({"authority_view":query.temporal_frame.authority_view,"revision_view":query.temporal_frame.revision_view}), temporal).trim().to_owned(),
    };
    builder.section("Temporal orientation", &temporal, 1024);
    for (label, select, maximum, chars) in [
        ("Entities", "entity", limits.max_context_items, 256),
        ("Concepts", "tag", limits.max_context_items, 4608),
        ("Schemas", "cognitive_schema", limits.max_context_items, 512),
        (
            "Current cognition",
            "current",
            limits.max_context_items,
            512,
        ),
        ("Resources", "resource", limits.max_context_items, 512),
    ] {
        let mut values = descriptors
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
            .map(|d| {
                let priority =
                    descriptor_priority(&d.reference, &sources, matches!(select, "entity" | "tag"));
                (d.text.trim().to_owned(), priority)
            })
            .filter(|(text, _)| !text.is_empty())
            .collect::<Vec<_>>();
        values.sort_by_key(|(_, priority)| *priority);
        let mut seen = HashSet::new();
        values.retain(|(text, _)| seen.insert(text.clone()));
        if values.len() > maximum {
            builder.flags.insert(label.into());
        }
        for (text, priority) in values.into_iter().take(maximum) {
            if text.chars().count() > chars {
                builder.flags.insert(label.into());
            }
            builder.prioritized_section(
                label,
                &format!("- {}", text.chars().take(chars).collect::<String>()),
                limits.total_chars,
                priority,
            );
        }
    }
    builder.section("Semantic concepts", &semantic_concepts, 2048);
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
                "{}\n{}\n{}",
                context.purpose,
                context.unresolved_questions.join("\n"),
                context.context_text
            ),
            limits.total_chars,
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
        historical: Option<&HistoricalAuthoritySnapshot>,
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
        if intent(&query.expression).chars().count() + "Intent:\n".len() + 2 > limits.total_chars {
            return Err(Error::Invalid(
                "query intent exceeds configured representation bound".into(),
            ));
        }
        let mut selected = representation_sources
            .iter()
            .filter(|(_, source)| source.starts_with("explicit") || source == "work_context")
            .map(|(reference, _)| reference.clone())
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        selected.retain(|reference| seen.insert(reference.clone()));
        let current_count = representation_sources
            .iter()
            .filter(|(_, source)| !source.starts_with("explicit") && source != "work_context")
            .map(|(reference, _)| reference.clone())
            .collect::<HashSet<_>>()
            .len();
        selected.extend(
            representation_sources
                .iter()
                .filter(|(_, source)| !source.starts_with("explicit") && source != "work_context")
                .map(|(reference, _)| reference.clone())
                .filter(|reference| seen.insert(reference.clone()))
                .take(limits.max_context_items),
        );
        let catalog_truncated = selected.len() > 256;
        selected.truncate(256);
        let descriptor_refs = selected;
        let mut descriptors = self
            .store
            .query_descriptors_in_view(query.subject, &descriptor_refs, historical)
            .await?;
        descriptors.sort_by_key(|descriptor| {
            descriptor_refs
                .iter()
                .position(|reference| reference == &descriptor.reference)
                .unwrap_or(usize::MAX)
        });
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

    pub(super) async fn prepare_query_context(
        &self,
        query: &mut CognitiveQuery,
        view: Option<&nous_core::HistoricalAuthoritySnapshot>,
    ) -> Result<QueryContextSnapshot> {
        let mut context_snapshot = self.query_context(query).await?;
        let mut work_context = context_snapshot.work_context.clone();
        let mut representation_sources = context_snapshot.source_refs.clone();
        let context_degradation = super::historical_context::fence(
            query,
            &mut work_context,
            &mut representation_sources,
            view,
        );
        context_snapshot.work_context = work_context.clone();
        context_snapshot.history_exclusions = context_degradation.clone();
        context_snapshot.source_refs = representation_sources.clone();
        let admitted = representation_sources
            .iter()
            .map(|(r, _)| r)
            .collect::<HashSet<_>>();
        context_snapshot
            .situation_refs
            .retain(|r| admitted.contains(r));
        if let Some(session) = &mut context_snapshot.session {
            session.resident.retain(|r| admitted.contains(&r.reference));
        }
        context_snapshot.freeze()?;
        Ok(context_snapshot)
    }
    pub(super) async fn query_context(
        &self,
        query: &mut CognitiveQuery,
    ) -> Result<QueryContextSnapshot> {
        let mut tx = self.store.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(nous_persistence::database_error)?;
        let situation_refs = query.situation.current_refs.clone();
        let mut sources = query
            .situation
            .current_refs
            .iter()
            .cloned()
            .map(|reference| (reference, "runtime_situation".into()))
            .collect::<Vec<_>>();
        let session = if let Some(id) = query.session {
            Some(self.session_in(query.subject, id, &mut tx).await?)
        } else {
            None
        };
        if session.as_ref().is_some_and(|s| s.closed_at.is_some()) {
            return Err(Error::FailedPrecondition("STALE_CONTEXT".into()));
        }
        let context_id = query.work_context.or_else(|| {
            session
                .as_ref()
                .and_then(|session| session.active_work_context_id)
        });
        let context = if let Some(id) = context_id {
            let context = self.work_context_in(query.subject, id, &mut tx).await?;
            if context.state != WorkContextState::Open {
                return Err(Error::FailedPrecondition("STALE_CONTEXT".into()));
            }
            sources.extend(
                context
                    .cognition_anchors
                    .iter()
                    .cloned()
                    .map(|reference| (reference, "work_context".into())),
            );
            sources.extend(
                context
                    .entity_anchors
                    .iter()
                    .cloned()
                    .map(|entity| (CognitiveRef::Entity(entity), "work_context".into())),
            );
            sources.extend(
                context
                    .tag_anchors
                    .iter()
                    .copied()
                    .map(|tag| (CognitiveRef::Tag(tag), "work_context".into())),
            );
            query
                .situation
                .current_refs
                .extend(context.cognition_anchors.clone());
            query.work_context = Some(id);
            Some(context)
        } else {
            None
        };
        let mut session = session;
        if let Some(session) = &mut session {
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
                    .iter()
                    .map(|resident| resident.reference.clone()),
            );
        }
        query
            .situation
            .current_refs
            .sort_by_key(ToString::to_string);
        query.situation.current_refs.dedup();
        tx.commit()
            .await
            .map_err(nous_persistence::database_error)?;
        Ok(QueryContextSnapshot {
            work_context: context,
            session,
            situation_refs,
            source_refs: sources,
            history_exclusions: vec![],
            digest: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> CognitiveQuery {
        CognitiveQuery {
            subject: SubjectId::new(),
            projection: Default::default(),
            temporal_frame: Default::default(),

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
    fn explicit_and_pinned_descriptors_precede_work_and_resident_background() {
        let query = query();
        let tag = CognitiveRef::Tag(TagId::new());
        let exact = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let mut descriptors = vec![
            QueryDescriptor {
                reference: tag.clone(),
                text: "Pinned blogging practice".into(),
            },
            QueryDescriptor {
                reference: exact.clone(),
                text: "Explicit selected revision".into(),
            },
        ];
        let mut sources = vec![
            (tag, "work_context".into()),
            (exact, "explicit_exact".into()),
        ];
        for index in 0..16 {
            let reference = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
            descriptors.push(QueryDescriptor {
                reference: reference.clone(),
                text: format!("Resident {index}: {}", "background ".repeat(60)),
            });
            sources.push((reference, "runtime_resident".into()));
        }
        let now = chrono::Utc::now();
        let context = WorkContextView {
            work_context_id: uuid::Uuid::new_v4(),
            subject_id: query.subject,
            state: WorkContextState::Open,
            purpose: "Investigate blogging".into(),
            context_text: format!(
                "Simon Willison link blog beats task context {}",
                "task background ".repeat(600)
            ),
            unresolved_questions: vec!["Why did the practice change?".into()],
            constraints: serde_json::json!({}),
            resume_conditions: vec![],
            budget_summary: serde_json::json!({}),
            revision: 1,
            created_at: now,
            updated_at: now,
            ended_at: None,
            cognition_anchors: vec![],
            entity_anchors: vec![],
            tag_anchors: vec![],
        };
        let representation = build_query_representation(
            &query,
            &descriptors,
            Some(context),
            sources,
            &QueryRepresentationLimits::default(),
        );
        assert!(representation.text.contains("Alice develops Nous Wave"));
        assert!(representation.text.contains("Pinned blogging practice"));
        assert!(representation.text.contains("Explicit selected revision"));
        assert!(
            representation
                .text
                .contains("Simon Willison link blog beats task context")
        );
        assert!(representation.text.contains("Why did the practice change?"));
        assert!(
            representation
                .truncation_flags
                .contains(&"Current cognition:runtime_background".into())
        );
        assert_eq!(representation.text.matches("Current cognition:").count(), 1);
        assert!(representation.text.chars().count() <= 8192);
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
    fn representation_is_semantic_deterministic_and_bounded() {
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
        query.expression.cues = vec![Cue::Text(TextCue {
            text: "What did I discuss yesterday?".into(),
        })];
        let raw = build_query_representation(&query, &[], None, vec![], &limits);
        assert_eq!(
            raw.text,
            "Intent:\nWhat did I discuss yesterday?\n\nCurrent objects:\nNous Wave issue triage"
        );
        assert_eq!(raw.representation_version, "cognitive-query-v2");
    }
}
