#[path = "test_support/mod.rs"]
mod test_support;

use nous_configuration_service::{ConfigActorTier, ConfigKey, SUBJECT_DEFAULT_MEMORY};
use nous_core::OperationId;
use nous_subject_core::{CognitiveSeedInput, CreateSubject};
use serde_json::json;
use test_support::{database, open_runtime};

const EPSILON: ConfigKey<f64> = ConfigKey::new("memory.accessibility.epsilon");

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
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
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
