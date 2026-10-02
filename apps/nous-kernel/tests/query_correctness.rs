mod test_support;

use chrono::Duration;
use chrono::Utc;
use nous_core::{
    CognitiveQuery, CognitiveQueryExpr, CognitiveRef, Cue, EntityCue, EntityRef, EpistemicClass,
    OperationId, QueryConstraints, QueryOperation, QueryTarget, ResultNeed, TemporalExtent,
    TextCue, TimeInterval, UseEventId,
};
use nous_memory::{
    AssociationPolarity, AssociationSupport, AssociationSupportClass, CognitiveRole,
    CreateSchemaInput, EvidenceLocator, EvidenceRef, FormationMode, RevisionSupport,
    SchemaEvidenceLinkInput, SchemaEvidenceRole, SchemaFormationKind, SchemaScope, SupportRole,
    UseEventRef,
};
use nous_runtime::{QueryPlan, UseFeedback, UseFeedbackEvent, UseKind};
use nous_subject::{CognitiveSeedInput, CreateSubject};
use test_support::{database, form_input, observation, open_runtime, open_runtime_with_serving};
use uuid::Uuid;

fn query(subject: nous_core::SubjectId) -> CognitiveQuery {
    CognitiveQuery {
        api_version: nous_core::API_VERSION,
        subject,
        session: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            operation: QueryOperation::Atom,
            preferences: Vec::new(),
            children: Vec::new(),
            targets: Vec::new(),
            cues: Vec::new(),
            constraints: QueryConstraints::default(),
        },
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
                format: nous_subject::COGNITIVE_SEED_FORMAT.into(),
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
#[expect(
    clippy::too_many_lines,
    reason = "one frozen Resource cohort exercises forged results, single-use tickets and descriptor drift without repeated database setup"
)]
async fn resource_continuation_fences_identity_access_and_descriptor_drift() {
    use nous_core::{
        ExternalResourceRecord, ExternalResourceResult, ResourceRef, StableExternalRef,
    };
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let owner = subject(&runtime).await;
    let other = subject(&runtime).await;
    let resource = ResourceRef::new("resource:external").unwrap();
    let descriptor = nous_runtime::ResourceUpsert {
        resource_ref: resource.clone(),
        adapter_kind: "ragflow".into(),
        provider_profile: "external".into(),
        provider_locator: "dataset selector".into(),
        display_label: None,
        authority_class: "external".into(),
        coverage: serde_json::json!({}),
        query_dimensions: serde_json::json!({}),
        modalities: serde_json::json!([]),
        freshness_policy: serde_json::json!({}),
        access_cost_class: "network".into(),
        readiness: "ready".into(),
    };
    runtime
        .cognition
        .upsert_resource(owner, descriptor.clone())
        .await
        .unwrap();
    let mut request = query(owner);
    request.expression.targets = vec![QueryTarget::Resource];
    request.expression.cues = vec![Cue::Text(TextCue {
        text: "external fact".into(),
    })];
    request.expression.constraints.current_authority = nous_core::CurrentAuthorityNeed::Required;
    request.result_need.limit = 1;
    for case in [
        "valid",
        "max_material",
        "oversized_material",
        "unknown_action",
        "foreign_resource",
        "excess",
        "denied",
        "stale",
        "drift",
    ] {
        let execution = runtime.execute_query(request.clone(), None).await.unwrap();
        let (pool, ticket) = runtime.cognition.retain_query(execution).unwrap();
        let ticket = ticket.unwrap();
        assert_eq!(pool.resource_actions.len(), 1);
        let action = &pool.resource_actions[0];
        let record = ExternalResourceRecord {
            resource_ref: resource.clone(),
            reference: StableExternalRef {
                provider_kind: "ragflow".into(),
                provider_profile: "external".into(),
                profile_digest: "a".repeat(64),
                resource_ref: resource.clone(),
                provider_resource_id: "dataset".into(),
                entry_id: "chunk".into(),
                entry_version: None,
                content_digest: "c6e1ab9c432c107aa308f5e1b6e0cc5eac5f87067d80fc8a792a76e724e53879"
                    .into(),
                source_locator: "opaque locator".into(),
                retrieved_at: Utc::now().to_rfc3339(),
                access_scope: action.provider_locator.clone(),
            },
            title: None,
            content: "external fact".into(),
            provider_rank: 1,
            provider_score: None,
            version_status: "current".into(),
            access_status: "allowed".into(),
        };
        let mut response = ExternalResourceResult {
            action_id: action.action_id,
            resource_ref: resource.clone(),
            status: "success".into(),
            records: vec![record],
        };
        match case {
            "max_material" | "oversized_material" => {
                use sha2::{Digest, Sha256};
                response.records[0].content =
                    "x".repeat(1048576 + usize::from(case == "oversized_material"));
                response.records[0].reference.content_digest =
                    Sha256::digest(response.records[0].content.as_bytes())
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect();
            }
            "unknown_action" => response.action_id = Uuid::new_v4(),
            "foreign_resource" => {
                response.records[0].reference.resource_ref =
                    ResourceRef::new("resource:foreign").unwrap()
            }
            "excess" => response.records.push(response.records[0].clone()),
            "denied" => response.records[0].access_status = "denied".into(),
            "stale" => response.records[0].version_status = "stale".into(),
            "drift" => {
                runtime
                    .cognition
                    .delete_resource(owner, resource.clone())
                    .await
                    .unwrap();
            }
            _ => {}
        }
        let contributors = || nous_runtime::CognitiveContributors {
            shared: None,
            memory: Some(runtime.require_memory().unwrap()),
        };
        assert!(
            runtime
                .cognition
                .finalize_query(
                    other,
                    ticket,
                    Vec::new(),
                    vec![response.clone()],
                    contributors()
                )
                .await
                .is_err()
        );
        let outcome = runtime
            .cognition
            .finalize_query(owner, ticket, Vec::new(), vec![response], contributors())
            .await;
        if matches!(case, "valid" | "max_material") {
            let result = outcome.unwrap();
            assert_eq!(result.resource_records.len(), 1);
            assert!(result.resource_actions.is_empty());
            assert!(
                result
                    .results
                    .iter()
                    .all(|hit| !matches!(hit.reference, CognitiveRef::Memory(_)))
            );
        } else if case == "drift" {
            let result = outcome.unwrap();
            assert!(result.resource_records.is_empty());
            assert!(
                result
                    .degradation
                    .iter()
                    .any(|value| value.code == "resource_descriptor_changed_during_query")
            );
        } else {
            assert!(outcome.is_err(), "must reject {case}");
        }
        assert!(
            runtime
                .cognition
                .finalize_query(owner, ticket, Vec::new(), Vec::new(), contributors())
                .await
                .is_err()
        );
    }
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "shared entity cohort verifies Boolean scope and soft preference eligibility without duplicate database setup"
)]
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
    let mut domain_fenced = query(subject);
    domain_fenced.expression = CognitiveQueryExpr {
        operation: QueryOperation::Any,
        targets: vec![QueryTarget::Memory],
        children: vec![
            CognitiveQueryExpr {
                targets: vec![QueryTarget::Exact {
                    reference: CognitiveRef::Artifact(
                        first_observation.artifact.as_ref().unwrap().artifact_id,
                    ),
                }],
                ..Default::default()
            },
            CognitiveQueryExpr {
                targets: vec![QueryTarget::Exact {
                    reference: CognitiveRef::MemoryRevision(first.revision.memory_revision_id),
                }],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let fenced = runtime
        .query(domain_fenced)
        .await
        .expect("parent domain fences exact evidence");
    assert!(
        fenced
            .results
            .iter()
            .all(|hit| !matches!(hit.reference, CognitiveRef::Artifact(_)))
    );
    assert!(fenced.results.iter().any(
        |hit| hit.reference == CognitiveRef::MemoryRevision(first.revision.memory_revision_id)
    ));
    let mut request = query(subject);
    request.expression.cues = vec![Cue::Entity(nous_core::EntityCue { entity_ref: alice })];
    request.expression.constraints.cognitive_roles_include =
        vec!["declarative".into(), "experiential".into()];
    let result = runtime.query(request.clone()).await.expect("entity query");
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
    let mut declarative = request.expression.clone();
    declarative.constraints.cognitive_roles_include = vec!["declarative".into()];
    let mut experiential = request.expression.clone();
    experiential.constraints.cognitive_roles_include = vec!["experiential".into()];
    let mut tree = query(subject);
    tree.expression = CognitiveQueryExpr {
        operation: QueryOperation::All,
        targets: vec![QueryTarget::Memory],
        children: vec![declarative, experiential],
        ..Default::default()
    };
    assert!(
        runtime
            .query(tree.clone())
            .await
            .expect("scoped intersection")
            .results
            .is_empty()
    );
    tree.expression.operation = QueryOperation::Any;
    assert_eq!(
        runtime
            .query(tree.clone())
            .await
            .expect("scoped union")
            .results
            .len(),
        2
    );
    tree.expression.constraints.cognitive_roles_include = vec!["declarative".into()];
    let narrowed = runtime.query(tree).await.expect("parent narrowing");
    assert_eq!(narrowed.results.len(), 1);
    assert_eq!(
        narrowed.results[0].reference,
        CognitiveRef::MemoryRevision(first.revision.memory_revision_id)
    );
    let mut preferred = request.clone();
    preferred.expression.preferences = vec![nous_core::QueryPreference {
        negative: false,
        operand: nous_core::PreferenceOperand::Cue(Cue::Text(TextCue {
            text: "experiential".into(),
        })),
    }];
    let positive = runtime
        .query(preferred.clone())
        .await
        .expect("positive preference");
    assert_eq!(positive.results.len(), 2);
    assert_eq!(
        positive.results[0].reference,
        CognitiveRef::MemoryRevision(second.revision.memory_revision_id)
    );
    assert!(positive.results[0].match_evidence.preference_score > 0.0);
    preferred.expression.preferences[0].negative = true;
    let negative = runtime
        .query(preferred.clone())
        .await
        .expect("negative preference");
    assert_eq!(
        negative.results[0].reference,
        CognitiveRef::MemoryRevision(first.revision.memory_revision_id)
    );
    preferred.expression.preferences = vec![nous_core::QueryPreference {
        negative: false,
        operand: nous_core::PreferenceOperand::Recent(nous_core::TimeAxis::Valid),
    }];
    assert!(
        runtime
            .query(preferred.clone())
            .await
            .expect("unknown valid axis")
            .results
            .iter()
            .all(|hit| hit.match_evidence.preference_score == 0.0)
    );
    preferred.expression.preferences = vec![
        nous_core::QueryPreference {
            negative: false,
            operand: nous_core::PreferenceOperand::Cue(Cue::Text(TextCue {
                text: "experiential".into()
            }))
        };
        16
    ];
    let capped = runtime
        .query(preferred.clone())
        .await
        .expect("bounded preference");
    assert_eq!(capped.results[0].match_evidence.preference_score, 0.06);
    preferred.expression.constraints.cognitive_roles_include = vec!["declarative".into()];
    assert_eq!(
        runtime
            .query(preferred)
            .await
            .expect("preference cannot bypass constraints")
            .results
            .len(),
        1
    );
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one retained query is finalized after revise, suppress and purge, including the purged proposal fence"
)]
async fn rerank_revalidates_original_candidates_after_revise_suppress_and_purge() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let source = observation(&runtime, subject, "shared source").await;
    let entity = EntityRef::new("entity:cohort").unwrap();
    let mut cohort = Vec::new();
    let mut operations = Vec::new();
    for index in 0..3 {
        let operation = OperationId::new();
        operations.push(operation);
        let mut input = form_input(
            subject,
            source.occurrence.occurrence_id,
            operation,
            &format!("candidate {index}"),
        );
        input.aboutness = vec![entity.clone()];
        cohort.push(
            runtime
                .require_memory()
                .unwrap()
                .form_memory(input)
                .await
                .unwrap(),
        );
    }
    let mut request = query(subject);
    request.expression.cues = vec![Cue::Entity(nous_core::EntityCue { entity_ref: entity })];
    let execution = runtime.execute_query(request, Some(64)).await.unwrap();
    assert_eq!(execution.result.results.len(), 3);
    let (pool, ticket) = runtime.cognition.retain_query(execution).unwrap();
    let ticket = ticket.expect("validated query snapshot should be retained");
    let first = &cohort[0];
    runtime
        .require_memory()
        .unwrap()
        .revise_memory(nous_memory::ReviseMemoryInput {
            producer: None,
            operation_id: OperationId::new(),
            subject,
            memory_id: first.object.memory_id,
            expected_object_epoch: first.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            formation_mode: FormationMode::Grounded,
            grounding_occurrence_id: Some(source.occurrence.occurrence_id),
            semantic_role: "fact".into(),
            representation_text: "replacement candidate".into(),
            title: None,
            supports: first.supports.clone(),
            aboutness: first.aboutness.clone(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Observed,
        })
        .await
        .unwrap();
    let second = &cohort[1];
    runtime
        .require_memory()
        .unwrap()
        .suppress(
            subject,
            second.object.memory_id,
            OperationId::new(),
            second.object.object_epoch,
        )
        .await
        .unwrap();
    let third = &cohort[2];
    let key = operations[2].0.to_string();
    let reservation = runtime
        .store
        .reserve_model_workflow(
            subject,
            "memory",
            &key,
            "semantic input",
            &serde_json::json!({"model":"frozen"}),
        )
        .await
        .unwrap();
    let lease = reservation.lease_token.unwrap();
    runtime
        .store
        .save_model_workflow(
            subject,
            "memory",
            &key,
            lease,
            Some(&serde_json::json!({"text":"private-cognitive-marker"})),
            None,
        )
        .await
        .unwrap();
    runtime
        .require_memory()
        .unwrap()
        .purge_memory(
            subject,
            third.object.memory_id,
            OperationId::new(),
            third.object.object_epoch,
        )
        .await
        .unwrap();
    use sqlx::Row;
    let row = sqlx::query("SELECT snapshot,proposal,outcome FROM model_workflow_operations WHERE subject_id=$1 AND owner='memory' AND operation_key=$2").bind(subject.0).bind(&key).fetch_one(runtime.store.pool()).await.unwrap();
    assert_eq!(
        row.get::<serde_json::Value, _>("snapshot"),
        serde_json::json!({})
    );
    assert!(
        row.get::<Option<serde_json::Value>, _>("proposal")
            .is_none()
    );
    assert_eq!(
        row.get::<serde_json::Value, _>("outcome"),
        serde_json::json!({"purged":true})
    );
    assert!(
        runtime
            .store
            .save_model_workflow(
                subject,
                "memory",
                &key,
                lease,
                Some(&serde_json::json!({"text":"resurrect"})),
                None
            )
            .await
            .is_err()
    );
    let order = pool
        .results
        .into_iter()
        .map(|hit| (hit.reference, 1.0))
        .collect();
    let result = runtime
        .cognition
        .finalize_query(
            subject,
            ticket,
            order,
            Vec::new(),
            nous_runtime::CognitiveContributors {
                shared: None,
                memory: Some(runtime.require_memory().unwrap()),
            },
        )
        .await
        .unwrap();
    assert!(result.results.is_empty());
    assert!(
        result
            .degradation
            .iter()
            .any(|value| value.code == "authority_changed_during_rerank")
    );
    assert_eq!(
        result.diagnostics.unwrap().candidate_counts["drop_authority_changed_during_rerank"],
        3
    );
    assert!(runtime.cognition.release_query(subject, ticket).is_ok());
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
    request.expression.targets = vec![QueryTarget::AnyRelevantCognition];
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
    request.expression.targets = vec![QueryTarget::Exact {
        reference: CognitiveRef::Memory(memory.object.memory_id),
    }];
    let bound = runtime.cognition.bind_query(request).await.expect("bind");
    let plan = QueryPlan::for_bound_query(&bound);
    let revision = nous_memory::ReviseMemoryInput {
        producer: None,
        operation_id: OperationId::new(),
        subject,
        memory_id: memory.object.memory_id,
        expected_object_epoch: memory.object.object_epoch,
        intent: nous_memory::RevisionIntent::Correct,
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
            nous_runtime::CognitiveContributors {
                shared: None,
                memory: Some(memory_service as &dyn nous_runtime::CognitiveContributor),
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
        Some(1),
        "query diagnostics: {:?}",
        result.diagnostics
    );
    let historical = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            expression: CognitiveQueryExpr {
                operation: QueryOperation::Atom,
                preferences: Vec::new(),
                children: Vec::new(),
                targets: vec![QueryTarget::Exact {
                    reference: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
                }],
                cues: Vec::new(),
                constraints: Default::default(),
            },
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
    request.expression.cues = vec![Cue::Text(TextCue { text: "lat".into() })];
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
    request.expression.cues = vec![Cue::Text(TextCue {
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
        .revise_memory(nous_memory::ReviseMemoryInput {
            producer: None,
            operation_id: OperationId::new(),
            subject,
            memory_id: first.object.memory_id,
            expected_object_epoch: first.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
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
    request.expression.cues = vec![Cue::Text(TextCue {
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
    entity_query.expression.cues = vec![Cue::Entity(nous_core::EntityCue {
        entity_ref: EntityRef::new(target_entity).expect("entity"),
    })];
    let entity_result = runtime.query(entity_query).await.expect("entity query");
    assert!(entity_result.results.iter().any(|hit| {
        hit.reference == CognitiveRef::MemoryRevision(nous_core::MemoryRevisionId(tail_revision))
    }));
    let mut temporal_query = query(subject);
    temporal_query.expression.constraints.valid = Some(nous_core::TimeInterval {
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
        .revise_memory(nous_memory::ReviseMemoryInput {
            producer: None,
            operation_id: revision.operation_id,
            subject,
            memory_id: memory.object.memory_id,
            expected_object_epoch: memory.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
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
    reason = "association contract keeps invalid and valid producer paths together"
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
            nous_memory::CreateAssociationRequest {
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
            nous_memory::CreateAssociationRequest {
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
        .topology_projection_input(subject, true)
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
            nous_memory::CreateAssociationRequest {
                operation_id: OperationId::new(),
                from: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
                to: CognitiveRef::Entity(entity),
                relation_kind: "assoc.derived".into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::DerivedStructure,
                supports: vec![AssociationSupport::Revision(
                    RevisionSupport::CognitionDependency(nous_memory::CognitionDependency {
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

fn text_query(subject: nous_core::SubjectId) -> CognitiveQuery {
    CognitiveQuery {
        api_version: nous_core::API_VERSION,
        subject,
        session: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            operation: QueryOperation::Atom,
            preferences: Vec::new(),
            children: Vec::new(),
            targets: vec![QueryTarget::AnyRelevantCognition],
            cues: vec![Cue::Text(TextCue {
                text: "diagnostic phrase".into(),
            })],
            constraints: Default::default(),
        },
        exploration: Default::default(),
        resources: Default::default(),
        result_need: ResultNeed {
            limit: 4,
            ..Default::default()
        },
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: Default::default(),
    }
}

#[tokio::test]
async fn query_distinguishes_provider_unavailable_from_unknown() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = subject(&runtime).await;
    let source = observation(&runtime, subject, "diagnostic phrase").await;
    runtime
        .require_memory()
        .expect("Memory")
        .form_memory(form_input(
            subject,
            source.occurrence.occurrence_id,
            nous_core::OperationId::new(),
            "diagnostic phrase",
        ))
        .await
        .expect("memory");
    let result = runtime.query(text_query(subject)).await.expect("query");
    assert_eq!(result.status, nous_core::QueryStatus::Degraded);
    assert!(
        result
            .degradation
            .iter()
            .any(|value| value.code == "dense_lane_unavailable")
    );
}

#[tokio::test]
async fn query_reports_validation_budget_exhaustion() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let mut tx = runtime.store.begin().await.expect("fixture transaction");
    sqlx::query("INSERT INTO memory_objects(memory_id,subject_id,cognitive_role,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,accessibility_mode,created_at) SELECT md5('budget-memory-'||n)::uuid,$1,'declarative',md5('budget-rev-'||n)::uuid,1,'accepted','valid','suppressed','normal','auto',now() FROM generate_series(1,40) AS values(n)")
        .bind(subject.0)
        .execute(&mut *tx)
        .await
        .expect("budget memories");
    sqlx::query("INSERT INTO memory_revisions(memory_revision_id,memory_id,subject_id,revision_no,parent_revision_id,revision_intent,formation_mode,grounding_occurrence_id,semantic_role,title,representation_text,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) SELECT md5('budget-rev-'||n)::uuid,md5('budget-memory-'||n)::uuid,$1,1,NULL,NULL,'synthesized',NULL,'budget',NULL,'budget memory','inferred','unknown',NULL,NULL,now(),now() FROM generate_series(1,40) AS values(n)")
        .bind(subject.0)
        .execute(&mut *tx)
        .await
        .expect("budget revisions");
    sqlx::query("INSERT INTO memory_revision_aboutness(memory_revision_id,entity_ref) SELECT md5('budget-rev-'||n)::uuid,'entity:budget' FROM generate_series(1,40) AS values(n)")
        .execute(&mut *tx)
        .await
        .expect("budget aboutness");
    sqlx::query("INSERT INTO memory_objects(memory_id,subject_id,cognitive_role,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,accessibility_mode,created_at) SELECT md5('budget-temporal-memory-'||n)::uuid,$1,'declarative',md5('budget-temporal-rev-'||n)::uuid,1,'accepted','valid','normal','normal','auto',now() FROM generate_series(1,40) AS values(n)")
        .bind(subject.0)
        .execute(&mut *tx)
        .await
        .expect("temporal budget memories");
    sqlx::query("INSERT INTO memory_revisions(memory_revision_id,memory_id,subject_id,revision_no,parent_revision_id,revision_intent,formation_mode,grounding_occurrence_id,semantic_role,title,representation_text,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) SELECT md5('budget-temporal-rev-'||n)::uuid,md5('budget-temporal-memory-'||n)::uuid,$1,1,NULL,NULL,'synthesized',NULL,'budget',NULL,'temporal budget memory','inferred','instant',now(),NULL,now(),now() FROM generate_series(1,40) AS values(n)")
        .bind(subject.0)
        .execute(&mut *tx)
        .await
        .expect("temporal budget revisions");
    tx.commit().await.expect("budget commit");
    let mut query = text_query(subject);
    query.expression.cues = vec![Cue::Entity(EntityCue {
        entity_ref: nous_core::EntityRef::new("entity:budget").unwrap(),
    })];
    query.result_need.limit = 8;
    query.expression.constraints = QueryConstraints {
        valid: Some(TimeInterval {
            start: Some(Utc::now() - Duration::seconds(5)),
            end: Some(Utc::now() + Duration::seconds(5)),
        }),
        ..Default::default()
    };
    let result = runtime.query(query).await.expect("budget query");
    assert_eq!(
        result.status,
        nous_core::QueryStatus::Partial,
        "result={result:?}"
    );
    assert!(
        result
            .degradation
            .iter()
            .any(|value| value.code == "validation_budget_exhausted")
    );
}
