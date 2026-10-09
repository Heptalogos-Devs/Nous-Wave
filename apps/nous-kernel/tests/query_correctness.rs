// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

mod test_support;

use chrono::Duration;
use chrono::Utc;
use nous_core::{
    CognitiveQueryExpr, CognitiveRef, Cue, EntityCue, EntityRef, OperationId, QueryConstraints,
    QueryOperation, QueryTarget, TextCue, TimeInterval,
};
use nous_memory::CognitiveRole;
use test_support::query::{query, subject, text_query};
use test_support::{database, form_input, observation, open_runtime, open_runtime_with_serving};
use uuid::Uuid;

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
        adapter_kind: "local-documents".into(),
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
    request.expression.cues = vec![Cue::Text(TextCue {
        text: "external fact".into(),
    })];
    assert!(!request.requests_resources());
    request.projection.domains = vec![nous_core::ResultDomain::Resource];
    assert!(request.requests_resources());
    request.result_need.limit = 1;
    let explicit = runtime.execute_query(request.clone(), None).await.unwrap();
    let (pool, ticket) = runtime.cognition.retain_query(explicit).unwrap();
    assert_eq!(pool.resource_actions.len(), 1);
    runtime
        .cognition
        .release_query(owner, ticket.unwrap())
        .unwrap();
    request.expression.constraints.current_authority = nous_core::CurrentAuthorityNeed::Required;
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
                provider_kind: "local-documents".into(),
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
            material: Some(&runtime.material),
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
    domain_fenced.projection.domains = vec![nous_core::ResultDomain::Memory];
    domain_fenced.expression = CognitiveQueryExpr {
        operation: QueryOperation::Any,
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
                cues: vec![Cue::Text(TextCue {
                    text: "Recall memory within requested domain".into(),
                })],
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
    request.expression.cues = vec![
        Cue::Text(nous_core::TextCue {
            text: "Recall cognition for this entity".into(),
        }),
        Cue::Entity(nous_core::EntityCue { entity_ref: alice }),
    ];
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
    tree.projection.domains = vec![nous_core::ResultDomain::Memory];
    tree.expression = CognitiveQueryExpr {
        operation: QueryOperation::All,
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
    entity_query.expression.cues = vec![
        Cue::Text(nous_core::TextCue {
            text: "Recall cognition for this entity".into(),
        }),
        Cue::Entity(nous_core::EntityCue {
            entity_ref: EntityRef::new(target_entity).expect("entity"),
        }),
    ];
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
    query.expression.cues = vec![
        Cue::Text(nous_core::TextCue {
            text: "Recall cognition for this entity".into(),
        }),
        Cue::Entity(EntityCue {
            entity_ref: nous_core::EntityRef::new("entity:budget").unwrap(),
        }),
    ];
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
