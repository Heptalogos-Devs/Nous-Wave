mod test_support;

use chrono::Utc;
use nous_persistence::{ProjectionInvalidation, ServingRecord};
use nous_runtime::{UseFeedback, UseFeedbackEvent, UseKind};
use nous_core::{CognitiveRef, EpistemicClass, OperationId, ServingGenerationId, TemporalExtent};
use nous_memory::*;
use nous_subject::{CognitiveSeedInput, CreateSubject};
use test_support::{
    database, external_observation, form_input, observation, occurrence_only_observation,
    open_runtime,
};

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Reference profile qualification exercises the complete vertical contract"
)]
async fn reference_profile_authority_runtime_and_purge_contracts() {
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
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: None,
        })
        .await
        .expect("subject")
        .subject_id;
    let desired = runtime
        .store
        .invalidate(subject, ProjectionInvalidation::text())
        .await
        .expect("watermark");
    let stale = ServingRecord {
        generation_id: ServingGenerationId::new(),
        subject,
        family: "lexical".into(),
        space: String::new(),
        authority_watermark: desired - 1,
        implementation_id: "test".into(),
        implementation_revision: "1".into(),
        config_digest: "test".into(),
        artifact_location: "test".into(),
        artifact_hash: "test".into(),
        built_at: Utc::now(),
        metadata: serde_json::json!({}),
    };
    runtime
        .store
        .publish_generation(stale)
        .await
        .expect("stale generation record");
    assert!(
        runtime
            .store
            .serving_current(subject)
            .await
            .expect("current serving")
            .iter()
            .all(|record| record.family != "lexical")
    );
    let current = ServingRecord {
        generation_id: ServingGenerationId::new(),
        subject,
        family: "lexical".into(),
        space: String::new(),
        authority_watermark: desired,
        implementation_id: "test".into(),
        implementation_revision: "1".into(),
        config_digest: "test".into(),
        artifact_location: "test".into(),
        artifact_hash: "test".into(),
        built_at: Utc::now(),
        metadata: serde_json::json!({}),
    };
    runtime
        .store
        .publish_generation(current)
        .await
        .expect("current generation record");
    assert!(
        runtime
            .store
            .serving_current(subject)
            .await
            .expect("current serving")
            .iter()
            .any(|record| record.family == "lexical")
    );
    let first_observation = observation(&runtime, subject, "Alice lives in Paris").await;
    let repeated_observation = observation(&runtime, subject, "Alice lives in Paris").await;
    let second_observation = observation(&runtime, subject, "Alice lives in Lyon").await;
    assert_eq!(
        first_observation
            .artifact
            .as_ref()
            .map(|value| value.artifact_id),
        repeated_observation
            .artifact
            .as_ref()
            .map(|value| value.artifact_id)
    );
    let same_root_supports = vec![
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: first_observation.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: repeated_observation.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Corroborating,
        }),
    ];
    let same_root_summary = runtime
        .require_memory()
        .unwrap()
        .provenance_summary(subject, &same_root_supports)
        .await
        .expect("same-root provenance");
    assert_eq!(same_root_summary.roots.len(), 1);
    assert_eq!(same_root_summary.normalized_inputs.len(), 1);
    let external_a = external_observation(&runtime, subject, "object:source-a").await;
    let external_a_repeat = external_observation(&runtime, subject, "object:source-a").await;
    let external_b = external_observation(&runtime, subject, "object:source-b").await;
    let external_a_summary = runtime
        .require_memory()
        .unwrap()
        .provenance_summary(
            subject,
            &[RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: external_a.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
        )
        .await
        .expect("external source root");
    let external_a_repeat_summary = runtime
        .require_memory()
        .unwrap()
        .provenance_summary(
            subject,
            &[RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: external_a_repeat.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
        )
        .await
        .expect("repeated external source root");
    let external_b_summary = runtime
        .require_memory()
        .unwrap()
        .provenance_summary(
            subject,
            &[RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: external_b.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
        )
        .await
        .expect("independent external source root");
    assert_eq!(external_a_summary.roots.len(), 1);
    assert_eq!(external_a_summary.roots, external_a_repeat_summary.roots);
    assert_eq!(
        nous_memory::dependency_relation(
            &external_a_summary.roots.iter().cloned().collect::<Vec<_>>(),
            &external_b_summary.roots.iter().cloned().collect::<Vec<_>>(),
        ),
        nous_memory::DependencyRelation::Independent
    );
    let occurrence_only_a = occurrence_only_observation(&runtime, subject).await;
    let occurrence_only_b = occurrence_only_observation(&runtime, subject).await;
    let occurrence_only_a_summary = runtime
        .require_memory()
        .unwrap()
        .provenance_summary(
            subject,
            &[RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: occurrence_only_a.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
        )
        .await
        .expect("occurrence-only root");
    let occurrence_only_b_summary = runtime
        .require_memory()
        .unwrap()
        .provenance_summary(
            subject,
            &[RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: occurrence_only_b.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
        )
        .await
        .expect("occurrence-only root");
    assert_eq!(
        nous_memory::dependency_relation(
            &occurrence_only_a_summary
                .roots
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            &occurrence_only_b_summary
                .roots
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        ),
        nous_memory::DependencyRelation::UnknownDependency
    );
    let mut duplicate_source = form_input(
        subject,
        first_observation.occurrence.occurrence_id,
        OperationId::new(),
        "Alice lives in Paris from one source",
    );
    duplicate_source.formation_mode = FormationMode::Synthesized;
    duplicate_source.grounding_occurrence_id = None;
    duplicate_source.supports = same_root_supports;
    duplicate_source.epistemic_class = EpistemicClass::Inferred;
    assert!(matches!(
        runtime
            .require_memory()
            .unwrap()
            .form_memory(duplicate_source)
            .await,
        Err(nous_core::Error::Invalid(_))
    ));
    let operation = OperationId::new();
    let form = form_input(
        subject,
        first_observation.occurrence.occurrence_id,
        operation,
        "Alice lives in Paris",
    );
    let first = runtime
        .require_memory()
        .unwrap()
        .form_memory(form.clone())
        .await
        .expect("form memory");
    let retry = runtime
        .require_memory()
        .unwrap()
        .form_memory(form)
        .await
        .expect("idempotent form");
    assert_eq!(first.object.memory_id, retry.object.memory_id);
    let changed = form_input(
        subject,
        first_observation.occurrence.occurrence_id,
        operation,
        "changed",
    );
    assert!(matches!(
        runtime.require_memory().unwrap().form_memory(changed).await,
        Err(nous_core::Error::Conflict(_))
    ));
    let second = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            second_observation.occurrence.occurrence_id,
            OperationId::new(),
            "Alice lives in Lyon",
        ))
        .await
        .expect("second memory");
    let concurrent_revision =
        |operation_id: OperationId, text: &str| nous_memory::ReviseMemoryInput {
            operation_id,
            subject,
            memory_id: second.object.memory_id,
            expected_object_epoch: second.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            formation_mode: FormationMode::Grounded,
            grounding_occurrence_id: Some(second_observation.occurrence.occurrence_id),
            semantic_role: "fact".into(),
            representation_text: text.into(),
            title: None,
            supports: vec![RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: second_observation.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            aboutness: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Observed,
        };
    let memory_service = runtime.require_memory().unwrap();
    let (left_revision, right_revision) = tokio::join!(
        memory_service.revise_memory(concurrent_revision(OperationId::new(), "Lyon correction A")),
        memory_service.revise_memory(concurrent_revision(OperationId::new(), "Lyon correction B")),
    );
    assert_eq!(
        [left_revision.is_ok(), right_revision.is_ok()]
            .into_iter()
            .filter(|value| *value)
            .count(),
        1
    );
    assert!(matches!(
        (left_revision, right_revision),
        (Err(nous_core::Error::Conflict(_)), Ok(_)) | (Ok(_), Err(nous_core::Error::Conflict(_)))
    ));
    let revision = runtime
        .require_memory()
        .unwrap()
        .revise_memory(nous_memory::ReviseMemoryInput {
            operation_id: OperationId::new(),
            subject,
            memory_id: first.object.memory_id,
            expected_object_epoch: first.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            formation_mode: FormationMode::Synthesized,
            grounding_occurrence_id: None,
            semantic_role: "fact".into(),
            representation_text: "Alice lives in Lyon".into(),
            title: None,
            supports: vec![
                RevisionSupport::CognitionDependency(nous_memory::CognitionDependency {
                    target_revision: CognitiveRef::MemoryRevision(
                        second.revision.memory_revision_id,
                    ),
                    support_role: SupportRole::Direct,
                }),
                RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: first_observation.occurrence.occurrence_id,
                    locator: EvidenceLocator::WholeOccurrence,
                    support_role: SupportRole::Contextual,
                }),
            ],
            aboutness: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Inferred,
        })
        .await
        .expect("revise memory");
    assert_eq!(revision.object.memory_id, first.object.memory_id);
    assert_eq!(revision.object.object_epoch, first.object.object_epoch + 1);
    assert_eq!(
        runtime
            .require_memory()
            .unwrap()
            .memory_history(subject, first.object.memory_id)
            .await
            .expect("memory history")
            .len(),
        2
    );
    let historical_query = runtime
        .query(nous_core::CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![nous_core::QueryTarget::Exact {
                reference: CognitiveRef::MemoryRevision(first.revision.memory_revision_id),
            }],
            cues: Vec::new(),
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: nous_core::ResultNeed {
                limit: 1,
                ..Default::default()
            },
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("historical exact query");
    assert_eq!(
        historical_query.results[0].reference,
        CognitiveRef::MemoryRevision(first.revision.memory_revision_id)
    );
    let association = runtime
        .require_memory()
        .unwrap()
        .create_association(
            nous_memory::CreateAssociationRequest {
                operation_id: OperationId::new(),
                from: CognitiveRef::MemoryRevision(first.revision.memory_revision_id),
                to: CognitiveRef::MemoryRevision(second.revision.memory_revision_id),
                relation_kind: "custom.unknown_relation".into(),
                polarity: nous_memory::AssociationPolarity::Positive,
                support_class: nous_memory::AssociationSupportClass::HostExplicit,
                supports: vec![nous_memory::AssociationSupport::Revision(
                    RevisionSupport::Evidence(EvidenceRef {
                        occurrence_id: first_observation.occurrence.occurrence_id,
                        locator: EvidenceLocator::WholeOccurrence,
                        support_role: SupportRole::Direct,
                    }),
                )],
                producer_signature_id: None,
                valid_time: TemporalExtent::Unknown,
            },
            subject,
        )
        .await
        .expect("unknown association kind");
    assert_eq!(association.relation_kind, "custom.unknown_relation");
    assert_eq!(association.supports.len(), 1);
    runtime
        .require_memory()
        .unwrap()
        .revoke_association(
            subject,
            association.association_evidence_id,
            OperationId::new(),
        )
        .await
        .expect("revoke association");
    let revoked_at: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT revoked_at FROM association_evidence WHERE subject_id=$1 AND association_evidence_id=$2",
    )
    .bind(subject.0)
    .bind(association.association_evidence_id.0)
    .fetch_one(runtime.store.pool())
    .await
    .expect("association lifecycle");
    assert!(revoked_at.is_some());
    let second_current = runtime
        .require_memory()
        .unwrap()
        .memory(subject, second.object.memory_id, None)
        .await
        .expect("current second memory");
    let cycle_attempt = runtime
        .require_memory()
        .unwrap()
        .revise_memory(nous_memory::ReviseMemoryInput {
            operation_id: OperationId::new(),
            subject,
            memory_id: second.object.memory_id,
            expected_object_epoch: second_current.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            formation_mode: FormationMode::Synthesized,
            grounding_occurrence_id: None,
            semantic_role: "fact".into(),
            representation_text: "cycle attempt".into(),
            title: None,
            supports: vec![
                RevisionSupport::CognitionDependency(nous_memory::CognitionDependency {
                    target_revision: CognitiveRef::MemoryRevision(
                        revision.revision.memory_revision_id,
                    ),
                    support_role: SupportRole::Direct,
                }),
                RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: second_observation.occurrence.occurrence_id,
                    locator: EvidenceLocator::WholeOccurrence,
                    support_role: SupportRole::Contextual,
                }),
            ],
            aboutness: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Inferred,
        })
        .await;
    assert!(matches!(
        cycle_attempt,
        Err(nous_core::Error::FailedPrecondition(_))
    ));
    let dependent = runtime
        .require_memory()
        .unwrap()
        .form_memory(ExplicitMemoryInput {
            operation_id: OperationId::new(),
            subject,
            cognitive_role: CognitiveRole::Declarative,
            formation_mode: FormationMode::Synthesized,
            grounding_occurrence_id: None,
            semantic_role: "derived_fact".into(),
            representation_text: "Alice's location was corrected by a later source".into(),
            title: None,
            supports: vec![
                RevisionSupport::CognitionDependency(nous_memory::CognitionDependency {
                    target_revision: CognitiveRef::MemoryRevision(
                        revision.revision.memory_revision_id,
                    ),
                    support_role: SupportRole::Interpretation,
                }),
                RevisionSupport::CognitionDependency(nous_memory::CognitionDependency {
                    target_revision: CognitiveRef::MemoryRevision(
                        second_current.revision.memory_revision_id,
                    ),
                    support_role: SupportRole::Direct,
                }),
            ],
            aboutness: Vec::new(),
            tags: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            epistemic_class: EpistemicClass::Inferred,
        })
        .await
        .expect("dependent memory");
    let event_id = nous_core::UseEventId::new();
    let event_occurred_at = Utc::now();
    let make_event = |kind| UseFeedbackEvent {
        event_id,
        reference: CognitiveRef::MemoryRevision(revision.revision.memory_revision_id),
        use_kind: kind,
        occurred_at: event_occurred_at,
        context: serde_json::json!({"receipt":"opaque:1"}),
    };
    let (accepted, duplicate, _) = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![make_event(UseKind::ResultRefuted)],
        })
        .await
        .expect("use event");
    assert_eq!((accepted, duplicate), (1, 0));
    let (_, duplicate, _) = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![make_event(UseKind::ResultRefuted)],
        })
        .await
        .expect("idempotent use retry");
    assert_eq!(duplicate, 1);
    let (accepted, duplicate, _) = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:other:runtime".into(),
            events: vec![make_event(UseKind::ResultRefuted)],
        })
        .await
        .expect("consumer scoped use");
    assert_eq!((accepted, duplicate), (1, 0));
    assert!(matches!(
        runtime
            .cognition
            .use_feedback(UseFeedback {
                subject,
                session_id: None,
                consumer_ref: "consumer:test:runtime".into(),
                events: vec![make_event(UseKind::Corrected)]
            })
            .await,
        Err(nous_core::Error::Conflict(_))
    ));
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM cognitive_use_events WHERE subject_id=$1")
            .bind(subject.0)
            .fetch_one(runtime.store.pool())
            .await
            .expect("use count");
    let batch_event = UseFeedbackEvent {
        event_id: nous_core::UseEventId::new(),
        reference: CognitiveRef::MemoryRevision(second.revision.memory_revision_id),
        use_kind: UseKind::Referenced,
        occurred_at: Utc::now(),
        context: serde_json::json!({}),
    };
    assert!(matches!(
        runtime
            .cognition
            .use_feedback(UseFeedback {
                subject,
                session_id: None,
                consumer_ref: "consumer:test:runtime".into(),
                events: vec![batch_event, make_event(UseKind::Corrected)]
            })
            .await,
        Err(nous_core::Error::Conflict(_))
    ));
    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM cognitive_use_events WHERE subject_id=$1")
            .bind(subject.0)
            .fetch_one(runtime.store.pool())
            .await
            .expect("use count");
    assert_eq!(before, after);
    let schema_link = |role| SchemaEvidenceLinkInput {
        role,
        support: RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: first_observation.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
    };
    let schema_input = |operation_id| CreateSchemaInput {
        operation_id,
        subject,
        title: Some("learning schema".into()),
        structural_claim: "Repeated practice improves recall".into(),
        applicability_scope: SchemaScope {
            description: "learning situations".into(),
            aboutness: Vec::new(),
            tags: Vec::new(),
            valid_time: TemporalExtent::Unknown,
        },
        boundary_definition: "when practice is absent".into(),
        formed_at: Utc::now(),
        formation_kind: SchemaFormationKind::ExplicitImport,
        evidence_links: vec![
            schema_link(SchemaEvidenceRole::Support),
            schema_link(SchemaEvidenceRole::BoundaryCase),
        ],
    };
    let schema = runtime
        .require_memory()
        .unwrap()
        .create_schema(schema_input(OperationId::new()))
        .await
        .expect("schema create");
    let updated_schema = runtime
        .require_memory()
        .unwrap()
        .add_schema_evidence(
            subject,
            schema.schema.schema_id,
            OperationId::new(),
            schema.schema.object_epoch,
            schema_link(SchemaEvidenceRole::Counterexample),
        )
        .await
        .expect("schema evidence");
    assert_eq!(
        schema.revision.schema_revision_id,
        updated_schema.revision.schema_revision_id
    );
    assert_eq!(
        updated_schema.schema.object_epoch,
        schema.schema.object_epoch + 1
    );
    let revised_schema = runtime
        .require_memory()
        .unwrap()
        .revise_schema(nous_memory::ReviseSchemaInput {
            operation_id: OperationId::new(),
            subject,
            schema_id: schema.schema.schema_id,
            expected_object_epoch: updated_schema.schema.object_epoch,
            intent: nous_memory::RevisionIntent::Rephrase,
            title: Some("learning schema revised".into()),
            structural_claim: "Repeated deliberate practice improves recall".into(),
            applicability_scope: SchemaScope {
                description: "deliberate learning situations".into(),
                aboutness: Vec::new(),
                tags: Vec::new(),
                valid_time: TemporalExtent::Unknown,
            },
            boundary_definition: "when deliberate practice is absent".into(),
            formed_at: Utc::now(),
            copy_link_ids: updated_schema
                .evidence_links
                .iter()
                .map(|link| link.link_id)
                .collect(),
        })
        .await
        .expect("schema content revision");
    assert_eq!(revised_schema.revision.revision_no, 2);
    assert_eq!(
        revised_schema.schema.object_epoch,
        updated_schema.schema.object_epoch + 1
    );
    assert_eq!(revised_schema.evidence_links.len(), 3);
    let children = runtime
        .require_memory()
        .unwrap()
        .split_schemas(
            subject,
            schema.schema.schema_id,
            OperationId::new(),
            revised_schema.schema.object_epoch,
            vec![
                schema_input(OperationId::new()),
                schema_input(OperationId::new()),
            ],
        )
        .await
        .expect("schema split");
    assert_eq!(children.len(), 2);
    assert!(children.iter().all(|child| matches!(
        child.schema.acceptance_state,
        nous_memory::AcceptanceState::Accepted
    )));
    let merged_schema = runtime
        .require_memory()
        .unwrap()
        .merge_schemas(
            subject,
            OperationId::new(),
            children
                .iter()
                .map(|child| child.schema.schema_id)
                .collect(),
            children
                .iter()
                .map(|child| child.schema.object_epoch)
                .collect(),
            schema_input(OperationId::new()),
        )
        .await
        .expect("schema merge");
    assert_eq!(merged_schema.evidence_links.len(), 2);
    for child in &children {
        assert!(matches!(
            runtime
                .require_memory()
                .unwrap()
                .schema(subject, child.schema.schema_id)
                .await
                .expect("merged source")
                .schema
                .acceptance_state,
            nous_memory::AcceptanceState::Withdrawn
        ));
    }
    let dependent_schema = runtime
        .require_memory()
        .unwrap()
        .create_schema(CreateSchemaInput {
            operation_id: OperationId::new(),
            subject,
            title: Some("dependent schema".into()),
            structural_claim: "A correction depends on both revisions".into(),
            applicability_scope: SchemaScope {
                description: "dependent cognition".into(),
                aboutness: Vec::new(),
                tags: Vec::new(),
                valid_time: TemporalExtent::Unknown,
            },
            boundary_definition: "when either support is purged".into(),
            formed_at: Utc::now(),
            formation_kind: SchemaFormationKind::ExplicitImport,
            evidence_links: vec![
                SchemaEvidenceLinkInput {
                    role: SchemaEvidenceRole::Support,
                    support: RevisionSupport::CognitionDependency(
                        nous_memory::CognitionDependency {
                            target_revision: CognitiveRef::MemoryRevision(
                                revision.revision.memory_revision_id,
                            ),
                            support_role: SupportRole::Direct,
                        },
                    ),
                },
                SchemaEvidenceLinkInput {
                    role: SchemaEvidenceRole::BoundaryCase,
                    support: RevisionSupport::CognitionDependency(
                        nous_memory::CognitionDependency {
                            target_revision: CognitiveRef::MemoryRevision(
                                second_current.revision.memory_revision_id,
                            ),
                            support_role: SupportRole::Contextual,
                        },
                    ),
                },
            ],
        })
        .await
        .expect("dependent schema");
    let operation = OperationId::new();
    let current = runtime
        .require_memory()
        .unwrap()
        .memory(subject, first.object.memory_id, None)
        .await
        .expect("current");
    runtime
        .require_memory()
        .unwrap()
        .suppress(
            subject,
            current.object.memory_id,
            operation,
            current.object.object_epoch,
        )
        .await
        .expect("suppress");
    let current = runtime
        .require_memory()
        .unwrap()
        .memory(subject, first.object.memory_id, None)
        .await
        .expect("suppressed");
    runtime
        .require_memory()
        .unwrap()
        .restore(
            subject,
            current.object.memory_id,
            OperationId::new(),
            current.object.object_epoch,
        )
        .await
        .expect("restore");
    let current = runtime
        .require_memory()
        .unwrap()
        .memory(subject, first.object.memory_id, None)
        .await
        .expect("restored");
    runtime
        .require_memory()
        .unwrap()
        .purge_memory(
            subject,
            current.object.memory_id,
            OperationId::new(),
            current.object.object_epoch,
        )
        .await
        .expect("purge");
    let dependent_after_purge = runtime
        .require_memory()
        .unwrap()
        .memory(subject, dependent.object.memory_id, None)
        .await
        .expect("dependent after purge");
    assert!(matches!(
        dependent_after_purge.object.integrity_state,
        nous_memory::IntegrityState::RevalidationRequired
    ));
    let dependent_schema_after_purge = runtime
        .require_memory()
        .unwrap()
        .schema(subject, dependent_schema.schema.schema_id)
        .await
        .expect("dependent schema after purge");
    assert!(matches!(
        dependent_schema_after_purge.schema.integrity_state,
        nous_memory::IntegrityState::RevalidationRequired
    ));
    assert_eq!(dependent_schema_after_purge.evidence_links.len(), 1);
    assert!(matches!(
        dependent_schema_after_purge.evidence_links[0].support,
        RevisionSupport::CognitionDependency(nous_memory::CognitionDependency {
            target_revision: CognitiveRef::MemoryRevision(id),
            ..
        }) if id == second_current.revision.memory_revision_id
    ));
    assert!(matches!(
        runtime
            .require_memory()
            .unwrap()
            .memory(subject, first.object.memory_id, None)
            .await,
        Err(nous_core::Error::NotFound(_))
    ));
    let detailed_use_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cognitive_use_events WHERE subject_id=$1 AND ref_value=$2",
    )
    .bind(subject.0)
    .bind(revision.revision.memory_revision_id.0.to_string())
    .fetch_one(runtime.store.pool())
    .await
    .expect("purged use detail count");
    assert_eq!(detailed_use_count, 0);
    let purged_use_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM purged_use_receipts WHERE subject_id=$1 AND event_id=$2",
    )
    .bind(subject.0)
    .bind(event_id.0)
    .fetch_one(runtime.store.pool())
    .await
    .expect("purged use tombstone count");
    assert_eq!(purged_use_count, 2);
    assert!(
        runtime
            .require_memory()
            .unwrap()
            .memory(subject, second.object.memory_id, None)
            .await
            .is_ok()
    );
    let first_projection = runtime
        .serving
        .refresh(subject)
        .await
        .expect("build exact serving projection");
    assert!(first_projection.generations.contains_key("exact"));
    let exact_before_corruption = runtime
        .store
        .serving_current(subject)
        .await
        .expect("exact serving record")
        .into_iter()
        .find(|record| record.family == "exact")
        .expect("exact generation");
    std::fs::write(
        std::path::Path::new(&exact_before_corruption.artifact_location).join("postings.json"),
        b"corrupted serving artifact",
    )
    .expect("corrupt temporary artifact");
    drop(runtime);
    let reopened = open_runtime(&url, &root).await;
    assert!(
        reopened
            .require_memory()
            .unwrap()
            .memory(subject, second.object.memory_id, None)
            .await
            .is_ok()
    );
    let exact_after_rebuild = reopened
        .store
        .serving_current(subject)
        .await
        .expect("rebuilt exact serving record")
        .into_iter()
        .find(|record| record.family == "exact")
        .expect("rebuilt exact generation");
    assert_ne!(
        exact_before_corruption.generation_id,
        exact_after_rebuild.generation_id
    );
    let crash_purge_operation = OperationId::new();
    let crash_purge_digest = nous_core::canonical_request_digest(
        "purge_memory",
        subject,
        &serde_json::json!({
            "memory_id": dependent.object.memory_id,
            "expected_object_epoch": dependent_after_purge.object.object_epoch,
        }),
    )
    .expect("purge digest");
    sqlx::query("INSERT INTO mutation_receipts(subject_id,operation_id,operation_kind,request_digest,state,created_at) VALUES($1,$2,'purge_memory',$3,'in_progress',$4)")
        .bind(subject.0)
        .bind(crash_purge_operation.0)
        .bind(crash_purge_digest)
        .bind(Utc::now())
        .execute(reopened.store.pool())
        .await
        .expect("phase one receipt");
    sqlx::query("UPDATE memory_objects SET purge_state='purging',object_epoch=object_epoch+1 WHERE subject_id=$1 AND memory_id=$2")
        .bind(subject.0)
        .bind(dependent.object.memory_id.0)
        .execute(reopened.store.pool())
        .await
        .expect("phase one fence");
    reopened
        .store
        .invalidate(subject, ProjectionInvalidation::all())
        .await
        .expect("phase one watermark");
    reopened
        .require_memory()
        .unwrap()
        .purge_memory(
            subject,
            dependent.object.memory_id,
            crash_purge_operation,
            dependent_after_purge.object.object_epoch,
        )
        .await
        .expect("resume phase two purge");
    assert!(matches!(
        reopened
            .require_memory()
            .unwrap()
            .memory(subject, dependent.object.memory_id, None)
            .await,
        Err(nous_core::Error::NotFound(_))
    ));
    let (_, duplicate, _) = reopened
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:test:runtime".into(),
            events: vec![make_event(UseKind::ResultRefuted)],
        })
        .await
        .expect("purged use retry");
    assert_eq!(duplicate, 1);
}
