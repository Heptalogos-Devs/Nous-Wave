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
        text_only_compatibility: false,
        work_context: None,
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
            360,
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
            text_only_compatibility: false,
            work_context: None,
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
    let projection_budget = nous_persistence::EpisodeTextBudget {
        max_members: 32,
        fragment_max_bytes: 4096,
        total_max_bytes: 16384,
    };
    let cognitive = runtime
        .store
        .cognitive_projection_input(subject, true, projection_budget)
        .await
        .expect("coherent cognitive projection");
    assert_eq!(cognitive.topology.watermark, topology.watermark);
    assert_eq!(cognitive.topology.nodes, topology.nodes);
    let memory_reference = CognitiveRef::MemoryRevision(memory.revision.memory_revision_id);
    assert!(
        cognitive
            .sources
            .iter()
            .any(|s| s.reference == memory_reference && s.text.is_some())
    );
    assert!(
        cognitive
            .topology
            .edges
            .iter()
            .any(|e| e.association_kind == "assoc.related" && e.provenance_root.is_some())
    );
    let forbidden = runtime
        .store
        .cognitive_projection_input(subject, false, projection_budget)
        .await
        .expect("cognitive projection without memory capability");
    assert!(
        !forbidden
            .sources
            .iter()
            .any(|s| s.reference == memory_reference)
    );
    assert!(!forbidden.topology.nodes.contains(&memory_reference));
    let memory_roots = &cognitive.evidence_roots[&memory_reference];
    assert!(!memory_roots.is_empty());
    assert!(
        memory_roots
            .iter()
            .any(|root| !root.starts_with("unknown-dependency:"))
    );
    assert!(!forbidden.evidence_roots.contains_key(&memory_reference));
    check_vcp_projection_material(&runtime, subject, &memory_reference).await;
    let mut native_request = query(subject);
    native_request.expression.targets = vec![QueryTarget::Exact {
        reference: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
    }];
    native_request.exploration = nous_core::ExplorationIntent::BoundedAssociative;
    native_request.diagnostics = nous_core::DiagnosticsRequest::Summary;
    let result = runtime
        .query(native_request)
        .await
        .expect("native cognitive readout");
    let diagnostics = result.diagnostics.expect("native observation summary");
    assert_eq!(
        diagnostics.lane_status["topology_profile"],
        "nous-node-potential-v1"
    );
    assert_eq!(
        diagnostics.lane_status["topology_mechanism"],
        "experimental-node-potential-v1"
    );
    assert_eq!(diagnostics.lane_status["topology_profile_digest"].len(), 64);
    assert!(diagnostics.candidate_counts["topology_seed_count"] > 0);
    assert!(diagnostics.candidate_counts["topology_activated_edges"] > 0);
    assert!(diagnostics.candidate_counts["topology_max_hop_observed"] > 0);
    assert!(result.results.iter().any(
        |hit| hit.reference == CognitiveRef::MemoryRevision(memory.revision.memory_revision_id)
    ));
    let mut request = query(subject);
    request.expression.targets = vec![QueryTarget::Exact {
        reference: CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
    }];
    request.exploration = nous_core::ExplorationIntent::BoundedAssociative;
    let frozen = runtime
        .cognition
        .bind_query(request.clone())
        .await
        .expect("frozen native query");
    let plan = QueryPlan::for_bound_query(&frozen);
    let old_generation = runtime
        .serving
        .publisher
        .snapshot_for(subject)
        .topology
        .as_ref()
        .expect("native generation")
        .generation_id;
    let receipt = runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_runtime::COGNITIVE_PROFILE.path(),
            serde_json::json!("baseline-rrf"),
        )
        .await
        .expect("baseline profile");
    assert_eq!(
        receipt.apply_mode,
        nous_configuration::ConfigApplyMode::Live
    );
    assert_eq!(
        plan.cognitive_profile,
        nous_runtime::CognitiveProfile::NousNodePotential
    );
    let baseline = runtime
        .cognition
        .bind_query(request.clone())
        .await
        .expect("baseline bound");
    let baseline_plan = QueryPlan::for_bound_query(&baseline);
    assert_eq!(
        baseline_plan.cognitive_profile,
        nous_runtime::CognitiveProfile::BaselineRrf
    );
    assert!(!baseline_plan.expand_topology);
    assert!(!baseline.lane_enabled(nous_core::EvidenceFamily::TopologyWave));
    let old_output = nous_runtime::SharedLaneProvider::lanes(&runtime.serving, &frozen, &plan)
        .await
        .expect("frozen profile readout");
    assert_eq!(
        old_output
            .iter()
            .find(|lane| lane.family == nous_core::EvidenceFamily::TopologyWave)
            .expect("native lane")
            .generation_ref,
        Some(old_generation)
    );
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_runtime::COGNITIVE_PROFILE.path(),
            serde_json::json!("vcp-rivermemo-v3.1-adapter-v1"),
        )
        .await
        .expect("reference profile");
    let reference_bound = runtime
        .cognition
        .bind_query(request)
        .await
        .expect("reference bound");
    let reference_plan = QueryPlan::for_bound_query(&reference_bound);
    runtime
        .serving
        .prepare_with_snapshot(
            subject,
            reference_plan.serving_need(&reference_bound.source_query),
            &reference_bound.config_snapshot,
        )
        .await
        .expect("reference prepare");
    let generation = runtime.serving.publisher.snapshot_for(subject);
    let graph = generation.vcp.as_ref().expect("VCP profile generation");
    assert!(generation.topology.is_none());
    graph.validate().unwrap();
    let query_embedding = nous_retrieval::TextEmbeddingOutput {
        vector: vec![1.0, 0.0, 0.0],
        space: graph.space.clone(),
        producer: graph.producer.clone(),
    };
    let observation = nous_retrieval::VcpQueryObservation::prepare(
        graph,
        &reference_bound,
        &reference_plan,
        &query_embedding,
    )
    .unwrap();
    assert_eq!(observation.original_vector(), &[1.0, 0.0, 0.0]);
    assert_eq!(observation.generation_id(), graph.generation_id);
    assert_eq!(observation.profile_id(), "vcp-rivermemo-v3.1-adapter-v1");
    assert!(!observation.numerical().epa.cache_available);
    assert!(observation.numerical().sense.source_field.is_empty());
    check_vcp_nonempty_observation(
        &runtime,
        subject,
        &reference_bound,
        &reference_plan,
        &query_embedding,
    )
    .await;

    let mut insufficient_plan = reference_plan.clone();
    insufficient_plan.topology_nodes = 50;
    assert!(
        nous_retrieval::VcpQueryObservation::prepare(
            graph,
            &reference_bound,
            &insufficient_plan,
            &query_embedding
        )
        .is_err()
    );
    let mut wrong_embedding = query_embedding.clone();
    wrong_embedding.producer.signature_hash = "different".into();
    assert!(
        nous_retrieval::VcpQueryObservation::prepare(
            graph,
            &reference_bound,
            &reference_plan,
            &wrong_embedding
        )
        .is_err()
    );
    let mut forbidden_bound = reference_bound.clone();
    forbidden_bound.source_query.capabilities.text_embedding =
        nous_core::RequirementStrength::Forbidden;
    assert!(
        nous_retrieval::VcpQueryObservation::prepare(
            graph,
            &forbidden_bound,
            &reference_plan,
            &query_embedding
        )
        .is_err()
    );

    let indexed = graph
        .search_candidates(&[1.0, 0.0, 0.0], graph.vectors.len())
        .unwrap();
    assert_eq!(indexed.len(), graph.vectors.len());
    assert!(
        indexed
            .iter()
            .any(|hit| hit.record.as_ref().unwrap().reference == memory_reference)
    );
    assert!(indexed.iter().all(|hit| hit.distance.abs() < 1e-6));
    assert!(
        graph
            .search_residual_tags(&[1.0, 0.0, 0.0], 10)
            .unwrap()
            .is_empty()
    );

    let encoded = serde_json::to_value(graph.as_ref()).unwrap();
    let mut decoded: nous_retrieval::VcpGeneration =
        serde_json::from_value(encoded.clone()).unwrap();
    decoded.curves[0].chunk_vector[0] += 0.1;
    assert!(decoded.validate().is_err());
    let mut reordered = encoded;
    reordered["identities"]["references"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let decoded: nous_retrieval::VcpGeneration = serde_json::from_value(reordered).unwrap();
    assert!(decoded.validate().is_err());
    assert_eq!(graph.policy.epa_anchors, 64);
    assert!(
        graph
            .curves
            .iter()
            .any(|c| c.id == graph.identities.id(&memory_reference).unwrap())
    );
    assert_ne!(old_generation, graph.generation_id);
    assert_eq!(
        graph.cognitive_profile,
        nous_runtime::CognitiveProfile::VcpRiverMemo
    );
    let current = runtime
        .store
        .serving_current(subject)
        .await
        .expect("published metadata");
    let topology = current
        .iter()
        .find(|record| record.family == "topology")
        .expect("topology record");
    assert_eq!(
        topology.metadata["cognitive_profile"],
        "vcp-rivermemo-v3.1-adapter-v1"
    );
    assert!(
        std::path::Path::new(&topology.artifact_location)
            .join("vcp.json")
            .exists()
    );
    runtime
        .serving
        .publisher
        .publish_for(subject, nous_retrieval::ServingSnapshot::default());
    let reopened = runtime
        .serving
        .prepare_with_snapshot(
            subject,
            reference_plan.serving_need(&reference_bound.source_query),
            &reference_bound.config_snapshot,
        )
        .await
        .unwrap();
    assert_eq!(reopened.reopened, vec!["exact", "topology"]);
    assert_eq!(
        runtime
            .serving
            .publisher
            .snapshot_for(subject)
            .vcp
            .as_ref()
            .unwrap()
            .generation_id,
        graph.generation_id
    );

    let stale_output = nous_runtime::SharedLaneProvider::lanes(&runtime.serving, &frozen, &plan)
        .await
        .expect("generation mismatch");
    let stale_topology = stale_output
        .iter()
        .find(|lane| lane.family == nous_core::EvidenceFamily::TopologyWave)
        .expect("old lane");
    assert_eq!(stale_topology.status, nous_runtime::LaneStatus::Unavailable);
    assert!(stale_topology.candidates.is_empty());
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
    let previous_vcp = runtime
        .serving
        .publisher
        .snapshot_for(subject)
        .vcp
        .as_ref()
        .unwrap()
        .generation_id;
    let mut policy = nous_retrieval::VcpAssetPolicy::default();
    policy.graph.outbound_mass = 0.7;
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_retrieval::VCP_ASSETS.path(),
            serde_json::to_value(policy).unwrap(),
        )
        .await
        .unwrap();
    let changed_snapshot = runtime.configuration.snapshot_for_subject(subject).unwrap();
    let changed = runtime
        .serving
        .prepare_with_snapshot(
            subject,
            nous_core::ServingNeed {
                exact: false,
                lexical: false,
                dense: false,
                topology: true,
            },
            &changed_snapshot,
        )
        .await
        .unwrap();
    assert_eq!(changed.rebuilt, vec!["topology"]);
    let published = runtime.serving.publisher.snapshot_for(subject);
    let assets = published.vcp.as_ref().unwrap();
    assert_ne!(assets.generation_id, previous_vcp);
    assert_eq!(assets.policy.graph.outbound_mass, 0.7);
    assert!(
        nous_retrieval::VcpQueryObservation::prepare(
            assets,
            &reference_bound,
            &reference_plan,
            &query_embedding
        )
        .is_err()
    );
    assert_eq!(observation.generation_id(), graph.generation_id);

    let authority_seq: i64 =
        sqlx::query_scalar("SELECT authority_seq FROM subjects WHERE subject_id=$1")
            .bind(subject.0)
            .fetch_one(runtime.store.pool())
            .await
            .unwrap();
    assert_eq!(assets.authority_watermark, authority_seq);
    let transport = &assets.graph.graph.transport;
    for row in transport
        .row_offsets
        .windows(2)
        .filter(|row| row[1] > row[0])
    {
        assert!((transport.weights[row[0]..row[1]].iter().sum::<f64>() - 0.7).abs() < 1e-12);
    }
    Box::pin(check_public_vcp_queries(
        &runtime,
        subject,
        &memory_reference,
        &query_embedding,
    ))
    .await;
}

async fn check_public_vcp_queries(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
    memory: &CognitiveRef,
    embedding: &nous_retrieval::TextEmbeddingOutput,
) {
    for profile in [
        "vcp-rivermemo-v3.1-adapter-v1",
        "vcp-dtsc-v9.2.1-adapter-v1",
    ] {
        runtime
            .configuration
            .set_system_override(
                OperationId::new(),
                nous_runtime::COGNITIVE_PROFILE.path(),
                serde_json::json!(profile),
            )
            .await
            .unwrap();
        let mut request = query(subject);
        request.expression.cues = vec![nous_core::Cue::Text(nous_core::TextCue {
            text: "association memory".into(),
        })];
        request.expression.targets = vec![QueryTarget::Memory];
        request.exploration = nous_core::ExplorationIntent::BoundedAssociative;
        request.diagnostics = nous_core::DiagnosticsRequest::Summary;
        let material = vec![nous_retrieval::QueryEmbedding {
            text: runtime
                .cognition
                .bind_query(request.clone())
                .await
                .unwrap()
                .representation
                .text,
            output: embedding.clone(),
        }];
        let result =
            nous_retrieval::with_query_material(material.clone(), runtime.query(request.clone()))
                .await
                .unwrap();
        assert!(result.results.iter().any(|hit| &hit.reference == memory));
        let diagnostics = result.diagnostics.unwrap();
        assert_eq!(
            diagnostics
                .lane_status
                .get("topology_profile")
                .map(String::as_str),
            Some(profile),
            "{diagnostics:?}"
        );
        assert!(diagnostics.lane_status["topologywave"].contains("ready"));
        assert_eq!(diagnostics.topology_discarded_mass, None);
        let bound = runtime.cognition.bind_query(request.clone()).await.unwrap();
        let plan = QueryPlan::for_bound_query(&bound);
        let CognitiveRef::MemoryRevision(revision) = memory else {
            panic!("memory revision fixture");
        };
        let memory_id: Uuid = sqlx::query_scalar(
            "SELECT memory_id FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2",
        )
        .bind(subject.0)
        .bind(revision.0)
        .fetch_one(runtime.store.pool())
        .await
        .unwrap();
        let view = runtime
            .require_memory()
            .unwrap()
            .memory(subject, nous_core::MemoryId(memory_id), None)
            .await
            .unwrap();
        let suppressed = runtime
            .require_memory()
            .unwrap()
            .suppress(
                subject,
                view.object.memory_id,
                OperationId::new(),
                view.object.object_epoch,
            )
            .await
            .unwrap();
        let stale = nous_retrieval::with_query_material(
            material,
            nous_runtime::SharedLaneProvider::lanes(&runtime.serving, &bound, &plan),
        )
        .await
        .unwrap();
        let lane = stale
            .iter()
            .find(|lane| lane.family == nous_core::EvidenceFamily::TopologyWave)
            .unwrap();
        assert_eq!(lane.status, nous_runtime::LaneStatus::Unavailable);
        assert!(lane.candidates.is_empty());
        runtime
            .require_memory()
            .unwrap()
            .restore(
                subject,
                suppressed.object.memory_id,
                OperationId::new(),
                suppressed.object.object_epoch,
            )
            .await
            .unwrap();
        request.capabilities.text_embedding = nous_core::RequirementStrength::Forbidden;
        let forbidden = runtime.query(request).await.unwrap();
        assert!(
            !forbidden
                .diagnostics
                .unwrap()
                .lane_status
                .contains_key("topology_profile")
        );
    }
    check_vcp_native_switch_freshness(runtime, subject).await;
}

async fn check_vcp_native_switch_freshness(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
) {
    observation(
        runtime,
        subject,
        "topology publication full-watermark regression",
    )
    .await;
    let provider = runtime.serving.embedding().unwrap();
    for need in runtime.serving.embedding_needs(subject, 256).await.unwrap() {
        runtime
            .serving
            .commit_embedding(
                subject,
                need.reference,
                need.text,
                &provider.space().space_hash,
                &provider.producer().signature_hash,
                vec![1.0, 0.0, 0.0],
            )
            .await
            .unwrap();
    }
    let desired = runtime
        .store
        .projection_watermark(subject, "topology", "")
        .await
        .unwrap();
    let full: i64 = sqlx::query_scalar("SELECT authority_seq FROM subjects WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(runtime.store.pool())
        .await
        .unwrap();
    assert!(full > desired);
    let need = nous_core::ServingNeed {
        exact: false,
        lexical: false,
        dense: false,
        topology: true,
    };
    let mut visited = std::collections::HashSet::new();
    for index in 0..100 {
        let profile = match index % 4 {
            0 | 2 => "vcp-dtsc-v9.2.1-adapter-v1",
            1 => "vcp-rivermemo-v3.1-adapter-v1",
            _ => "nous-node-potential-v1",
        };
        runtime
            .configuration
            .set_system_override(
                OperationId::new(),
                nous_runtime::COGNITIVE_PROFILE.path(),
                serde_json::json!(profile),
            )
            .await
            .unwrap();
        let snapshot = runtime.configuration.snapshot_for_subject(subject).unwrap();
        let status = runtime
            .serving
            .prepare_with_snapshot(subject, need, &snapshot)
            .await
            .unwrap();
        assert!(status.degradation.is_empty());
        visited.insert(status.generations["topology"]);
        if index > 3 {
            assert!(status.rebuilt.is_empty());
        }
    }
    assert_eq!(
        visited.len(),
        2,
        "readout switches reuse VCP/native artifacts"
    );
    let snapshot = runtime.serving.publisher.snapshot_for(subject);
    assert!(snapshot.vcp.is_none());
    assert_eq!(
        snapshot.topology.as_ref().unwrap().cognitive_profile,
        nous_runtime::CognitiveProfile::NousNodePotential
    );
    let current = runtime.store.serving_current(subject).await.unwrap();
    assert_eq!(
        current
            .iter()
            .find(|record| record.family == "topology")
            .unwrap()
            .authority_watermark,
        full
    );
    check_generation_reclamation(runtime, subject, &visited).await;
}

async fn check_generation_reclamation(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
    visited: &std::collections::HashSet<nous_core::ServingGenerationId>,
) {
    let retired = runtime
        .store
        .serving_reusable(subject)
        .await
        .unwrap()
        .into_iter()
        .find(|record| {
            visited.contains(&record.generation_id)
                && record.metadata["cognitive_profile"] == "vcp-dtsc-v9.2.1-adapter-v1"
        })
        .unwrap();
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_runtime::COGNITIVE_PROFILE.path(),
            serde_json::json!("vcp-dtsc-v9.2.1-adapter-v1"),
        )
        .await
        .unwrap();
    let mut lease_query = query(subject);
    lease_query.expression.cues.push(Cue::Entity(EntityCue {
        entity_ref: EntityRef::new("entity:association").unwrap(),
    }));
    lease_query.exploration = nous_core::ExplorationIntent::BoundedAssociative;
    let held = runtime.execute_query(lease_query, Some(5)).await.unwrap();
    let (_, ticket) = runtime.cognition.retain_query(held).unwrap();
    let ticket = ticket.unwrap();
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_runtime::COGNITIVE_PROFILE.path(),
            serde_json::json!("nous-node-potential-v1"),
        )
        .await
        .unwrap();
    runtime
        .serving
        .prepare(
            subject,
            nous_core::ServingNeed {
                topology: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let busy = runtime
        .serving
        .reclaim_retired(subject, std::time::Duration::ZERO)
        .await
        .unwrap();
    assert!(busy.readers_active);
    assert!(!busy.reclaimed.contains(&retired.generation_id));
    assert!(
        !busy.reclaimed.is_empty(),
        "active ticket does not block unrelated retired assets"
    );
    assert!(std::path::Path::new(&retired.artifact_location).exists());
    runtime
        .serving
        .pin_research_generation(retired.generation_id, true)
        .await
        .unwrap();
    runtime.cognition.release_query(subject, ticket).unwrap();
    let pinned = runtime
        .serving
        .reclaim_retired(subject, std::time::Duration::ZERO)
        .await
        .unwrap();
    assert!(!pinned.reclaimed.contains(&retired.generation_id));
    runtime
        .serving
        .pin_research_generation(retired.generation_id, false)
        .await
        .unwrap();
    let orphan = runtime.serving.options.root.join(".staging-abandoned");
    std::fs::create_dir(&orphan).unwrap();
    std::fs::write(orphan.join("partial"), b"orphan").unwrap();
    let collected = runtime
        .serving
        .reclaim_retired(subject, std::time::Duration::ZERO)
        .await
        .unwrap();
    assert!(collected.reclaimed.contains(&retired.generation_id));
    assert!(collected.bytes_reclaimed > 0);
    assert_eq!(collected.orphan_directories, 1);
    assert!(!orphan.exists());
    assert!(!std::path::Path::new(&retired.artifact_location).exists());
    let active = runtime.store.serving_current(subject).await.unwrap();
    assert!(
        active
            .iter()
            .all(|record| std::path::Path::new(&record.artifact_location).exists())
    );
}

async fn check_vcp_nonempty_observation(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
    bound: &nous_runtime::BoundQuery,
    plan: &QueryPlan,
    embedding: &nous_retrieval::TextEmbeddingOutput,
) {
    let (material, a) = nonempty_vcp_lab_material(bound, embedding);
    let tag_a = CognitiveRef::Tag(a);
    let assets = nous_retrieval::VcpGeneration::build(
        nous_core::ServingGenerationId::new(),
        plan.cognitive_profile,
        material,
        bound
            .config_snapshot
            .get(nous_retrieval::VCP_ASSETS)
            .unwrap(),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let generation =
        nous_retrieval::VcpServingGeneration::create(assets, directory.path()).unwrap();
    let baseline_policy = nous_retrieval::VcpQueryPolicy::default();
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_retrieval::VCP_QUERY.path(),
            serde_json::to_value(&baseline_policy).unwrap(),
        )
        .await
        .unwrap();
    let mut bound = bound.clone();
    bound.config_snapshot = runtime.configuration.snapshot_for_subject(subject).unwrap();
    bound
        .source_query
        .expression
        .cues
        .push(nous_core::Cue::Tag(nous_core::TagCue { tag: a }));
    let observation =
        nous_retrieval::VcpQueryObservation::prepare(&generation, &bound, plan, embedding).unwrap();
    assert_eq!(
        observation.core_tag_ids(),
        &[generation.identities.id(&tag_a).unwrap()]
    );
    assert!(!observation.numerical().pyramid.levels.is_empty());
    assert!(!observation.numerical().sense.source_field.is_empty());
    assert!(!observation.numerical().fields.local_field.is_empty());
    assert!(observation.numerical().fields.local_converged);
    check_vcp_readouts(&generation, &observation, &bound);
    let mut query_policy = baseline_policy;
    query_policy.sense.fir_gamma = 0.9;
    runtime
        .configuration
        .set_system_override(
            OperationId::new(),
            nous_retrieval::VCP_QUERY.path(),
            serde_json::to_value(query_policy).unwrap(),
        )
        .await
        .unwrap();
    let changed_snapshot = runtime.configuration.snapshot_for_subject(subject).unwrap();
    let previous_generation = runtime
        .serving
        .publisher
        .snapshot_for(subject)
        .vcp
        .as_ref()
        .unwrap()
        .generation_id;
    let status = runtime
        .serving
        .prepare_with_snapshot(
            subject,
            nous_core::ServingNeed {
                exact: false,
                lexical: false,
                dense: false,
                topology: true,
            },
            &changed_snapshot,
        )
        .await
        .unwrap();
    assert!(status.rebuilt.is_empty());
    assert_eq!(
        runtime
            .serving
            .publisher
            .snapshot_for(subject)
            .vcp
            .as_ref()
            .unwrap()
            .generation_id,
        previous_generation
    );
    bound.config_snapshot = changed_snapshot;
    let changed =
        nous_retrieval::VcpQueryObservation::prepare(&generation, &bound, plan, embedding).unwrap();
    assert_eq!(changed.policy().sense.fir_gamma, 0.9);
    assert_ne!(
        changed.config_subset_digest(),
        observation.config_subset_digest()
    );
    assert_ne!(
        changed.numerical().sense.source_field,
        observation.numerical().sense.source_field
    );
    assert_eq!(changed.generation_id(), observation.generation_id());
}

fn check_vcp_readouts(
    generation: &nous_retrieval::VcpServingGeneration,
    observation: &nous_retrieval::VcpQueryObservation,
    bound: &nous_runtime::BoundQuery,
) {
    let body = bound.exact_bindings[0].bound_ref.clone();
    let candidate = nous_retrieval::VcpReadoutCandidate {
        reference: body.clone(),
        base_score: 0.9,
        bm25_score: 0.2,
        time_score: 0.0,
        anchor_score: 0.0,
        self_evidence_roots: Default::default(),
    };
    let policy = bound
        .config_snapshot
        .get(nous_retrieval::VCP_READOUT)
        .unwrap();
    let before = serde_json::to_value(observation).unwrap();
    let dtsc = nous_retrieval::vcp_dtsc_readout(
        generation,
        observation,
        std::slice::from_ref(&candidate),
        &policy.dtsc,
        1,
    )
    .unwrap();
    let v3 = nous_retrieval::vcp_v3_readout(
        generation,
        observation,
        std::slice::from_ref(&candidate),
        &policy.v3,
        1,
    )
    .unwrap();
    assert_eq!(dtsc.results.len(), 1);
    assert_eq!(v3.results.len(), 1);
    assert_eq!(
        generation.identities.reference(dtsc.results[0].id).unwrap(),
        &body
    );
    assert_eq!(
        generation.identities.reference(v3.results[0].id).unwrap(),
        &body
    );
    assert!(dtsc.results[0].score.is_finite() && v3.results[0].score.is_finite());
    assert_eq!(serde_json::to_value(observation).unwrap(), before);
    let mut own = candidate.clone();
    own.self_evidence_roots
        .insert("occurrence:lab-independent".into());
    let owned = nous_retrieval::vcp_v3_readout(
        generation,
        observation,
        std::slice::from_ref(&own),
        &policy.v3,
        1,
    )
    .unwrap();
    assert!(
        owned.results[0].relative_topology.edge_topology_score
            < v3.results[0].relative_topology.edge_topology_score
    );
    own.self_evidence_roots.insert("unknown:root".into());
    assert!(
        nous_retrieval::vcp_v3_readout(generation, observation, &[own], &policy.v3, 1).is_err()
    );
    assert!(
        nous_retrieval::vcp_dtsc_readout(
            generation,
            observation,
            &[candidate.clone(), candidate],
            &policy.dtsc,
            1
        )
        .is_err()
    );
}

fn nonempty_vcp_lab_material(
    bound: &nous_runtime::BoundQuery,
    embedding: &nous_retrieval::TextEmbeddingOutput,
) -> (nous_retrieval::VcpProjectionMaterial, nous_core::TagId) {
    use nous_retrieval::{VcpCurveOrder, VcpProjectedDocument};
    let a = nous_core::TagId::new();
    let b = nous_core::TagId::new();
    let tag_a = CognitiveRef::Tag(a);
    let tag_b = CognitiveRef::Tag(b);
    let body = bound.exact_bindings[0].bound_ref.clone();
    // Independent lab material exercises numerical adapter input. These Tag
    // identities are not persisted or used as public-query Authority evidence.
    let material = nous_retrieval::VcpProjectionMaterial {
        authority_watermark: bound.bound_at_authority_seq,
        space: embedding.space.clone(),
        producer: embedding.producer.clone(),
        identities: nous_retrieval::VcpIdentityMap::new([
            body.clone(),
            tag_a.clone(),
            tag_b.clone(),
        ]),
        edges: vec![nous_persistence::TopologyEdgeSource {
            from: tag_a.clone(),
            to: tag_b.clone(),
            support_class: "source_evidence".into(),
            association_kind: "assoc.related".into(),
            polarity: "positive".into(),
            support_mass: 0.6,
            provenance_root: Some("occurrence:lab-independent".into()),
        }],
        documents: vec![
            VcpProjectedDocument {
                reference: body,
                representation_text: "body".into(),
                vector: vec![1.0, 0.0, 0.0],
                concept_refs: vec![tag_a.clone(), tag_b.clone()],
                curve_order: VcpCurveOrder::StableIdentity,
                evidence_roots: Default::default(),
            },
            VcpProjectedDocument {
                reference: tag_a.clone(),
                representation_text: "查询线索".into(),
                vector: vec![1.0, 0.0, 0.0],
                concept_refs: Vec::new(),
                curve_order: VcpCurveOrder::StableIdentity,
                evidence_roots: Default::default(),
            },
            VcpProjectedDocument {
                reference: tag_b,
                representation_text: "其他线索".into(),
                vector: vec![0.6, 0.8, 0.0],
                concept_refs: Vec::new(),
                curve_order: VcpCurveOrder::StableIdentity,
                evidence_roots: Default::default(),
            },
        ],
    };
    (material, a)
}

async fn check_vcp_projection_material(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
    memory: &CognitiveRef,
) {
    let mut space = nous_core::EmbeddingSpaceSignature {
        space_hash: String::new(),
        model_identity: "fixture-vcp".into(),
        weights_revision: "1".into(),
        task: "retrieval".into(),
        input_representation: "text".into(),
        preprocessing_identity: "fixture".into(),
        preprocessing_revision: "1".into(),
        dimension: 3,
        normalization: "l2".into(),
        output_semantics: "dense".into(),
    };
    space.space_hash = blake3::hash(&serde_json::to_vec(&space).unwrap())
        .to_hex()
        .to_string();
    let producer =
        nous_persistence::AuthorityStore::canonical_producer(&nous_core::ProducerSignature {
            signature_hash: String::new(),
            provider_class: "fixture".into(),
            operation: nous_core::CapabilityOperation::TextEmbedding,
            implementation: "fixture".into(),
            model_identity: Some("fixture-vcp".into()),
            model_revision: Some("1".into()),
            output_schema_digest: None,
            preprocessing_identity: "fixture".into(),
            preprocessing_revision: "1".into(),
            config_digest: "fixture".into(),
        })
        .unwrap();
    runtime
        .serving
        .initialize_embedding(nous_retrieval::StoredEmbeddingConfig {
            space: space.clone(),
            producer: producer.clone(),
        })
        .unwrap();
    let snapshot = runtime.configuration.snapshot_for_subject(subject).unwrap();
    assert!(matches!(
        runtime
            .serving
            .vcp_projection_material(subject, &snapshot)
            .await,
        Err(nous_core::Error::Unavailable(_))
    ));
    for need in runtime.serving.embedding_needs(subject, 256).await.unwrap() {
        runtime
            .serving
            .commit_embedding(
                subject,
                need.reference,
                need.text,
                &space.space_hash,
                &producer.signature_hash,
                vec![1.0, 0.0, 0.0],
            )
            .await
            .unwrap();
    }
    let material = runtime
        .serving
        .vcp_projection_material(subject, &snapshot)
        .await
        .unwrap();
    assert_eq!(material.space, space);
    assert!(
        material
            .documents
            .iter()
            .find(|d| &d.reference == memory)
            .unwrap()
            .evidence_roots
            .iter()
            .any(|root| !root.starts_with("unknown-dependency:"))
    );
    assert_eq!(material.producer.signature_hash, producer.signature_hash);
    assert!(
        material
            .documents
            .iter()
            .any(|d| &d.reference == memory && d.vector == vec![1.0, 0.0, 0.0])
    );
    assert!(
        material
            .edges
            .iter()
            .any(|e| e.association_kind == "assoc.related")
    );
    assert_eq!(
        material
            .identities
            .reference(material.identities.id(memory).unwrap())
            .unwrap(),
        memory
    );
    let current: i64 = sqlx::query_scalar("SELECT authority_seq FROM subjects WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(runtime.store.pool())
        .await
        .unwrap();
    assert_eq!(material.authority_watermark, current);
    let config = nous_retrieval::VcpAssetPolicy::default().graph;
    let assets = nous_retrieval::vcp_graph_assets(&material, &[], &[], &config).unwrap();
    assert!(
        assets
            .evidence
            .iter()
            .any(|e| e.association_kind == "assoc.related")
    );
    assert!(!assets.graph.transport.weights.is_empty());
    for edge in &assets.graph.provenance {
        material.identities.reference(edge.source_id).unwrap();
        material.identities.reference(edge.target_id).unwrap();
        for (root_id, mass) in &edge.file_contributions {
            assert!(*root_id > 0 && *root_id as usize <= assets.provenance_roots.len());
            assert!(mass.is_finite() && *mass > 0.0);
        }
    }
}

fn text_query(subject: nous_core::SubjectId) -> CognitiveQuery {
    CognitiveQuery {
        text_only_compatibility: false,
        work_context: None,
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
    let mut associative = text_query(subject);
    associative.exploration = nous_core::ExplorationIntent::BoundedAssociative;
    let unavailable = runtime.query(associative).await.expect("associative query");
    assert!(
        unavailable
            .degradation
            .iter()
            .any(|value| value.code == "topologywave_lane_unavailable")
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

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one real evidence cohort verifies past/future occurrence and region filters through finalization"
)]
async fn evidence_time_constraints_filter_occurrences_and_regions_through_finalization() {
    use nous_material::{
        ObservationInput, ObservationMaterial, OccurrenceDescriptor, RuntimeDirective,
    };
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = subject(&runtime).await;
    let now = Utc::now();
    let mut observations = Vec::new();
    for (time, text) in [
        (now - Duration::days(2), "chronicle historical approval"),
        (now + Duration::days(2), "chronicle future approval"),
    ] {
        observations.push(
            runtime
                .material
                .record_observation(ObservationInput {
                    subject,
                    session: None,
                    occurrence: OccurrenceDescriptor {
                        source_class: nous_core::SourceClass::Message,
                        external_object_ref: None,
                        occurred_time: TemporalExtent::Instant { at: time },
                        observed_at: Some(now),
                        conversation_ref: None,
                        actor_entity_ref: None,
                        context: serde_json::json!({}),
                    },
                    material: ObservationMaterial::InlineText {
                        text: text.into(),
                        media_type: "text/plain".into(),
                    },
                    entities: Vec::new(),
                    runtime: RuntimeDirective::default(),
                })
                .await
                .unwrap(),
        );
    }
    let mut input = query(subject);
    input.expression.cues.push(Cue::Text(TextCue {
        text: "chronicle approval".into(),
    }));
    input.expression.constraints.occurred = Some(TimeInterval {
        start: None,
        end: Some(now),
    });
    let execution = runtime.execute_query(input, Some(32)).await.unwrap();
    let past = CognitiveRef::Occurrence(observations[0].occurrence.occurrence_id);
    let future = CognitiveRef::Occurrence(observations[1].occurrence.occurrence_id);
    let past_region = CognitiveRef::SourceRegion(
        observations[0]
            .source_region
            .as_ref()
            .unwrap()
            .source_region_id,
    );
    let future_region = CognitiveRef::SourceRegion(
        observations[1]
            .source_region
            .as_ref()
            .unwrap()
            .source_region_id,
    );
    assert!(
        execution
            .result
            .results
            .iter()
            .any(|hit| hit.reference == past_region)
    );
    assert!(
        !execution
            .result
            .results
            .iter()
            .any(|hit| hit.reference == future_region)
    );
    assert!(
        execution
            .result
            .results
            .iter()
            .any(|hit| hit.reference == past)
    );
    assert!(
        !execution
            .result
            .results
            .iter()
            .any(|hit| hit.reference == future)
    );
    assert!(
        execution
            .result
            .results
            .iter()
            .filter(|hit| matches!(
                hit.reference,
                CognitiveRef::Occurrence(_) | CognitiveRef::SourceRegion(_)
            ))
            .all(|hit| !hit.freshness.occurred.is_empty()
                && hit
                    .freshness
                    .occurred
                    .iter()
                    .all(|time| time.overlaps_interval(&TimeInterval {
                        start: None,
                        end: Some(now)
                    })))
    );
    let (_, ticket) = runtime.cognition.retain_query(execution).unwrap();
    let result = runtime
        .cognition
        .finalize_query(
            subject,
            ticket.unwrap(),
            Vec::new(),
            Vec::new(),
            nous_runtime::CognitiveContributors {
                shared: None,
                memory: runtime
                    .memory
                    .as_ref()
                    .map(|owner| owner as &dyn nous_runtime::CognitiveContributor),
            },
        )
        .await
        .unwrap();
    assert!(result.results.iter().any(|hit| hit.reference == past));
    assert!(!result.results.iter().any(|hit| hit.reference == future));
    assert!(
        !result
            .results
            .iter()
            .any(|hit| hit.reference == future_region)
    );
}
