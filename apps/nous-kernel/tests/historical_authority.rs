// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
mod test_support;
use nous_core::*;
use nous_memory::{AssociationPolarity, AssociationSupport, AssociationSupportClass};
use nous_memory::{CreateTagRequest, ReviseTagInput, TagContent, TagExpectation};
use nous_runtime::{CognitiveRuntimeService, ManualCognitiveClock};
use nous_subject::{CognitiveSeedInput, CreateSubject};
use std::sync::Arc;
use test_support::*;
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one historical owner timeline verifies revisions, lifecycle, concepts, aliases and state digest independence from requested timestamps"
)]
async fn owner_projection_uses_recorded_revisions_and_past_concept_state() {
    let (root, url, _pg) = database().await;
    let mut rt = open_runtime(&url, &root).await;
    let subject = rt
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: None,
        })
        .await
        .unwrap()
        .subject_id;
    let start = chrono::Utc::now();
    let clock = Arc::new(ManualCognitiveClock::new(start));
    rt.cognition = CognitiveRuntimeService::with_clock(
        rt.store.clone(),
        rt.cognition.resident_limit,
        rt.configuration.clone(),
        clock.clone(),
    )
    .unwrap();
    rt.memory.as_mut().unwrap().cognition = rt.cognition.clone();
    rt.material.cognition = rt.cognition.clone();
    let source = observation(&rt, subject, "original authority fact").await;
    let owner = rt.require_memory().unwrap();
    let tag = owner
        .create_tag(
            subject,
            CreateTagRequest {
                operation_id: OperationId::new(),
                label: "Past concept".into(),
                description: Some("PAST_DESCRIPTION reader ownership".into()),
                kind_hint: None,
                origin: "host_explicit".into(),
                producer: None,
            },
        )
        .await
        .unwrap();
    let mut input = form_input(
        subject,
        source.occurrence.occurrence_id,
        OperationId::new(),
        "original authority fact",
    );
    input.tags = vec![tag.tag_id];
    let supports = input.supports.clone();
    let memory = owner.form_memory(input).await.unwrap();
    let old = CognitiveRef::MemoryRevision(memory.revision.memory_revision_id);
    let exact_boundary = owner
        .project_as_of(subject, memory.revision.recorded_at, RevisionView::Current)
        .await
        .unwrap();
    assert_eq!(exact_boundary.cognition[0].head, old);
    assert_eq!(exact_boundary.tags[0].tag, tag.tag_id);
    let before = start + chrono::Duration::seconds(1);
    let same = start + chrono::Duration::seconds(2);
    let baseline = owner
        .project_as_of(subject, before, RevisionView::Current)
        .await
        .unwrap();
    let equivalent = owner
        .project_as_of(subject, same, RevisionView::Current)
        .await
        .unwrap();
    assert_eq!(baseline.snapshot_digest, equivalent.snapshot_digest);
    assert_eq!(baseline.cognition.len(), 1);
    assert_eq!(baseline.cognition[0].head, old);
    assert_eq!(baseline.canonical_tag(tag.tag_id), Some(tag.tag_id));
    assert!(baseline.tags[0].semantic.text.contains("PAST_DESCRIPTION"));
    assert!(
        baseline
            .lexical_visibility
            .iter()
            .any(|binding| binding.object_kind == "tag" && binding.display_name == "Past concept")
    );
    clock
        .advance_by(subject, chrono::Duration::seconds(10))
        .unwrap();
    let revised = owner
        .revise_memory(nous_memory::ReviseMemoryInput {
            producer: None,
            operation_id: OperationId::new(),
            subject,
            memory_id: memory.object.memory_id,
            expected_object_epoch: memory.object.object_epoch,
            intent: nous_memory::RevisionIntent::Rephrase,
            formation_mode: nous_memory::FormationMode::Grounded,
            grounding_occurrence_id: Some(source.occurrence.occurrence_id),
            semantic_role: "fact".into(),
            representation_text: "rephrased authority fact".into(),
            title: None,
            supports: supports.clone(),
            aboutness: vec![],
            valid_time: TemporalExtent::Unknown,
            epistemic_class: EpistemicClass::Observed,
        })
        .await
        .unwrap();
    let new = CognitiveRef::MemoryRevision(revised.revision.memory_revision_id);
    let tag_revised = owner
        .revise_tag(
            subject,
            ReviseTagInput {
                operation_id: OperationId::new(),
                target: TagExpectation {
                    tag_id: tag.tag_id,
                    expected_revision_id: tag.current_revision_id,
                },
                content: TagContent {
                    label: "Future concept".into(),
                    description: Some("FUTURE_DESCRIPTION different reader meaning".into()),
                    kind_hint: None,
                },
                producer: None,
            },
        )
        .await
        .unwrap();
    let past = owner
        .project_as_of(subject, before, RevisionView::Current)
        .await
        .unwrap();
    assert_eq!(past.snapshot_digest, baseline.snapshot_digest);
    assert_eq!(past.cognition[0].head, old);
    assert!(!past.contains(&new));
    assert!(past.tags[0].semantic.text.contains("PAST_DESCRIPTION"));
    assert!(!past.tags[0].semantic.text.contains("FUTURE_DESCRIPTION"));
    assert!(
        past.lexical_visibility
            .iter()
            .all(|binding| binding.display_name != "Future concept")
    );
    let later = start + chrono::Duration::seconds(11);
    let current = owner
        .project_as_of(subject, later, RevisionView::Current)
        .await
        .unwrap();
    assert_eq!(current.cognition[0].head, new);
    assert_eq!(current.cognition[0].revisions, vec![new.clone()]);
    assert!(current.contains(&old)); // Exact historical refs remain available without becoming ordinary current candidates.
    assert_eq!(current.tags[0].revision_id, tag_revised.current_revision_id);
    let history = owner
        .project_as_of(subject, later, RevisionView::History)
        .await
        .unwrap();
    assert_eq!(
        history.cognition[0].revisions,
        vec![new.clone(), old.clone()]
    );
    assert_ne!(history.snapshot_digest, current.snapshot_digest);
    clock
        .advance_to(subject, start + chrono::Duration::seconds(15))
        .unwrap();
    let association = owner
        .create_association(
            nous_memory::CreateAssociationRequest {
                producer: None,
                operation_id: OperationId::new(),
                from: CognitiveRef::MemoryRevision(revised.revision.memory_revision_id),
                to: CognitiveRef::Tag(tag.tag_id),
                relation_kind: "tag_attachment".into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::HostExplicit,
                supports: supports
                    .iter()
                    .cloned()
                    .map(AssociationSupport::Revision)
                    .collect(),
                producer_signature_id: None,
                valid_time: TemporalExtent::Unknown,
            },
            subject,
        )
        .await
        .unwrap();
    assert!(
        owner
            .project_as_of(subject, later, RevisionView::Current)
            .await
            .unwrap()
            .associations
            .is_empty()
    );
    assert!(
        owner
            .project_as_of(
                subject,
                start + chrono::Duration::seconds(16),
                RevisionView::Current
            )
            .await
            .unwrap()
            .associations
            .contains(&association.association_evidence_id)
    );
    clock
        .advance_to(subject, start + chrono::Duration::seconds(20))
        .unwrap();
    owner
        .suppress(
            subject,
            memory.object.memory_id,
            OperationId::new(),
            revised.object.object_epoch,
        )
        .await
        .unwrap();
    assert_eq!(
        owner
            .project_as_of(subject, before, RevisionView::Current)
            .await
            .unwrap()
            .cognition[0]
            .state["suppression_state"],
        "normal"
    );
    assert_eq!(
        owner
            .project_as_of(
                subject,
                start + chrono::Duration::seconds(21),
                RevisionView::Current
            )
            .await
            .unwrap()
            .cognition[0]
            .state["suppression_state"],
        "suppressed"
    );
    let journal_text:serde_json::Value=sqlx::query_scalar("SELECT state FROM authority_object_states WHERE subject_id=$1 AND object_kind='memory' ORDER BY event_id DESC LIMIT 1").bind(subject.0).fetch_one(rt.store.pool()).await.unwrap();
    assert!(journal_text.get("representation_text").is_none());
    clock
        .advance_to(subject, start + chrono::Duration::seconds(25))
        .unwrap();
    owner
        .revoke_association(
            subject,
            association.association_evidence_id,
            OperationId::new(),
        )
        .await
        .unwrap();
    assert!(
        owner
            .project_as_of(
                subject,
                start + chrono::Duration::seconds(16),
                RevisionView::Current
            )
            .await
            .unwrap()
            .associations
            .contains(&association.association_evidence_id)
    );
    assert!(
        !owner
            .project_as_of(
                subject,
                start + chrono::Duration::seconds(26),
                RevisionView::Current
            )
            .await
            .unwrap()
            .associations
            .contains(&association.association_evidence_id)
    );
    let survivor = owner
        .create_tag(
            subject,
            CreateTagRequest {
                operation_id: OperationId::new(),
                label: "New survivor concept".into(),
                description: None,
                kind_hint: None,
                origin: "host_explicit".into(),
                producer: None,
            },
        )
        .await
        .unwrap();
    clock
        .advance_to(subject, start + chrono::Duration::seconds(30))
        .unwrap();
    owner
        .merge_tags(
            subject,
            nous_memory::MergeTagsInput {
                operation_id: OperationId::new(),
                survivor: TagExpectation {
                    tag_id: survivor.tag_id,
                    expected_revision_id: survivor.current_revision_id,
                },
                retired: vec![TagExpectation {
                    tag_id: tag.tag_id,
                    expected_revision_id: tag_revised.current_revision_id,
                }],
                supports,
            },
        )
        .await
        .unwrap();
    let old_view = owner
        .project_as_of(subject, before, RevisionView::Current)
        .await
        .unwrap();
    assert_eq!(old_view.snapshot_digest, baseline.snapshot_digest);
    assert_eq!(old_view.canonical_tag(tag.tag_id), Some(tag.tag_id));
    assert!(!old_view.contains(&CognitiveRef::Tag(survivor.tag_id)));
    assert_eq!(
        owner
            .project_as_of(
                subject,
                start + chrono::Duration::seconds(31),
                RevisionView::Current
            )
            .await
            .unwrap()
            .canonical_tag(tag.tag_id),
        Some(survivor.tag_id)
    );
    check_historical_binding(
        &rt,
        subject,
        &baseline,
        memory.object.memory_id,
        (old, new),
        (tag.tag_id, survivor.tag_id),
    )
    .await;
    owner
        .purge_memory(
            subject,
            memory.object.memory_id,
            OperationId::new(),
            revised.object.object_epoch + 1,
        )
        .await
        .unwrap();
    assert!(
        owner
            .project_as_of(subject, before, RevisionView::History)
            .await
            .unwrap()
            .cognition
            .is_empty()
    );
}

async fn check_historical_binding(
    rt: &nous_kernel::NousRuntime,
    subject: SubjectId,
    view: &HistoricalAuthoritySnapshot,
    memory: MemoryId,
    references: (CognitiveRef, CognitiveRef),
    tags: (TagId, TagId),
) {
    let (old, future) = references;
    let (tag, future_tag) = tags;
    let context = rt
        .cognition
        .create_work_context(nous_runtime::CreateWorkContextInput {
            operation_id: OperationId::new(),
            subject,
            purpose: "CURRENT_QUESTION PURPOSE about old knowledge".into(),
            unresolved_questions: vec![],
            constraints: serde_json::json!({}),
            resume_conditions: vec![],
            budget_summary: serde_json::json!({}),
            references: vec![old.clone(), future.clone()],
        })
        .await
        .unwrap();
    let query = CognitiveQuery {
        api_version: API_VERSION,
        subject,
        session: None,
        work_context: Some(context.work_context_id),
        projection: Default::default(),
        temporal_frame: TemporalFrame {
            authority_view: AuthorityView::AsOf(view.as_of),
            ..Default::default()
        },
        text_only_compatibility: false,
        situation: SituationDescriptor {
            current_refs: vec![CognitiveRef::Tag(future_tag)],
            ..Default::default()
        },
        expression: CognitiveQueryExpr {
            targets: vec![QueryTarget::Exact {
                reference: CognitiveRef::Memory(memory),
            }],
            cues: vec![
                Cue::Text(TextCue {
                    text: "what was known".into(),
                }),
                Cue::Tag(TagCue { tag }),
            ],
            ..Default::default()
        },
        exploration: Default::default(),
        resources: Default::default(),
        result_need: Default::default(),
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: Default::default(),
    };
    let config = rt.configuration.snapshot_for_subject(subject).unwrap();
    let bound = rt
        .cognition
        .bind_query_with_authority_view(query.clone(), config.clone(), Some(Arc::new(view.clone())))
        .await
        .unwrap();
    assert_eq!(bound.exact_bindings[0].bound_ref, old);
    assert!(bound.runtime_refs.contains(&old));
    assert!(!bound.runtime_refs.contains(&future));
    assert!(!bound.runtime_refs.contains(&CognitiveRef::Tag(future_tag)));
    assert!(bound.representation.text.contains("PAST_DESCRIPTION"));
    assert!(
        bound
            .representation
            .text
            .contains("original authority fact")
    );
    assert!(
        bound
            .representation
            .text
            .contains("CURRENT_QUESTION PURPOSE")
    );
    assert!(!bound.representation.text.contains("FUTURE_DESCRIPTION"));
    assert!(
        !bound
            .representation
            .text
            .contains("rephrased authority fact")
    );
    assert!(
        bound
            .activation
            .degradation
            .iter()
            .any(|degradation| degradation.code == "future_context_ref_excluded")
    );
    assert_eq!(bound.activation.explicit_tags[0].tag, tag);
    let composed = rt
        .bind_query_with_snapshot(query.clone(), config.clone())
        .await
        .unwrap();
    assert!(composed.historical_authority.is_some());
    assert_eq!(composed.exact_bindings[0].bound_ref, old);
    let mut future_query = query;
    future_query
        .expression
        .cues
        .push(Cue::Tag(TagCue { tag: future_tag }));
    assert!(matches!(
        rt.cognition
            .bind_query_with_authority_view(future_query, config, Some(Arc::new(view.clone())))
            .await,
        Err(Error::NotFound(_))
    ));
}
