mod test_support;
use nous_core::*;
use nous_subject::{CognitiveSeedInput, CreateSubject};
use test_support::*;

fn query(subject: SubjectId) -> CognitiveQuery {
    CognitiveQuery {
        api_version: API_VERSION,
        subject,
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
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one owner scenario preserves WorkContext mutation and query snapshot ordering"
)]
async fn preparation_captures_work_context_exact_sources_and_frozen_policy_without_serving() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = runtime
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
    let entity = EntityRef::new("entity:alice-opaque").unwrap();
    runtime
        .store
        .bind_identity(
            subject,
            CognitiveRef::Entity(entity.clone()),
            "Alice".into(),
            vec!["A".into()],
        )
        .await
        .unwrap();
    let observed = observation(&runtime, subject, "Alice plans the Nous Wave release").await;
    let mut input = form_input(
        subject,
        observed.occurrence.occurrence_id,
        OperationId::new(),
        "Alice plans the Nous Wave release",
    );
    input.aboutness = vec![entity.clone()];
    let memory = runtime
        .require_memory()
        .unwrap()
        .form_memory(input)
        .await
        .unwrap();
    let reference = CognitiveRef::MemoryRevision(memory.revision.memory_revision_id);
    let session = runtime
        .cognition
        .open_session(
            subject,
            serde_json::json!({"transcript":"UNRELATED HISTORY MUST NOT BE EMBEDDED"}),
        )
        .await
        .unwrap();
    let context = runtime
        .cognition
        .create_work_context(nous_runtime::CreateWorkContextInput {
            operation_id: OperationId::new(),
            subject,
            purpose: "Nous Wave release preparation".into(),
            unresolved_questions: vec!["Alice's pending release tasks".into()],
            constraints: serde_json::json!({}),
            resume_conditions: vec![],
            budget_summary: serde_json::json!({}),
            references: vec![reference.clone()],
        })
        .await
        .unwrap();
    runtime
        .cognition
        .set_active_work_context(
            subject,
            session.session_id,
            Some(context.work_context_id),
            session.runtime_revision,
            OperationId::new(),
        )
        .await
        .unwrap();
    let mut request = query(subject);
    request.session = Some(session.session_id);
    request
        .expression
        .cues
        .push(Cue::Entity(EntityCue { entity_ref: entity }));
    let first = runtime.cognition.bind_query(request.clone()).await.unwrap();
    assert!(
        first
            .representation
            .text
            .contains("Nous Wave release preparation")
    );
    assert!(first.representation.text.contains("Alice (A)"));
    assert!(
        first
            .representation
            .text
            .contains("Alice plans the Nous Wave release")
    );
    assert!(!first.representation.text.contains("UNRELATED HISTORY"));
    assert!(!first.representation.text.contains("entity:alice-opaque"));
    assert!(first.runtime_refs.contains(&reference));
    assert!(
        first
            .representation
            .source_refs
            .iter()
            .any(|(r, source)| r == &reference && source == "active_work_context")
    );
    assert!(
        runtime
            .store
            .serving_current(subject)
            .await
            .unwrap()
            .is_empty()
    );
    let selected = first
        .for_profile(nous_runtime::CognitiveProfile::VcpDtsc)
        .unwrap();
    assert_eq!(selected.representation.sha256, first.representation.sha256);
    assert_eq!(selected.runtime_refs, first.runtime_refs);
    assert_eq!(
        selected
            .config_snapshot
            .source(nous_runtime::COGNITIVE_PROFILE.path()),
        Some(nous_configuration::ConfigSource::OperationOverride)
    );
    assert_eq!(
        runtime
            .configuration
            .snapshot_for_subject(subject)
            .unwrap()
            .effective_digest,
        first.config_snapshot.effective_digest
    );
    assert!(
        first
            .config_snapshot
            .query_override(nous_runtime::QUERY_LEASE, 1)
            .is_err()
    );
    let token = runtime
        .cognition
        .retain_prepared_query(first.clone())
        .unwrap();
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_runtime::COGNITIVE_PROFILE.path(),
            serde_json::json!("baseline-rrf"),
        )
        .await
        .unwrap();
    let frozen = runtime
        .cognition
        .take_prepared_query(subject, token)
        .unwrap();
    assert_eq!(
        frozen.config_snapshot.effective_digest,
        first.config_snapshot.effective_digest
    );
    assert_eq!(
        frozen.retrieval_policy.cognitive_profile,
        first.retrieval_policy.cognitive_profile
    );
    assert!(
        runtime
            .cognition
            .take_prepared_query(subject, token)
            .is_err()
    );
    runtime
        .cognition
        .update_work_context(nous_runtime::UpdateWorkContextInput {
            operation_id: OperationId::new(),
            subject,
            work_context_id: context.work_context_id,
            expected_revision: context.revision,
            purpose: "Nous Wave release stabilization".into(),
            unresolved_questions: context.unresolved_questions,
            constraints: context.constraints,
            resume_conditions: context.resume_conditions,
            budget_summary: context.budget_summary,
            references: context.references,
        })
        .await
        .unwrap();
    let changed = runtime.cognition.bind_query(request).await.unwrap();
    assert_ne!(changed.representation.sha256, first.representation.sha256);
    let mut runtime_query = query(subject);
    runtime_query.session = Some(session.session_id);
    let frozen_runtime = runtime.cognition.bind_query(runtime_query).await.unwrap();
    let newer_source = observation(&runtime, subject, "Bob reports an unrelated task").await;
    let newer = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            newer_source.occurrence.occurrence_id,
            OperationId::new(),
            "Bob reports an unrelated task",
        ))
        .await
        .unwrap();
    let newer_ref = CognitiveRef::MemoryRevision(newer.revision.memory_revision_id);
    runtime
        .cognition
        .use_feedback(nous_runtime::UseFeedback {
            subject,
            session_id: Some(session.session_id),
            consumer_ref: "consumer:test:prepared-query".into(),
            events: vec![nous_runtime::UseFeedbackEvent {
                event_id: UseEventId::new(),
                reference: newer_ref.clone(),
                use_kind: nous_runtime::UseKind::Referenced,
                occurred_at: chrono::Utc::now(),
                context: serde_json::json!({}),
            }],
        })
        .await
        .unwrap();
    let frozen_results = runtime
        .execute_bound_query(frozen_runtime, None)
        .await
        .unwrap()
        .result;
    assert!(
        frozen_results
            .results
            .iter()
            .any(|hit| hit.reference == reference)
    );
    assert!(
        !frozen_results
            .results
            .iter()
            .any(|hit| hit.reference == newer_ref)
    );
    let mut raw = query(subject);
    raw.expression.cues = vec![Cue::Text(TextCue {
        text: "What did I discuss yesterday?".into(),
    })];
    assert!(
        runtime
            .cognition
            .bind_query(raw.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("UNRESOLVED_QUERY_REFERENCE")
    );
    raw.text_only_compatibility = true;
    assert_eq!(
        runtime
            .cognition
            .bind_query(raw.clone())
            .await
            .unwrap()
            .representation
            .text,
        "What did I discuss yesterday?"
    );
    raw.session = Some(session.session_id);
    assert!(runtime.cognition.bind_query(raw).await.is_err());
}
