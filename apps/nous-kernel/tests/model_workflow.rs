#[path = "test_support/mod.rs"]
mod test_support;
use nous_core::OperationId;
use nous_subject::{CognitiveSeedInput, CreateSubject};

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one workflow scenario covers reservation concurrency, proposal recovery and terminal replay"
)]
async fn workflow_reservation_conflict_proposal_resume_and_outcome_replay() {
    let (root, url, _postgres) = test_support::database().await;
    let runtime = test_support::open_runtime(&url, &root).await;
    let subject = runtime
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
        .unwrap()
        .subject_id;
    let key = OperationId::new().0.to_string();
    let first = runtime
        .store
        .reserve_model_workflow(
            subject,
            "memory",
            &key,
            "same input",
            &serde_json::json!({"model":"original"}),
            360,
        )
        .await
        .unwrap();
    let token = first.lease_token.unwrap();
    let concurrent = runtime
        .store
        .reserve_model_workflow(
            subject,
            "memory",
            &key,
            "same input",
            &serde_json::json!({"model":"changed"}),
            360,
        )
        .await
        .unwrap();
    assert!(concurrent.busy);
    assert!(concurrent.lease_token.is_none());
    assert!(
        runtime
            .store
            .reserve_model_workflow(
                subject,
                "memory",
                &key,
                "different input",
                &serde_json::json!({}),
                360
            )
            .await
            .is_err()
    );
    runtime
        .store
        .save_model_workflow(
            subject,
            "memory",
            &key,
            token,
            Some(&serde_json::json!({"text":"validated proposal"})),
            None,
        )
        .await
        .unwrap();
    runtime
        .store
        .release_model_workflow(subject, "memory", &key, token)
        .await
        .unwrap();
    let resumed = runtime
        .store
        .reserve_model_workflow(
            subject,
            "memory",
            &key,
            "same input",
            &serde_json::json!({"model":"changed"}),
            360,
        )
        .await
        .unwrap();
    assert_eq!(resumed.snapshot, serde_json::json!({"model":"original"}));
    assert!(resumed.proposal.is_some());
    let outcome = serde_json::json!({"revision":"receipt reference"});
    runtime
        .store
        .save_model_workflow(
            subject,
            "memory",
            &key,
            resumed.lease_token.unwrap(),
            None,
            Some(&outcome),
        )
        .await
        .unwrap();
    let completed = runtime
        .store
        .reserve_model_workflow(
            subject,
            "memory",
            &key,
            "same input",
            &serde_json::json!({}),
            360,
        )
        .await
        .unwrap();
    assert_eq!(completed.outcome, Some(outcome));
    assert!(completed.proposal.is_none());
    assert!(completed.lease_token.is_none());
    assert_eq!(completed.snapshot, serde_json::json!({}));
    assert_owner_validation(&runtime.store, subject, &key).await;
}

async fn assert_owner_validation(
    store: &nous_persistence::AuthorityStore,
    subject: nous_core::SubjectId,
    key: &str,
) {
    use nous_persistence::WorkflowOwner;
    let domain = WorkflowOwner::new("test_domain_42").unwrap();
    assert_eq!(domain.as_str(), "test_domain_42");
    let generalized = store
        .reserve_model_workflow(
            subject,
            domain.as_str(),
            key,
            "domain input",
            &serde_json::json!({}),
            360,
        )
        .await
        .unwrap();
    assert!(generalized.lease_token.is_some());
    for owner in [
        "",
        "Memory",
        "1domain",
        "domain-name",
        "领域",
        &"a".repeat(65),
    ] {
        assert!(WorkflowOwner::new(owner).is_err());
        assert!(
            store
                .reserve_model_workflow(subject, owner, key, "input", &serde_json::json!({}), 360)
                .await
                .is_err()
        );
        assert!(sqlx::query("INSERT INTO model_workflow_operations(subject_id,owner,operation_key,semantic_digest,snapshot) VALUES($1,$2,'invalid','input','{}')").bind(subject.0).bind(owner).execute(store.pool()).await.is_err());
    }
}
