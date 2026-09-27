#[path = "test_support/mod.rs"]
mod test_support;

use chrono::Utc;
use nous_cognitive_runtime::{ContextResolver, UseFeedback, UseFeedbackEvent, UseKind};
use nous_core::{
    CognitiveQuery, CognitiveRef, EpistemicClass, OperationId, QueryTarget, ResultNeed,
    RevisionSupport, TemporalExtent, UseEventId,
};
use nous_self_domain::{
    CreateNarrativeIdentity, CreateSelfFacet, MutateSelfLifecycle, NarrativeReferenceInput,
    ReviseNarrativeIdentity, ReviseSelfFacet, SelfFacetKind, SelfLifecycleOperation,
    SelfRevisionIntent,
};
use nous_subject_core::{
    COGNITIVE_SEED_FORMAT, CognitiveSeedInput, CreateSubject, SeedAdoptionKind,
};
use test_support::{database, open_runtime, open_runtime_with_serving};

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Self lifecycle integration keeps the full vertical contract in one scenario"
)]
async fn self_facet_revision_fencing_lifecycle_and_direct_query() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({"source":"test"}),
            },
            config: serde_json::json!({}),
        })
        .await
        .expect("subject")
        .subject_id;
    let seed = runtime
        .subjects
        .latest_cognitive_seed(subject)
        .await
        .expect("seed");
    let before_facet = runtime
        .subjects
        .subject(subject)
        .await
        .expect("subject state");
    let created = runtime
        .self_cognition
        .create_facet(CreateSelfFacet {
            operation_id: OperationId::new(),
            subject,
            kind: SelfFacetKind::Identity,
            key: "primary-name".into(),
            statement: "Nous".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            producer_signature_id: None,
        })
        .await
        .expect("facet");
    assert_eq!(created.object.object_epoch, 1);
    assert_eq!(
        runtime
            .subjects
            .subject(subject)
            .await
            .expect("subject state")
            .authority_seq,
        before_facet.authority_seq + 1
    );
    let queried = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::SelfCognition],
            cues: Vec::new(),
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
        })
        .await
        .expect("Self query");
    assert_eq!(queried.results.len(), 1);
    assert_eq!(
        queried.results[0].reference,
        CognitiveRef::SelfFacetRevision(created.revision.self_facet_revision_id)
    );
    let revised = runtime
        .self_cognition
        .revise_facet(ReviseSelfFacet {
            operation_id: OperationId::new(),
            subject,
            self_facet_id: created.object.self_facet_id,
            expected_object_epoch: created.object.object_epoch,
            parent_revision_id: created.revision.self_facet_revision_id,
            revision_intent: SelfRevisionIntent::Refine,
            statement: "Nous Wave".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            producer_signature_id: None,
        })
        .await
        .expect("revision");
    assert_eq!(revised.object.object_epoch, 2);
    assert_eq!(
        runtime
            .subjects
            .subject(subject)
            .await
            .expect("subject state")
            .authority_seq,
        before_facet.authority_seq + 2
    );
    let historical = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::Exact {
                reference: CognitiveRef::SelfFacetRevision(created.revision.self_facet_revision_id),
            }],
            cues: Vec::new(),
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
        })
        .await
        .expect("historical Self query");
    assert_eq!(historical.results.len(), 1);
    assert_eq!(
        historical.results[0].representation.as_deref(),
        Some("Nous")
    );
    let stale = runtime
        .self_cognition
        .revise_facet(ReviseSelfFacet {
            operation_id: OperationId::new(),
            subject,
            self_facet_id: created.object.self_facet_id,
            expected_object_epoch: created.object.object_epoch,
            parent_revision_id: created.revision.self_facet_revision_id,
            revision_intent: SelfRevisionIntent::Refine,
            statement: "stale".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            producer_signature_id: None,
        })
        .await;
    assert!(matches!(stale, Err(nous_core::Error::Conflict(_))));
    runtime
        .self_cognition
        .lifecycle(MutateSelfLifecycle {
            operation_id: OperationId::new(),
            subject,
            reference: CognitiveRef::SelfFacet(created.object.self_facet_id),
            expected_object_epoch: revised.object.object_epoch,
            operation: SelfLifecycleOperation::Suppress,
        })
        .await
        .expect("suppress");
    let suppressed = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::SelfCognition],
            cues: Vec::new(),
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
        })
        .await
        .expect("suppressed query");
    assert!(suppressed.results.is_empty());
}

#[tokio::test]
async fn cognitive_seed_adoption_is_idempotent_and_digest_fenced() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            config: serde_json::json!({}),
        })
        .await
        .expect("subject")
        .subject_id;
    let operation_id = OperationId::new();
    let input = CognitiveSeedInput {
        text: "schema_version = 1\n\n[[self.facets]]".into(),
        format: COGNITIVE_SEED_FORMAT.into(),
        provenance: serde_json::json!({"source":"test"}),
    };
    let first = runtime
        .subjects
        .adopt_cognitive_seed(
            subject,
            operation_id,
            input.clone(),
            SeedAdoptionKind::Import,
        )
        .await
        .expect("first adoption");
    let second = runtime
        .subjects
        .adopt_cognitive_seed(subject, operation_id, input, SeedAdoptionKind::Import)
        .await
        .expect("idempotent adoption");
    assert_eq!(first.adoption.adoption_id, second.adoption.adoption_id);
    let conflict = runtime
        .subjects
        .adopt_cognitive_seed(
            subject,
            operation_id,
            CognitiveSeedInput {
                text: "schema_version = 2".into(),
                format: COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({"source":"different"}),
            },
            SeedAdoptionKind::Import,
        )
        .await;
    assert!(matches!(conflict, Err(nous_core::Error::Conflict(_))));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Narrative dependency integration keeps exact reference, revision, and purge evidence together"
)]
async fn narrative_reference_is_exact_and_source_purge_revalidates_narrative() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            config: serde_json::json!({}),
        })
        .await
        .expect("subject")
        .subject_id;
    let seed = runtime
        .subjects
        .latest_cognitive_seed(subject)
        .await
        .expect("seed");
    let facet = runtime
        .self_cognition
        .create_facet(CreateSelfFacet {
            operation_id: OperationId::new(),
            subject,
            kind: SelfFacetKind::Identity,
            key: "narrative-name".into(),
            statement: "Nous".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            producer_signature_id: None,
        })
        .await
        .expect("facet");
    let dependent = runtime
        .self_cognition
        .create_facet(CreateSelfFacet {
            operation_id: OperationId::new(),
            subject,
            kind: SelfFacetKind::Role,
            key: "dependent-role".into(),
            statement: "assistant".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::CognitionDependency(
                nous_core::CognitionDependency {
                    target_revision: CognitiveRef::SelfFacetRevision(
                        facet.revision.self_facet_revision_id,
                    ),
                    support_role: nous_core::SupportRole::Direct,
                },
            )],
            producer_signature_id: None,
        })
        .await
        .expect("dependent facet");
    let narrative = runtime
        .self_cognition
        .create_narrative(CreateNarrativeIdentity {
            operation_id: OperationId::new(),
            subject,
            key: "primary".into(),
            text: "I am Nous.".into(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            references: vec![NarrativeReferenceInput {
                target_exact_ref: CognitiveRef::SelfFacetRevision(
                    facet.revision.self_facet_revision_id,
                ),
                role: "identity".into(),
            }],
            producer_signature_id: None,
        })
        .await
        .expect("narrative");
    let revised = runtime
        .self_cognition
        .revise_narrative(ReviseNarrativeIdentity {
            operation_id: OperationId::new(),
            subject,
            narrative_identity_id: narrative.object.narrative_identity_id,
            expected_object_epoch: narrative.object.object_epoch,
            parent_revision_id: narrative.revision.narrative_identity_revision_id,
            revision_intent: SelfRevisionIntent::Refine,
            text: "I am Nous Wave.".into(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            references: narrative
                .revision
                .references
                .iter()
                .map(|reference| NarrativeReferenceInput {
                    target_exact_ref: reference.target_exact_ref.clone(),
                    role: reference.role.clone(),
                })
                .collect(),
            producer_signature_id: None,
        })
        .await
        .expect("narrative revision");
    runtime
        .self_cognition
        .lifecycle(MutateSelfLifecycle {
            operation_id: OperationId::new(),
            subject,
            reference: CognitiveRef::SelfFacet(facet.object.self_facet_id),
            expected_object_epoch: facet.object.object_epoch,
            operation: SelfLifecycleOperation::Purge,
        })
        .await
        .expect("facet purge");
    let after = runtime
        .self_cognition
        .narrative(subject, revised.object.narrative_identity_id)
        .await
        .expect("narrative remains for revalidation");
    assert!(matches!(
        after.object.integrity_state,
        nous_core::IntegrityState::RevalidationRequired
    ));
    let dependent_after = runtime
        .self_cognition
        .facet(subject, dependent.object.self_facet_id)
        .await
        .expect("dependent facet remains");
    assert!(matches!(
        dependent_after.object.integrity_state,
        nous_core::IntegrityState::RevalidationRequired
    ));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Serving and resident integration keeps one complete Self projection scenario together"
)]
async fn self_serving_projection_and_context_use_exact_revision_documents() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            config: serde_json::json!({}),
        })
        .await
        .expect("subject")
        .subject_id;
    let seed = runtime
        .subjects
        .latest_cognitive_seed(subject)
        .await
        .expect("seed");
    let facet = runtime
        .self_cognition
        .create_facet(CreateSelfFacet {
            operation_id: OperationId::new(),
            subject,
            kind: SelfFacetKind::Identity,
            key: "primary-name".into(),
            statement: "Nous".into(),
            scope: "global".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(seed.version.seed_version_id)],
            producer_signature_id: None,
        })
        .await
        .expect("facet");
    runtime
        .serving
        .refresh(subject)
        .await
        .expect("serving refresh");
    let snapshot = runtime.serving.publisher.snapshot_for(subject);
    let lexical = snapshot.lexical.as_ref().expect("lexical generation");
    let lexical_refs = lexical
        .search("Nous", 16)
        .expect("lexical search")
        .into_iter()
        .filter_map(|value| value.reference)
        .collect::<Vec<_>>();
    let exact = CognitiveRef::SelfFacetRevision(facet.revision.self_facet_revision_id);
    let session = runtime
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .expect("session");
    let (accepted, duplicates, _) = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            consumer_ref: "test:self:context".into(),
            session_id: Some(session.session_id),
            events: vec![UseFeedbackEvent {
                event_id: UseEventId::new(),
                reference: exact.clone(),
                use_kind: UseKind::Referenced,
                occurred_at: Utc::now(),
                context: serde_json::json!({}),
            }],
        })
        .await
        .expect("Self use feedback");
    assert_eq!((accepted, duplicates), (1, 0));
    let resident = runtime
        .cognition
        .session(subject, session.session_id)
        .await
        .expect("session view")
        .resident;
    assert!(resident.iter().any(|value| value.reference == exact));
    assert!(lexical_refs.contains(&exact));
    let query = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::SelfCognition],
            cues: vec![nous_core::Cue::Text(nous_core::TextCue {
                text: "Nous".into(),
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
        })
        .await
        .expect("Self lexical query");
    let hit = query
        .results
        .iter()
        .find(|value| value.reference == exact)
        .expect("Self lexical hit");
    assert!(
        hit.match_evidence
            .families
            .contains(&nous_core::EvidenceFamily::Lexical)
    );
    let context = runtime
        .context_source(subject, &exact, 1024, true)
        .await
        .expect("Self context");
    assert_eq!(context.text.as_deref(), Some("Nous"));
    runtime
        .self_cognition
        .lifecycle(MutateSelfLifecycle {
            operation_id: OperationId::new(),
            subject,
            reference: CognitiveRef::SelfFacet(facet.object.self_facet_id),
            expected_object_epoch: facet.object.object_epoch,
            operation: SelfLifecycleOperation::Purge,
        })
        .await
        .expect("Self purge");
    runtime
        .serving
        .refresh(subject)
        .await
        .expect("purge refresh");
    let refreshed = runtime.serving.publisher.snapshot_for(subject);
    let remaining = refreshed
        .lexical
        .as_ref()
        .expect("refreshed lexical generation")
        .search("Nous", 16)
        .expect("refreshed lexical search")
        .into_iter()
        .filter_map(|value| value.reference)
        .collect::<Vec<_>>();
    assert!(!remaining.contains(&exact));
}
