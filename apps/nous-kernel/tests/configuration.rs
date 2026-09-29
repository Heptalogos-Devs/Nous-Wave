#[path = "test_support/mod.rs"]
mod test_support;

use nous_configuration::{ConfigActorTier, ConfigKey, SUBJECT_DEFAULT_MEMORY};
use nous_core::OperationId;
use nous_subject::{CognitiveSeedInput, CreateSubject};
use serde_json::json;
use test_support::{database, open_runtime};

const EPSILON: ConfigKey<f64> = ConfigKey::new("memory.accessibility.epsilon");
const TAU_DAYS: ConfigKey<f64> = ConfigKey::new("memory.accessibility.tau_days");

#[tokio::test]
async fn configuration_precedence_permissions_and_restart_state_are_explicit() {
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
                provenance: json!({}),
            },
            metadata: json!({}),
            capabilities: None,
        })
        .await
        .expect("subject")
        .subject_id;
    let service = &runtime.configuration;
    assert_eq!(
        service
            .snapshot_for_subject(subject)
            .unwrap()
            .get(EPSILON)
            .unwrap(),
        0.02
    );
    service
        .set_system_override(
            OperationId::new(),
            EPSILON.path(),
            json!(0.04),
            ConfigActorTier::Developer,
        )
        .await
        .expect("system override");
    assert_eq!(
        service
            .snapshot_for_subject(subject)
            .unwrap()
            .get(EPSILON)
            .unwrap(),
        0.04
    );
    service
        .set_subject_override(
            OperationId::new(),
            subject,
            EPSILON.path(),
            json!(0.08),
            ConfigActorTier::AdvancedUser,
        )
        .await
        .expect("subject override");
    assert_eq!(
        service
            .snapshot_for_subject(subject)
            .unwrap()
            .get(EPSILON)
            .unwrap(),
        0.08
    );
    assert!(
        service
            .set_subject_override(
                OperationId::new(),
                subject,
                EPSILON.path(),
                json!(0.09),
                ConfigActorTier::StandardUser,
            )
            .await
            .is_err()
    );

    let active = service.active_system_snapshot().unwrap();
    let outcome = service
        .set_system_override(
            OperationId::new(),
            "runtime.resident_limit",
            json!(512),
            ConfigActorTier::Developer,
        )
        .await
        .expect("restart override");
    assert!(outcome.pending_restart);
    assert_eq!(
        active.get_path::<usize>("runtime.resident_limit").unwrap(),
        256
    );
    assert_eq!(
        service
            .desired_system_snapshot()
            .unwrap()
            .get_path::<usize>("runtime.resident_limit")
            .unwrap(),
        512
    );
    assert!(
        service
            .snapshot_for_subject(subject)
            .unwrap()
            .get(SUBJECT_DEFAULT_MEMORY)
            .unwrap(),
    );
}

#[tokio::test]
async fn configuration_receipts_freeze_subject_scope_and_replay_outcomes() {
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
                provenance: json!({}),
            },
            metadata: json!({}),
            capabilities: None,
        })
        .await
        .expect("subject")
        .subject_id;
    let service = &runtime.configuration;
    let first_id = OperationId::new();
    let first = service
        .set_subject_override(
            first_id,
            subject,
            EPSILON.path(),
            json!(0.08),
            ConfigActorTier::AdvancedUser,
        )
        .await
        .expect("first subject override");
    assert_eq!(
        first.active_digest,
        service
            .snapshot_for_subject(subject)
            .unwrap()
            .effective_digest
    );
    assert_eq!(first.active_digest, first.desired_digest);
    assert_ne!(
        first.active_digest,
        service.active_system_snapshot().unwrap().effective_digest
    );

    let later = service
        .set_subject_override(
            OperationId::new(),
            subject,
            EPSILON.path(),
            json!(0.09),
            ConfigActorTier::AdvancedUser,
        )
        .await
        .expect("later subject override");
    let replay = service
        .set_subject_override(
            first_id,
            subject,
            EPSILON.path(),
            json!(0.08),
            ConfigActorTier::AdvancedUser,
        )
        .await
        .expect("replay original subject override");
    assert_eq!(replay.revision, first.revision);
    assert_eq!(replay.active_digest, first.active_digest);
    assert_eq!(replay.desired_digest, first.desired_digest);
    assert_ne!(later.active_digest, replay.active_digest);
    assert!(matches!(
        service
            .set_subject_override(
                first_id,
                subject,
                EPSILON.path(),
                json!(0.1),
                ConfigActorTier::AdvancedUser,
            )
            .await,
        Err(nous_core::Error::Conflict(_))
    ));
}

#[tokio::test]
async fn concurrent_configuration_mutations_publish_one_complete_snapshot() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let service = runtime.configuration.clone();
    let first_id = OperationId::new();
    let second_id = OperationId::new();
    let (first, second) = tokio::join!(
        service.set_system_override(
            first_id,
            EPSILON.path(),
            json!(0.11),
            ConfigActorTier::Developer,
        ),
        service.set_system_override(
            second_id,
            TAU_DAYS.path(),
            json!(42.0),
            ConfigActorTier::Developer,
        )
    );
    first.expect("epsilon mutation");
    second.expect("tau mutation");
    let snapshot = service.active_system_snapshot().expect("active snapshot");
    assert_eq!(snapshot.get(EPSILON).expect("epsilon"), 0.11);
    assert_eq!(snapshot.get(TAU_DAYS).expect("tau"), 42.0);
    assert!(
        service
            .set_system_override(
                first_id,
                EPSILON.path(),
                json!(0.12),
                ConfigActorTier::Developer,
            )
            .await
            .is_err()
    );
}
