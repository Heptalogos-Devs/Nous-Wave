#[path = "test_support/mod.rs"]
mod test_support;

use chrono::{Duration, Utc};
use nous_core::{
    CognitiveQuery, CognitiveRef, EpistemicClass, QueryTarget, ResultNeed, TemporalExtent,
    TimeInterval,
};
use nous_material::{ObservationInput, ObservationMaterial, OccurrenceDescriptor};
use nous_memory_domain::{
    AcceptanceState, AccessibilityMode, CognitiveRole, EvidenceLocator, EvidenceRef,
    ExplicitMemoryInput, FormationMode, SupportRole, SuppressionState,
};
use nous_subject_core::{CognitiveSeedInput, CreateSubject};
use test_support::{database, open_runtime};

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "qualification edge test keeps temporal and lifecycle assertions together"
)]
async fn temporal_formation_and_lifecycle_contracts() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: nous_core::OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: None,
        })
        .await
        .expect("subject")
        .subject_id;
    let start = Utc::now() - Duration::hours(2);
    let end = start + Duration::hours(1);
    let occurrence = runtime
        .material
        .record_observation(ObservationInput {
            subject,
            session: None,
            occurrence: OccurrenceDescriptor {
                source_class: nous_core::SourceClass::Message,
                external_object_ref: None,
                occurred_time: TemporalExtent::Interval {
                    start: Some(start),
                    end: Some(end),
                },
                observed_at: Utc::now(),
                conversation_ref: None,
                actor_entity_ref: None,
                context: serde_json::json!({}),
            },
            material: ObservationMaterial::InlineText {
                text: "interval fact".into(),
                media_type: "text/plain".into(),
            },
            entities: Vec::new(),
            runtime: Default::default(),
        })
        .await
        .expect("interval observation");
    let formed_at = Utc::now() - Duration::minutes(1);
    let memory = runtime
        .require_memory()
        .unwrap()
        .form_memory(ExplicitMemoryInput {
            operation_id: nous_core::OperationId::new(),
            subject,
            cognitive_role: CognitiveRole::Declarative,
            formation_mode: FormationMode::Grounded,
            grounding_occurrence_id: Some(occurrence.occurrence.occurrence_id),
            semantic_role: "fact".into(),
            representation_text: "interval fact".into(),
            title: None,
            supports: vec![nous_memory_domain::RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: occurrence.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            aboutness: Vec::new(),
            tags: Vec::new(),
            valid_time: TemporalExtent::Interval {
                start: Some(start),
                end: Some(end),
            },
            formed_at,
            epistemic_class: EpistemicClass::Observed,
        })
        .await
        .expect("form interval memory");
    assert!(memory.revision.recorded_at >= memory.revision.formed_at);
    assert!(matches!(
        memory.temporal_evidence.occurred[0],
        TemporalExtent::Interval { .. }
    ));
    assert!(
        !TemporalExtent::Interval {
            start: Some(start),
            end: Some(end),
        }
        .overlaps_interval(&TimeInterval {
            start: Some(end),
            end: Some(end + Duration::minutes(1)),
        })
    );
    let deep = runtime
        .require_memory()
        .unwrap()
        .set_accessibility(
            subject,
            memory.object.memory_id,
            nous_core::OperationId::new(),
            memory.object.object_epoch,
            AccessibilityMode::Deep,
        )
        .await
        .expect("set accessibility");
    assert_eq!(deep.object.object_epoch, memory.object.object_epoch + 1);
    assert_eq!(deep.revision.revision_no, 1);
    let withdrawn = runtime
        .require_memory()
        .unwrap()
        .withdraw(
            subject,
            memory.object.memory_id,
            nous_core::OperationId::new(),
            deep.object.object_epoch,
        )
        .await
        .expect("withdraw");
    assert_eq!(
        withdrawn.object.acceptance_state,
        AcceptanceState::Withdrawn
    );
    assert_eq!(withdrawn.object.suppression_state, SuppressionState::Normal);
    let reaccepted = runtime
        .require_memory()
        .unwrap()
        .reaccept(
            subject,
            memory.object.memory_id,
            nous_core::OperationId::new(),
            withdrawn.object.object_epoch,
        )
        .await
        .expect("reaccept");
    assert_eq!(
        reaccepted.object.acceptance_state,
        AcceptanceState::Accepted
    );
    let query = CognitiveQuery {
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
    };
    assert_eq!(
        runtime
            .query(query)
            .await
            .expect("exact query")
            .results
            .len(),
        1
    );
}
