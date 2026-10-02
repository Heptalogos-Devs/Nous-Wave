mod test_support;

use chrono::{Duration, SubsecRound, Utc};
use nous_core::{SourceClass, TemporalExtent};
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, RuntimeDirective,
};
use nous_retrieval::ServingOptions;
use nous_runtime::{
    CognitiveClock, MaintenanceDisposition, MaintenanceRequest, ManualCognitiveClock,
};
use sqlx::Row;
use std::sync::Arc;
use test_support::database;

async fn create_subject(runtime: &NousRuntime) -> nous_core::SubjectId {
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
        .expect("create Subject")
        .subject_id
}

async fn runtime_with_clock(
    url: &str,
    root: &tempfile::TempDir,
    clock: Arc<dyn CognitiveClock>,
) -> NousRuntime {
    NousRuntime::open_with_clock(RuntimeOptions {
        postgres_url: url.into(), max_connections: 8,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        serving_options: ServingOptions { root: root.path().join("serving"), lexical: false, dense: false, topology: false, memory_enabled: true },
        embedding: None, stored_embedding: None,
        deployment_settings: serde_json::json!({"settings":{"serving":{"lexical":{"enabled":false},"dense":{"enabled":false},"topology":{"enabled":false}}}}),
    }, clock).await.expect("open clock-injected runtime")
}

fn observation(
    subject: nous_core::SubjectId,
    session: Option<nous_core::SessionId>,
) -> ObservationInput {
    ObservationInput {
        subject,
        session,
        occurrence: OccurrenceDescriptor {
            source_class: SourceClass::Message,
            external_object_ref: None,
            occurred_time: TemporalExtent::Unknown,
            observed_at: None,
            conversation_ref: None,
            actor_entity_ref: None,
            context: serde_json::json!({}),
        },
        material: ObservationMaterial::InlineText {
            text: "durable experience".into(),
            media_type: "text/plain".into(),
        },
        entities: vec![],
        runtime: RuntimeDirective::default(),
    }
}

#[tokio::test]
async fn experience_capture_cursor_idle_and_reopen_are_stable() {
    let (root, url, _postgres) = database().await;
    let start = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(start));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let first_session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let input = observation(subject, Some(first_session.session_id));
    let key = uuid::Uuid::new_v4();
    let accepted = rt
        .material
        .record_observation_once(input.clone(), Some(key))
        .await
        .unwrap();
    clock.advance_by(subject, Duration::seconds(30)).unwrap();
    let replay = rt
        .material
        .record_observation_once(input, Some(key))
        .await
        .unwrap();
    assert_eq!(
        accepted.occurrence.occurrence_id,
        replay.occurrence.occurrence_id
    );
    assert_eq!(replay.occurrence.observed_at, start);
    rt.material
        .record_observation(observation(subject, None))
        .await
        .unwrap();
    let second_session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    rt.material
        .record_observation(observation(subject, Some(second_session.session_id)))
        .await
        .unwrap();
    let rows = sqlx::query(
        "SELECT recorded_seq FROM experience_items WHERE subject_id=$1 ORDER BY recorded_seq",
    )
    .bind(subject.0)
    .fetch_all(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[1].get::<i64, _>("recorded_seq") > rows[0].get::<i64, _>("recorded_seq") + 1);
    let progress = rt
        .cognition
        .segment_experience(subject, "interaction", 128, false)
        .await
        .unwrap();
    assert_eq!(progress.processed_count, 2);
    assert!(progress.ready.is_empty());
    assert!(progress.next_due.is_some());
    let repeated = rt
        .cognition
        .segment_experience(subject, "interaction", 128, false)
        .await
        .unwrap();
    assert_eq!(repeated.processed_count, 0);
    clock.advance_by(subject, Duration::minutes(30)).unwrap();
    drop(rt);
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let progress = rt
        .cognition
        .segment_experience(subject, "interaction", 128, false)
        .await
        .unwrap();
    assert_eq!(progress.ready.len(), 1);
    assert_eq!(progress.ready[0].members.len(), 2);
    assert_eq!(
        progress.ready[0].boundary_reason.as_deref(),
        Some("hard_idle")
    );
    assert_eq!(progress.processed_count, 0);
    let organized = rt.organize_experience(subject, 128, false).await.unwrap();
    assert_eq!(organized.episodes.len(), 1);
    assert_eq!(organized.episodes[0].revision.formed_at, clock.now(subject));
    assert!(
        rt.organize_experience(subject, 128, false)
            .await
            .unwrap()
            .episodes
            .is_empty()
    );
    assert!(
        rt.cognition
            .maintenance_needs(subject)
            .await
            .unwrap()
            .iter()
            .any(|need| need.kind == "journal_review")
    );
    let input = test_support::form_input(
        subject,
        accepted.occurrence.occurrence_id,
        nous_core::OperationId::new(),
        "owner-timed grounded memory",
    );
    let at = clock.now(subject);
    let memory = rt
        .require_memory()
        .unwrap()
        .form_memory(input.clone())
        .await
        .unwrap();
    assert_eq!(memory.revision.formed_at, at);
    assert_eq!(memory.revision.recorded_at, at);
    clock.advance_by(subject, Duration::days(1)).unwrap();
    rt.require_memory()
        .unwrap()
        .revise_memory(nous_memory::ReviseMemoryInput {
            producer: None,
            operation_id: nous_core::OperationId::new(),
            subject,
            memory_id: memory.object.memory_id,
            expected_object_epoch: memory.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            formation_mode: input.formation_mode,
            grounding_occurrence_id: input.grounding_occurrence_id,
            semantic_role: input.semantic_role.clone(),
            representation_text: "corrected owner-timed memory".into(),
            title: None,
            supports: input.supports.clone(),
            aboutness: vec![],
            valid_time: TemporalExtent::Unknown,
            epistemic_class: input.epistemic_class,
        })
        .await
        .unwrap();
    let replay = rt
        .require_memory()
        .unwrap()
        .form_memory(input)
        .await
        .unwrap();
    assert_eq!(
        replay.revision.memory_revision_id,
        memory.revision.memory_revision_id
    );
    assert_eq!(replay.revision.formed_at, at);
    assert_eq!(replay.revision.recorded_at, at);
}

#[tokio::test]
async fn maintenance_claims_use_execution_leases_and_keep_new_triggers() {
    let (root, url, _postgres) = database().await;
    let start = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(start));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let request = MaintenanceRequest {
        subject,
        kind: "journal_review".into(),
        scope_kind: "track".into(),
        scope_ref: "interaction".into(),
        trigger_authority_seq: 1,
        due_at: start + Duration::hours(1),
        priority: 30,
    };
    let id = rt
        .cognition
        .enqueue_maintenance(request.clone())
        .await
        .unwrap();
    let kinds = vec!["journal_review".into()];
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60)
            .await
            .unwrap()
            .is_empty()
    );
    clock.advance_by(subject, Duration::hours(1)).unwrap();
    let (a, b) = tokio::join!(
        rt.cognition.lease_maintenance(subject, &kinds, 1, 60),
        rt.cognition.lease_maintenance(subject, &kinds, 1, 60)
    );
    let leased: Vec<_> = a.unwrap().into_iter().chain(b.unwrap()).collect();
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].need_id, id);
    clock.advance_by(subject, Duration::days(30)).unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60)
            .await
            .unwrap()
            .is_empty()
    );
    let newer = MaintenanceRequest {
        trigger_authority_seq: 2,
        ..request
    };
    assert_eq!(rt.cognition.enqueue_maintenance(newer).await.unwrap(), id);
    rt.cognition
        .acknowledge_maintenance(&leased[0], MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    let reclaimed = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60)
        .await
        .unwrap();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].trigger_authority_seq, 2);
    sqlx::query("UPDATE maintenance_needs SET lease_until=clock_timestamp()-interval '1 second' WHERE need_id=$1")
        .bind(id).execute(rt.store.pool()).await.unwrap();
    let retry = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60)
        .await
        .unwrap();
    assert_eq!(retry[0].attempt_count, 3);
    assert!(
        rt.cognition
            .acknowledge_maintenance(&reclaimed[0], MaintenanceDisposition::Satisfied)
            .await
            .is_err()
    );
    rt.cognition
        .acknowledge_maintenance(&retry[0], MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    assert!(
        rt.cognition
            .maintenance_needs(subject)
            .await
            .unwrap()
            .is_empty()
    );
}
