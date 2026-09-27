#[path = "test_support/mod.rs"]
mod test_support;

use nous_subject_core::{CharacterSeedInput, CreateSubject};
use test_support::{database, form_input, observation, open_runtime};

#[tokio::test]
async fn purge_keeps_shared_observation_authority() {
    let (_postgres, url, root) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            character_seed: CharacterSeedInput {
                text: "shared source".into(),
                media_type: "text/plain".into(),
                provenance: serde_json::json!({}),
            },
            config: serde_json::json!({}),
        })
        .await
        .expect("subject")
        .subject_id;
    let source = observation(&runtime, subject, "shared source material").await;
    let first = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            source.occurrence.occurrence_id,
            nous_core::OperationId::new(),
            "first cognition",
        ))
        .await
        .expect("first memory");
    let second = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            source.occurrence.occurrence_id,
            nous_core::OperationId::new(),
            "second cognition",
        ))
        .await
        .expect("second memory");
    let artifact_id = source.artifact.as_ref().expect("artifact").artifact_id;
    runtime
        .require_memory()
        .unwrap()
        .purge_memory(
            subject,
            first.object.memory_id,
            nous_core::OperationId::new(),
            first.object.object_epoch,
        )
        .await
        .expect("purge first cognition");
    assert!(
        runtime
            .require_memory()
            .unwrap()
            .memory(subject, second.object.memory_id, None)
            .await
            .is_ok()
    );
    assert!(
        runtime
            .material
            .artifact(subject, artifact_id)
            .await
            .is_ok()
    );
}
