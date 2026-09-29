#[path = "test_support/mod.rs"]
mod test_support;

use chrono::{Duration, Utc};
use nous_core::{
    CognitiveQuery, CognitiveRef, Cue, EntityCue, QueryConstraints, QueryTarget, ResultNeed,
    TextCue, TimeInterval,
};
use serde_json::Value;
use test_support::{database, form_input, observation, open_runtime, open_runtime_with_serving};

const ORACLE: &str = include_str!("fixtures/memory-reference-r1/oracle.json");

async fn subject(runtime: &nous_kernel::NousRuntime) -> nous_core::SubjectId {
    runtime
        .subjects
        .create_subject(nous_subject::CreateSubject {
            subject_id: None,
            operation_id: nous_core::OperationId::new(),
            cognitive_seed: nous_subject::CognitiveSeedInput {
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

fn text_query(subject: nous_core::SubjectId) -> CognitiveQuery {
    CognitiveQuery {
        api_version: nous_core::API_VERSION,
        subject,
        session: None,
        situation: Default::default(),
        targets: vec![QueryTarget::AnyRelevantCognition],
        cues: vec![Cue::Text(TextCue {
            text: "diagnostic phrase".into(),
        })],
        constraints: Default::default(),
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

fn oracle() -> Value {
    serde_json::from_str(ORACLE).expect("oracle JSON")
}

#[tokio::test]
async fn diagnostic_oracle_distinguishes_provider_unavailable_from_unknown() {
    let oracle = oracle();
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
    let expected = &oracle["diagnostics"]["dense-provider-unavailable"];
    assert_eq!(result.status, nous_core::QueryStatus::Degraded);
    assert!(
        result
            .degradation
            .iter()
            .any(|value| value.code == expected["code"].as_str().unwrap())
    );
}

#[tokio::test]
async fn diagnostic_oracle_reports_validation_budget_exhaustion() {
    let oracle = oracle();
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
    query.cues = vec![Cue::Entity(EntityCue {
        entity_ref: nous_core::EntityRef::new("entity:budget").unwrap(),
    })];
    query.result_need.limit = 8;
    query.constraints = QueryConstraints {
        valid: Some(TimeInterval {
            start: Some(Utc::now() - Duration::seconds(5)),
            end: Some(Utc::now() + Duration::seconds(5)),
        }),
        ..Default::default()
    };
    let result = runtime.query(query).await.expect("budget query");
    let expected = &oracle["diagnostics"]["validation-budget"];
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
    assert!(result.diagnostics.as_ref().is_some_and(|value| {
        value
            .candidate_counts
            .contains_key(&format!("drop_{}", expected["drop"].as_str().unwrap()))
    }));
}

#[tokio::test]
async fn diagnostic_oracle_reports_suppressed_exact_drop() {
    let oracle = oracle();
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let source = observation(&runtime, subject, "suppressed exact").await;
    let memory = runtime
        .require_memory()
        .expect("Memory")
        .form_memory(form_input(
            subject,
            source.occurrence.occurrence_id,
            nous_core::OperationId::new(),
            "suppressed exact",
        ))
        .await
        .expect("memory");
    runtime
        .require_memory()
        .expect("Memory")
        .suppress(
            subject,
            memory.object.memory_id,
            nous_core::OperationId::new(),
            memory.object.object_epoch,
        )
        .await
        .expect("suppress");
    let result = runtime
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
        .expect("suppressed query");
    let expected = &oracle["diagnostics"]["suppressed-exact"];
    assert!(result.results.is_empty());
    assert!(result.diagnostics.as_ref().is_some_and(|value| {
        value
            .candidate_counts
            .contains_key(&format!("drop_{}", expected["drop"].as_str().unwrap()))
    }));
}
