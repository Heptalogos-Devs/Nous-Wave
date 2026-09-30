#[path = "test_support/mod.rs"]
mod test_support;

use chrono::Utc;
use nous_core::{
    CapabilityOperation, CognitiveRef, DerivedRepresentationId, ProducerSignature,
    RepresentationKind,
};
use nous_material::{DerivationInput, DerivedRepresentation};
use nous_subject::{CognitiveSeedInput, CreateSubject};

#[tokio::test]
async fn ordered_derivation_graph_preserves_roots_and_reuses_success() {
    let (root, url, _postgres) = test_support::database().await;
    let runtime = test_support::open_runtime(&url, &root).await;
    let subject = runtime
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
        .unwrap()
        .subject_id;
    let observed = test_support::observation(&runtime, subject, "source facts").await;
    let source = observed.source_region.unwrap().source_region_id;
    let make = |input: CognitiveRef, kind| DerivedRepresentation {
        derived_representation_id: DerivedRepresentationId::new(),
        subject_id: subject,
        inputs: vec![DerivationInput {
            ordinal: 0,
            reference: input,
            role: "source".into(),
        }],
        strategy: "describe_then_structure".into(),
        representation_kind: kind,
        producer: ProducerSignature {
            signature_hash: "untrusted supplied hash".into(),
            provider_class: "deterministic-local-test".into(),
            operation: CapabilityOperation::TextInterpretation,
            implementation: "material-graph-test".into(),
            model_identity: None,
            model_revision: None,
            preprocessing_identity: "test".into(),
            preprocessing_revision: "1".into(),
            config_digest: "test-config".into(),
        },
        revision: 1,
        payload_text: Some("faithful representation".into()),
        payload_artifact_id: None,
        quality: serde_json::json!({}),
        created_at: Utc::now(),
        supersedes: None,
    };
    let description = runtime
        .material
        .persist_derived_representation(make(
            CognitiveRef::SourceRegion(source),
            RepresentationKind::Summary,
        ))
        .await
        .unwrap();
    assert_ne!(
        description.producer.signature_hash,
        "untrusted supplied hash"
    );
    let structured = runtime
        .material
        .persist_derived_representation(make(
            CognitiveRef::DerivedRepresentation(description.derived_representation_id),
            RepresentationKind::StructuredInterpretation,
        ))
        .await
        .unwrap();
    let roots: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT source_region_id FROM representation_source_regions($1,$2)")
            .bind(subject.0)
            .bind(structured.derived_representation_id.0)
            .fetch_all(runtime.store.pool())
            .await
            .unwrap();
    assert_eq!(roots, vec![source.0]);
    let mut retry = make(
        CognitiveRef::SourceRegion(source),
        RepresentationKind::Summary,
    );
    retry.payload_text = Some("different stochastic retry output".into());
    let retried = runtime
        .material
        .persist_derived_representation(retry)
        .await
        .unwrap();
    assert_eq!(
        retried.derived_representation_id,
        description.derived_representation_id
    );
    assert_eq!(retried.payload_text, description.payload_text);
    let mut duplicate = make(
        CognitiveRef::SourceRegion(source),
        RepresentationKind::Summary,
    );
    duplicate.inputs.push(DerivationInput {
        ordinal: 1,
        reference: CognitiveRef::SourceRegion(source),
        role: "duplicate".into(),
    });
    assert!(
        runtime
            .material
            .persist_derived_representation(duplicate)
            .await
            .is_err()
    );
    let mut cycle = make(
        CognitiveRef::SourceRegion(source),
        RepresentationKind::Summary,
    );
    cycle.inputs[0].reference =
        CognitiveRef::DerivedRepresentation(cycle.derived_representation_id);
    assert!(
        runtime
            .material
            .persist_derived_representation(cycle)
            .await
            .is_err()
    );
    let mut invalid = make(
        CognitiveRef::SourceRegion(source),
        RepresentationKind::Summary,
    );
    invalid.inputs[0].reference =
        CognitiveRef::DerivedRepresentation(DerivedRepresentationId::new());
    assert!(
        runtime
            .material
            .persist_derived_representation(invalid)
            .await
            .is_err()
    );
}
