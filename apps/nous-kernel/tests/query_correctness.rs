mod test_support;

use chrono::Duration;
use chrono::Utc;
use nous_cognitive_runtime::{QueryPlan, UseFeedback, UseFeedbackEvent, UseKind};
use nous_core::{
    CognitiveQuery, CognitiveRef, Cue, EntityRef, EpistemicClass, OperationId, QueryConstraints,
    QueryTarget, ResultNeed, TemporalExtent, TextCue, UseEventId,
};
use nous_memory_domain::{
    AssociationPolarity, AssociationSupport, AssociationSupportClass, CognitiveRole,
    CreateSchemaInput, EvidenceLocator, EvidenceRef, FormationMode, RevisionSupport,
    SchemaEvidenceLinkInput, SchemaEvidenceRole, SchemaFormationKind, SchemaScope, SupportRole,
    UseEventRef,
};
use nous_subject_core::{CognitiveSeedInput, CreateSubject};
use test_support::{database, form_input, observation, open_runtime, open_runtime_with_serving};
use uuid::Uuid;

fn query(subject: nous_core::SubjectId) -> CognitiveQuery {
    CognitiveQuery {
        api_version: nous_core::API_VERSION,
        subject,
        session: None,
        situation: Default::default(),
        targets: Vec::new(),
        cues: Vec::new(),
        constraints: QueryConstraints::default(),
        exploration: Default::default(),
        resources: Default::default(),
        result_need: ResultNeed {
            limit: 32,
            ..Default::default()
        },
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: Default::default(),
    }
}

async fn subject(runtime: &nous_kernel::NousRuntime) -> nous_core::SubjectId {
    runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: nous_core::OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: None,
        })
        .await
        .expect("subject")
        .subject_id
}

#[tokio::test]
async fn entity_lane_uses_aboutness_and_multi_value_include() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let alice = EntityRef::new("entity:alice").expect("entity");
    let first_observation = observation(&runtime, subject, "Alice declarative").await;
    let second_observation = observation(&runtime, subject, "Alice experiential").await;
    let mut first = form_input(
        subject,
        first_observation.occurrence.occurrence_id,
        OperationId::new(),
        "declarative about Alice",
    );
    first.aboutness = vec![alice.clone()];
    first.cognitive_role = CognitiveRole::Declarative;
    let mut second = form_input(
        subject,
        second_observation.occurrence.occurrence_id,
        OperationId::new(),
        "experiential about Alice",
    );
    second.aboutness = vec![alice.clone()];
    second.cognitive_role = CognitiveRole::Experiential;
    let first = runtime
        .require_memory()
        .unwrap()
        .form_memory(first)
        .await
        .expect("first memory");
    let second = runtime
        .require_memory()
        .unwrap()
        .form_memory(second)
        .await
        .expect("second memory");
    let mut request = query(subject);
    request.cues = vec![Cue::Entity(nous_core::EntityCue { entity_ref: alice })];
    request.constraints.cognitive_roles_include = vec!["declarative".into(), "experiential".into()];
    let result = runtime.query(request).await.expect("entity query");
    let references = result
        .results
        .into_iter()
        .map(|hit| hit.reference)
        .collect::<Vec<_>>();
    assert!(references.contains(&CognitiveRef::MemoryRevision(
        first.revision.memory_revision_id,
    )));
    assert!(references.contains(&CognitiveRef::MemoryRevision(
        second.revision.memory_revision_id,
    )));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "runtime regression covers isolation, coalescing, and retry side effects"
)]
async fn runtime_lane_is_session_resident_and_use_retry_has_zero_side_effect() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let first_observation = observation(&runtime, subject, "resident A").await;
    let second_observation = observation(&runtime, subject, "resident B").await;
    let first = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            first_observation.occurrence.occurrence_id,
            OperationId::new(),
            "resident A",
        ))
        .await
        .expect("first memory");
    let second = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            second_observation.occurrence.occurrence_id,
            OperationId::new(),
            "resident B",
        ))
        .await
        .expect("second memory");
    let session_a = runtime
        .cognition
        .open_session(subject, serde_json::json!({"name":"A"}))
        .await
        .expect("session A");
    let session_b = runtime
        .cognition
        .open_session(subject, serde_json::json!({"name":"B"}))
        .await
        .expect("session B");
    let event_a = UseFeedbackEvent {
        event_id: UseEventId::new(),
        reference: CognitiveRef::MemoryRevision(first.revision.memory_revision_id),
        use_kind: UseKind::Referenced,
        occurred_at: Utc::now(),
        context: serde_json::json!({"source":"query-correctness-test"}),
    };
    runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_a.session_id),
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![event_a.clone()],
        })
        .await
        .expect("resident A use");
    runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_b.session_id),
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![UseFeedbackEvent {
                event_id: UseEventId::new(),
                reference: CognitiveRef::MemoryRevision(second.revision.memory_revision_id),
                use_kind: UseKind::Referenced,
                occurred_at: Utc::now(),
                context: serde_json::json!({}),
            }],
        })
        .await
        .expect("resident B use");
    let mut request = query(subject);
    request.session = Some(session_a.session_id);
    request.targets = vec![QueryTarget::AnyRelevantCognition];
    let result = runtime.query(request).await.expect("runtime query");
    let references = result
        .results
        .into_iter()
        .map(|hit| hit.reference)
        .collect::<Vec<_>>();
    assert!(references.contains(&CognitiveRef::MemoryRevision(
        first.revision.memory_revision_id,
    )));
    assert!(!references.contains(&CognitiveRef::MemoryRevision(
        second.revision.memory_revision_id,
    )));

    let before = runtime
        .cognition
        .session(subject, session_a.session_id)
        .await
        .expect("session before")
        .runtime_revision;
    let batch_event = UseFeedbackEvent {
        event_id: UseEventId::new(),
        ..event_a.clone()
    };
    let accepted = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_a.session_id),
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![batch_event.clone(), batch_event.clone()],
        })
        .await
        .expect("same-batch coalesce");
    assert_eq!(accepted.0, 1);
    assert_eq!(accepted.1, 1);
    let after_batch = runtime
        .cognition
        .session(subject, session_a.session_id)
        .await
        .expect("session after batch")
        .runtime_revision;
    assert_eq!(after_batch, before + 1);
    let duplicate = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_a.session_id),
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![batch_event],
        })
        .await
        .expect("duplicate retry");
    assert_eq!(duplicate.0, 0);
    assert_eq!(duplicate.1, 1);
    assert_eq!(duplicate.2, Some(after_batch));
}

#[tokio::test]
async fn exact_mutable_binding_is_fenced_and_explicit_history_is_readable() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let observation = observation(&runtime, subject, "fenced fact").await;
    let memory = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            observation.occurrence.occurrence_id,
            OperationId::new(),
            "old fenced fact",
        ))
        .await
        .expect("memory");
    let mut request = query(subject);
    request.targets = vec![QueryTarget::Exact {
        reference: CognitiveRef::Memory(memory.object.memory_id),
    }];
    let bound = runtime.cognition.bind_query(request).await.expect("bind");
    let plan = QueryPlan::for_bound_query(&bound);
    let revision = nous_memory_service::ReviseMemoryInput {
        operation_id: OperationId::new(),
        subject,
        memory_id: memory.object.memory_id,
        expected_object_epoch: memory.object.object_epoch,
        intent: nous_memory_domain::RevisionIntent::Correct,
        formation_mode: FormationMode::Grounded,
        grounding_occurrence_id: Some(observation.occurrence.occurrence_id),
        semantic_role: "fact".into(),
        representation_text: "new fenced fact".into(),
        title: None,
        supports: vec![RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: observation.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        })],
        aboutness: Vec::new(),
        valid_time: Default::default(),
        formed_at: Utc::now(),
        epistemic_class: EpistemicClass::Observed,
    };
    let _current = runtime
        .require_memory()
        .unwrap()
        .revise_memory(revision.clone())
        .await
        .expect("revision");
    let memory_service = runtime.require_memory().unwrap();
    let result = runtime
        .cognition
        .query_with_plan(
            bound,
            nous_cognitive_runtime::CognitiveContributors {
                memory: Some(memory_service as &dyn nous_cognitive_runtime::CognitiveContributor),
                self_cognition: None,
                social: None,
            },
            plan,
        )
        .await
        .expect("stale exact query");
    assert!(result.results.is_empty());
    assert_eq!(
        result
            .diagnostics
            .as_ref()
            .and_then(|value| value.candidate_counts.get("drop_stale_exact_binding"))
            .copied(),
        Some(1)
    );
    let historical = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::Exact {
                reference: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
            }],
            cues: Vec::new(),
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: ResultNeed {
                limit: 1,
                ..Default::default()
            },
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("historical query");
    assert_eq!(
        historical.results[0].reference,
        CognitiveRef::MemoryRevision(memory.revision.memory_revision_id)
    );
}

#[tokio::test]
async fn synthesized_schema_requires_independent_known_roots() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let first = observation(&runtime, subject, "same root").await;
    let second = observation(&runtime, subject, "independent root").await;
    let link = |occurrence| SchemaEvidenceLinkInput {
        role: SchemaEvidenceRole::Support,
        support: RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: occurrence,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
    };
    let base = || SchemaScope {
        description: "cognitive retrieval schema".into(),
        aboutness: Vec::new(),
        tags: Vec::new(),
        valid_time: Default::default(),
    };
    let same_root = runtime
        .require_memory()
        .unwrap()
        .create_schema(CreateSchemaInput {
            operation_id: OperationId::new(),
            subject,
            title: None,
            structural_claim: "same root must reject".into(),
            applicability_scope: base(),
            boundary_definition: "none".into(),
            formed_at: Utc::now(),
            formation_kind: SchemaFormationKind::Synthesized,
            evidence_links: vec![
                link(first.occurrence.occurrence_id),
                link(first.occurrence.occurrence_id),
            ],
        })
        .await;
    assert!(matches!(same_root, Err(nous_core::Error::Invalid(_))));
    let independent = runtime
        .require_memory()
        .unwrap()
        .create_schema(CreateSchemaInput {
            operation_id: OperationId::new(),
            subject,
            title: None,
            structural_claim: "independent roots accept".into(),
            applicability_scope: base(),
            boundary_definition: "none".into(),
            formed_at: Utc::now(),
            formation_kind: SchemaFormationKind::Synthesized,
            evidence_links: vec![
                link(first.occurrence.occurrence_id),
                link(second.occurrence.occurrence_id),
            ],
        })
        .await;
    assert!(independent.is_ok());
}

#[tokio::test]
async fn lexical_lane_does_not_create_rank_from_substring() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = subject(&runtime).await;
    let observation = observation(&runtime, subject, "latency").await;
    runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            observation.occurrence.occurrence_id,
            OperationId::new(),
            "latency",
        ))
        .await
        .expect("memory");
    let mut request = query(subject);
    request.cues = vec![Cue::Text(TextCue { text: "lat".into() })];
    request.capabilities.text_embedding = nous_core::RequirementStrength::Forbidden;
    let result = runtime.query(request).await.expect("lexical query");
    assert!(result.results.is_empty());
}

#[tokio::test]
async fn lexical_provider_hit_survives_non_literal_case_difference() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = subject(&runtime).await;
    let observation = observation(&runtime, subject, "latency").await;
    runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            observation.occurrence.occurrence_id,
            OperationId::new(),
            "latency",
        ))
        .await
        .expect("memory");
    let mut request = query(subject);
    request.cues = vec![Cue::Text(TextCue {
        text: "LATENCY".into(),
    })];
    request.capabilities.text_embedding = nous_core::RequirementStrength::Forbidden;
    let result = runtime.query(request).await.expect("lexical query");
    assert!(
        result
            .results
            .iter()
            .any(|hit| hit.representation.as_deref() == Some("latency"))
    );
}

#[tokio::test]
async fn stale_lexical_generation_cannot_return_old_revision() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = subject(&runtime).await;
    let observation = observation(&runtime, subject, "old lexical phrase").await;
    let first = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            observation.occurrence.occurrence_id,
            OperationId::new(),
            "old lexical phrase",
        ))
        .await
        .expect("memory");
    runtime
        .serving
        .refresh(subject)
        .await
        .expect("serving refresh");
    runtime
        .require_memory()
        .unwrap()
        .revise_memory(nous_memory_service::ReviseMemoryInput {
            operation_id: OperationId::new(),
            subject,
            memory_id: first.object.memory_id,
            expected_object_epoch: first.object.object_epoch,
            intent: nous_memory_domain::RevisionIntent::Correct,
            formation_mode: FormationMode::Grounded,
            grounding_occurrence_id: Some(observation.occurrence.occurrence_id),
            semantic_role: "fact".into(),
            representation_text: "new lexical phrase".into(),
            title: None,
            supports: vec![RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: observation.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            aboutness: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Reported,
        })
        .await
        .expect("revision");
    let mut request = query(subject);
    request.cues = vec![Cue::Text(TextCue {
        text: "old lexical phrase".into(),
    })];
    request.capabilities.text_embedding = nous_core::RequirementStrength::Forbidden;
    let result = runtime.query(request).await.expect("lexical query");
    assert!(
        result
            .results
            .iter()
            .all(|hit| hit.representation.as_deref() != Some("old lexical phrase"))
    );
}

#[tokio::test]
async fn authority_lanes_reach_matches_beyond_first_n_objects() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let target_entity = "entity:tail-match";
    let temporal_start = Utc::now() - Duration::hours(1);
    let temporal_end = temporal_start + Duration::minutes(30);
    let mut tx = runtime.store.begin().await.expect("transaction");
    let mut tail_revision = None;
    for index in 0..10_000u32 {
        let memory_id = Uuid::now_v7();
        let revision_id = Uuid::now_v7();
        let valid = index == 9_999;
        sqlx::query("INSERT INTO memory_objects(memory_id,subject_id,cognitive_role,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,accessibility_mode,created_at) VALUES($1,$2,'declarative',$3,1,'accepted','valid','normal','normal','auto',now())")
            .bind(memory_id)
            .bind(subject.0)
            .bind(revision_id)
            .execute(&mut *tx)
            .await
            .expect("memory object");
        sqlx::query("INSERT INTO memory_revisions(memory_revision_id,memory_id,subject_id,revision_no,parent_revision_id,revision_intent,formation_mode,grounding_occurrence_id,semantic_role,title,representation_text,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) VALUES($1,$2,$3,1,NULL,NULL,'synthesized',NULL,'fact','tail fixture',$4,'inferred',$5,$6,$7,now(),now())")
            .bind(revision_id)
            .bind(memory_id)
            .bind(subject.0)
            .bind(if valid { "tail temporal match" } else { "ordinary fixture" })
            .bind(if valid { "interval" } else { "unknown" })
            .bind(valid.then_some(temporal_start))
            .bind(valid.then_some(temporal_end))
            .execute(&mut *tx)
            .await
            .expect("memory revision");
        if valid {
            sqlx::query("INSERT INTO memory_revision_aboutness(memory_revision_id,entity_ref) VALUES($1,$2)")
                .bind(revision_id)
                .bind(target_entity)
                .execute(&mut *tx)
                .await
                .expect("tail aboutness");
            tail_revision = Some(revision_id);
        }
    }
    tx.commit().await.expect("fixture commit");
    let tail_revision = tail_revision.expect("tail revision");
    let mut entity_query = query(subject);
    entity_query.cues = vec![Cue::Entity(nous_core::EntityCue {
        entity_ref: EntityRef::new(target_entity).expect("entity"),
    })];
    let entity_result = runtime.query(entity_query).await.expect("entity query");
    assert!(entity_result.results.iter().any(|hit| {
        hit.reference == CognitiveRef::MemoryRevision(nous_core::MemoryRevisionId(tail_revision))
    }));
    let mut temporal_query = query(subject);
    temporal_query.constraints.valid = Some(nous_core::TimeInterval {
        start: Some(temporal_start),
        end: Some(temporal_end),
    });
    let temporal_result = runtime.query(temporal_query).await.expect("temporal query");
    assert!(temporal_result.results.iter().any(|hit| {
        hit.reference == CognitiveRef::MemoryRevision(nous_core::MemoryRevisionId(tail_revision))
    }));
}

#[tokio::test]
async fn memory_revision_identity_guard_rejects_disjoint_aboutness() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let observation = observation(&runtime, subject, "identity guard").await;
    let alice = EntityRef::new("entity:alice").expect("alice");
    let bob = EntityRef::new("entity:bob").expect("bob");
    let mut input = form_input(
        subject,
        observation.occurrence.occurrence_id,
        OperationId::new(),
        "Alice identity matter",
    );
    input.aboutness = vec![alice.clone()];
    let memory = runtime
        .require_memory()
        .unwrap()
        .form_memory(input)
        .await
        .expect("memory");
    let mut revision = form_input(
        subject,
        observation.occurrence.occurrence_id,
        OperationId::new(),
        "Bob disjoint matter",
    );
    revision.aboutness = vec![bob.clone()];
    let rejected = runtime
        .require_memory()
        .unwrap()
        .revise_memory(nous_memory_service::ReviseMemoryInput {
            operation_id: revision.operation_id,
            subject,
            memory_id: memory.object.memory_id,
            expected_object_epoch: memory.object.object_epoch,
            intent: nous_memory_domain::RevisionIntent::Correct,
            formation_mode: revision.formation_mode,
            grounding_occurrence_id: revision.grounding_occurrence_id,
            semantic_role: revision.semantic_role,
            representation_text: revision.representation_text,
            title: revision.title,
            supports: revision.supports,
            aboutness: revision.aboutness,
            valid_time: revision.valid_time,
            formed_at: revision.formed_at,
            epistemic_class: revision.epistemic_class,
        })
        .await;
    assert!(matches!(
        rejected,
        Err(nous_core::Error::FailedPrecondition(_))
    ));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "association qualification keeps invalid and valid producer paths together"
)]
async fn association_requires_exact_cognition_and_valid_support_class() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, false, false, true).await;
    let subject = subject(&runtime).await;
    let entity = EntityRef::new("entity:association").expect("entity");
    let observation = observation(&runtime, subject, "association evidence").await;
    let mut input = form_input(
        subject,
        observation.occurrence.occurrence_id,
        OperationId::new(),
        "association memory",
    );
    input.aboutness = vec![entity.clone()];
    let memory = runtime
        .require_memory()
        .unwrap()
        .form_memory(input)
        .await
        .expect("memory");
    let invalid = runtime
        .require_memory()
        .unwrap()
        .create_association(
            nous_memory_service::CreateAssociationRequest {
                operation_id: OperationId::new(),
                from: CognitiveRef::Memory(memory.object.memory_id),
                to: CognitiveRef::Entity(entity.clone()),
                relation_kind: "assoc.related".into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::HostExplicit,
                supports: vec![AssociationSupport::Revision(RevisionSupport::Evidence(
                    EvidenceRef {
                        occurrence_id: observation.occurrence.occurrence_id,
                        locator: EvidenceLocator::WholeOccurrence,
                        support_role: SupportRole::Direct,
                    },
                ))],
                producer_signature_id: None,
                valid_time: Default::default(),
            },
            subject,
        )
        .await;
    assert!(matches!(invalid, Err(nous_core::Error::Invalid(_))));
    let event_id = UseEventId::new();
    runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:test:association".into(),
            events: vec![UseFeedbackEvent {
                event_id,
                reference: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
                use_kind: UseKind::Referenced,
                occurred_at: Utc::now(),
                context: serde_json::json!({"association":"test"}),
            }],
        })
        .await
        .expect("use event");
    let association = runtime
        .require_memory()
        .unwrap()
        .create_association(
            nous_memory_service::CreateAssociationRequest {
                operation_id: OperationId::new(),
                from: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
                to: CognitiveRef::Entity(entity.clone()),
                relation_kind: "assoc.related".into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::MeaningfulUse,
                supports: vec![AssociationSupport::UseEvent(UseEventRef {
                    subject_id: subject,
                    consumer_ref: "consumer:test:association".into(),
                    event_id,
                })],
                producer_signature_id: None,
                valid_time: Default::default(),
            },
            subject,
        )
        .await
        .expect("meaningful-use association");
    assert_eq!(association.supports.len(), 1);
    let topology = runtime
        .store
        .topology_projection_input(subject, true, true, false)
        .await
        .expect("topology input");
    assert!(topology.edges.iter().any(|edge| {
        edge.association_kind == "assoc.related" && edge.provenance_root.is_some()
    }));
    let producer = Uuid::now_v7();
    sqlx::query("INSERT INTO producer_signatures(producer_signature_id,signature_hash,provider_class,operation,implementation,model_identity,model_revision,preprocessing_identity,preprocessing_revision,config_digest,created_at,metadata) VALUES($1,$2,'test','text.interpretation','derived-test',NULL,NULL,'none','1','derived-test',now(),'{}')")
        .bind(producer)
        .bind(format!("derived-test-{producer}"))
        .execute(runtime.store.pool())
        .await
        .expect("producer signature");
    let derived = runtime
        .require_memory()
        .unwrap()
        .create_association(
            nous_memory_service::CreateAssociationRequest {
                operation_id: OperationId::new(),
                from: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
                to: CognitiveRef::Entity(entity),
                relation_kind: "assoc.derived".into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::DerivedStructure,
                supports: vec![AssociationSupport::Revision(
                    RevisionSupport::CognitionDependency(nous_memory_domain::CognitionDependency {
                        target_revision: CognitiveRef::MemoryRevision(
                            memory.revision.memory_revision_id,
                        ),
                        support_role: SupportRole::Direct,
                    }),
                )],
                producer_signature_id: Some(producer),
                valid_time: Default::default(),
            },
            subject,
        )
        .await
        .expect("derived producer association");
    assert_eq!(derived.producer_signature_id, Some(producer));
}
