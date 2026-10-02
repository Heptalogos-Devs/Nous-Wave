#[path = "test_support/mod.rs"]
mod test_support;

use chrono::{Duration, Utc};
use nous_core::{CognitiveRef, EntityRef, OperationId};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, ResolvedEntityMention,
    RuntimeDirective,
};
use nous_runtime::{ResidentAdmission, ResidentState};
use nous_subject::{CognitiveSeedInput, CreateSubject};
use test_support::{database, observation, open_runtime};
use uuid::Uuid;

async fn subject(runtime: &nous_kernel::NousRuntime) -> nous_core::SubjectId {
    runtime
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
        .expect("subject")
        .subject_id
}

#[tokio::test]
async fn observation_admission_is_a_single_idempotent_runtime_batch() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let session = runtime
        .cognition
        .open_session(subject, serde_json::json!({"consumer":"a"}))
        .await
        .expect("session");
    let entity = EntityRef::new("entity:alice").expect("entity");
    let request_id = Uuid::new_v4();
    let input = ObservationInput {
        subject,
        session: Some(session.session_id),
        occurrence: OccurrenceDescriptor {
            source_class: nous_core::SourceClass::Message,
            external_object_ref: None,
            occurred_time: nous_core::TemporalExtent::Unknown,
            observed_at: Some(Utc::now()),
            conversation_ref: None,
            actor_entity_ref: None,
            context: serde_json::json!({}),
        },
        material: ObservationMaterial::InlineText {
            text: "Alice was present".into(),
            media_type: "text/plain".into(),
        },
        entities: vec![ResolvedEntityMention {
            surface: "Alice".into(),
            entity_ref: Some(entity.clone()),
            semantic_role: Some("subject".into()),
        }],
        runtime: RuntimeDirective::default(),
    };
    let first = runtime
        .material
        .record_observation_once(input.clone(), Some(request_id))
        .await
        .expect("first observation");
    let after_first = runtime
        .cognition
        .session(subject, session.session_id)
        .await
        .expect("resident session");
    assert_eq!(after_first.runtime_revision, 1);
    assert_eq!(after_first.resident.len(), 2);
    assert!(
        after_first.resident.iter().any(
            |value| value.reference == CognitiveRef::Occurrence(first.occurrence.occurrence_id)
        )
    );
    assert!(
        after_first
            .resident
            .iter()
            .any(|value| value.reference == CognitiveRef::Entity(entity.clone()))
    );

    let retry = runtime
        .material
        .record_observation_once(input, Some(request_id))
        .await
        .expect("idempotent observation retry");
    let after_retry = runtime
        .cognition
        .session(subject, session.session_id)
        .await
        .expect("resident retry session");
    assert_eq!(
        retry.occurrence.occurrence_id,
        first.occurrence.occurrence_id
    );
    assert_eq!(after_retry.runtime_revision, after_first.runtime_revision);
    assert_eq!(after_retry.resident.len(), after_first.resident.len());
}

#[tokio::test]
async fn resident_batch_is_atomic_and_invalid_persisted_refs_are_visible() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject_a = subject(&runtime).await;
    let subject_b = subject(&runtime).await;
    let session = runtime
        .cognition
        .open_session(subject_a, serde_json::json!({}))
        .await
        .expect("session");
    let foreign = observation(&runtime, subject_b, "foreign").await;
    let result = runtime
        .cognition
        .admit_batch(
            subject_a,
            session.session_id,
            vec![ResidentAdmission {
                reference: CognitiveRef::Occurrence(foreign.occurrence.occurrence_id),
                reason: "foreign".into(),
                hold_until: None,
                state: ResidentState::Resident,
            }],
        )
        .await;
    assert!(matches!(
        result,
        Err(nous_core::Error::FailedPrecondition(_))
    ));
    let clean = runtime
        .cognition
        .session(subject_a, session.session_id)
        .await
        .expect("clean session");
    assert_eq!(clean.runtime_revision, 0);
    assert!(clean.resident.is_empty());

    sqlx::query("INSERT INTO resident_refs(session_id,ref_kind,ref_value,entered_at,entry_reason,state,metadata) VALUES($1,'unknown_ref','value',$2,'corrupt','resident','{}')")
        .bind(session.session_id.0)
        .bind(Utc::now())
        .execute(runtime.store.pool())
        .await
        .expect("persist invalid ref");
    let result = runtime
        .cognition
        .session(subject_a, session.session_id)
        .await;
    assert!(matches!(result, Err(nous_core::Error::Invalid(_))));
}

#[tokio::test]
async fn resident_noop_preserves_hold_and_revision_and_eviction_is_one_revision() {
    let (root, url, _postgres) = database().await;
    let mut runtime = open_runtime(&url, &root).await;
    let subject = subject(&runtime).await;
    let session = runtime
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .expect("session");
    let first = observation(&runtime, subject, "first").await;
    let hold = Utc::now() + Duration::hours(1);
    let first_admission = ResidentAdmission {
        reference: CognitiveRef::Occurrence(first.occurrence.occurrence_id),
        reason: "observed".into(),
        hold_until: Some(hold),
        state: ResidentState::Resident,
    };
    let inserted = runtime
        .cognition
        .admit_batch(subject, session.session_id, vec![first_admission.clone()])
        .await
        .expect("first admission");
    assert!(inserted.changed);
    assert_eq!(inserted.runtime_revision, 1);
    let no_op = runtime
        .cognition
        .admit_batch(
            subject,
            session.session_id,
            vec![ResidentAdmission {
                hold_until: Some(Utc::now()),
                ..first_admission
            }],
        )
        .await
        .expect("no-op admission");
    assert!(!no_op.changed);
    assert_eq!(no_op.runtime_revision, 1);
    let resident = runtime
        .cognition
        .session(subject, session.session_id)
        .await
        .expect("resident after no-op")
        .resident;
    assert_eq!(
        resident[0].hold_until.map(|value| value.timestamp_micros()),
        Some(hold.timestamp_micros())
    );

    runtime.cognition.resident_limit = 1;
    runtime.material.cognition.resident_limit = 1;
    let second = observation(&runtime, subject, "second").await;
    let eviction = runtime
        .cognition
        .admit_batch(
            subject,
            session.session_id,
            vec![ResidentAdmission {
                reference: CognitiveRef::Occurrence(second.occurrence.occurrence_id),
                reason: "observed".into(),
                hold_until: None,
                state: ResidentState::Resident,
            }],
        )
        .await
        .expect("eviction admission");
    assert_eq!(eviction.evicted_count, 1);
    assert_eq!(eviction.runtime_revision, 2);
    assert_eq!(
        runtime
            .cognition
            .session(subject, session.session_id)
            .await
            .expect("evicted session")
            .resident
            .len(),
        1
    );
}
