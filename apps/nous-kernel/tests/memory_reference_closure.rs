#[path = "test_support/mod.rs"]
mod test_support;

use chrono::{DateTime, Utc};
use nous_authority_store::ServingRecord;
use nous_cognitive_runtime::{
    CheckpointWrite, ConsumerProfile, ContextBudget, MaterializationPolicy, RuntimeMutation,
    UseFeedback, UseFeedbackEvent, UseKind, WorkingSetRequest,
};
use nous_core::{
    CognitiveQuery, CognitiveRef, Cue, EntityRef, EpistemicClass, OperationId, QueryTarget,
    ResultNeed, TemporalExtent, UseEventId,
};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, ResolvedEntityMention,
    RuntimeDirective,
};
use nous_material_service::MaterializeRequest;
use nous_memory_domain::{
    CognitiveRole, EvidenceLocator, EvidenceRef, ExplicitMemoryInput, FormationMode,
    RevisionIntent, RevisionSupport, SupportRole,
};
use nous_subject_core::{CognitiveSeedInput, CreateSubject, SubjectCapabilities};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use test_support::{database, open_memory_only_runtime};
use uuid::Uuid;

const CORPUS: &str = include_str!("fixtures/memory-reference-r1/corpus.json");
const ORACLE: &str = include_str!("fixtures/memory-reference-r1/oracle.json");

fn fixed_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("fixture timestamp")
        .with_timezone(&Utc)
}

fn operation(value: u128) -> OperationId {
    OperationId(Uuid::from_u128(value))
}

fn subject_id(value: u128) -> nous_core::SubjectId {
    nous_core::SubjectId(Uuid::from_u128(value))
}

async fn create_subject(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
) -> nous_core::SubjectId {
    runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: Some(subject),
            operation_id: operation(subject.0.as_u128() + 100),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({"fixture":"memory-reference-r1"}),
            },
            metadata: serde_json::json!({"fixture":"memory-reference-r1"}),
            capabilities: Some(SubjectCapabilities {
                memory: true,
                self_cognition: false,
                social: false,
            }),
        })
        .await
        .expect("fixture subject")
        .subject_id
}

fn observation_input(
    subject: nous_core::SubjectId,
    session: Option<nous_core::SessionId>,
    text: &str,
    observed_at: DateTime<Utc>,
    external_object: Option<&str>,
    entities: Vec<ResolvedEntityMention>,
) -> ObservationInput {
    let material = match external_object {
        Some(value) => ObservationMaterial::ExternalObjectRef {
            object_ref: nous_core::ObjectRef::new(value).expect("external object"),
        },
        None => ObservationMaterial::InlineText {
            text: text.into(),
            media_type: "text/plain".into(),
        },
    };
    ObservationInput {
        subject,
        session,
        occurrence: OccurrenceDescriptor {
            source_class: nous_core::SourceClass::Message,
            external_object_ref: external_object
                .map(|value| nous_core::ObjectRef::new(value).expect("external object")),
            occurred_time: TemporalExtent::Unknown,
            observed_at,
            conversation_ref: None,
            actor_entity_ref: None,
            context: serde_json::json!({"fixture":"memory-reference-r1"}),
        },
        material,
        entities,
        runtime: RuntimeDirective::default(),
    }
}

fn form_input(
    subject: nous_core::SubjectId,
    operation_id: OperationId,
    occurrence: nous_core::OccurrenceId,
    text: &str,
    mode: FormationMode,
    supports: Vec<RevisionSupport>,
    aboutness: Vec<EntityRef>,
    valid_time: TemporalExtent,
) -> ExplicitMemoryInput {
    ExplicitMemoryInput {
        operation_id,
        subject,
        cognitive_role: CognitiveRole::Declarative,
        formation_mode: mode,
        grounding_occurrence_id: (mode == FormationMode::Grounded).then_some(occurrence),
        semantic_role: "fixture_fact".into(),
        representation_text: text.into(),
        title: None,
        supports,
        aboutness,
        tags: Vec::new(),
        valid_time,
        formed_at: fixed_time("2026-09-03T00:00:00Z"),
        epistemic_class: if mode == FormationMode::Grounded {
            EpistemicClass::Observed
        } else {
            EpistemicClass::Inferred
        },
    }
}

fn query(
    subject: nous_core::SubjectId,
    session: Option<nous_core::SessionId>,
    target: QueryTarget,
    cues: Vec<Cue>,
) -> CognitiveQuery {
    CognitiveQuery {
        api_version: nous_core::API_VERSION,
        subject,
        session,
        situation: Default::default(),
        targets: vec![target],
        cues,
        constraints: Default::default(),
        exploration: Default::default(),
        resources: Default::default(),
        result_need: ResultNeed {
            limit: 16,
            ..Default::default()
        },
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: Default::default(),
    }
}

fn oracle_labels(oracle: &Value, query_id: &str, field: &str) -> Vec<String> {
    oracle["queries"][query_id][field]
        .as_array()
        .expect("oracle list")
        .iter()
        .map(|value| value.as_str().expect("oracle label").to_owned())
        .collect()
}

fn assert_oracle(
    oracle: &Value,
    query_id: &str,
    result: &nous_core::CognitiveQueryResult,
    references: &HashMap<String, CognitiveRef>,
) {
    let actual = result
        .results
        .iter()
        .map(|hit| hit.reference.to_string())
        .collect::<HashSet<_>>();
    for label in oracle_labels(oracle, query_id, "MUST_RETURN") {
        let reference = references.get(&label).expect("oracle reference");
        assert!(
            actual.contains(&reference.to_string()),
            "{query_id} must return {label}; actual={actual:?}"
        );
    }
    for label in oracle_labels(oracle, query_id, "MUST_NOT_RETURN") {
        let reference = references.get(&label).expect("oracle reference");
        assert!(
            !actual.contains(&reference.to_string()),
            "{query_id} must not return {label}; actual={actual:?}"
        );
    }
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "deterministic closure scenario is the qualification source for the R1 oracle"
)]
async fn memory_reference_profile_closure_is_multi_session_and_recoverable() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("corpus JSON");
    let oracle: Value = serde_json::from_str(ORACLE).expect("oracle JSON");
    assert_eq!(corpus["version"], oracle["version"]);
    assert_eq!(corpus["version"], "memory-reference-r1-2026-09-29");

    let (root, url, _postgres) = database().await;
    let runtime = open_memory_only_runtime(&url, &root).await;
    assert!(runtime.self_cognition.is_none());
    assert!(runtime.social.is_none());
    let subject = create_subject(&runtime, subject_id(1)).await;
    let foreign_subject = create_subject(&runtime, subject_id(2)).await;
    let session_a = runtime
        .cognition
        .open_session(subject, serde_json::json!({"fixture":"a"}))
        .await
        .expect("session a");
    let session_b = runtime
        .cognition
        .open_session(subject, serde_json::json!({"fixture":"b"}))
        .await
        .expect("session b");

    let alice = EntityRef::new("entity:person:alice").expect("Alice");
    let bob = EntityRef::new("entity:person:bob").expect("Bob");
    let o1 = runtime
        .material
        .record_observation_once(
            observation_input(
                subject,
                Some(session_a.session_id),
                "Alice lives in Paris; Bob is mentioned",
                fixed_time("2026-09-01T00:00:00Z"),
                None,
                vec![
                    ResolvedEntityMention {
                        surface: "Alice".into(),
                        entity_ref: Some(alice.clone()),
                        semantic_role: Some("subject".into()),
                    },
                    ResolvedEntityMention {
                        surface: "Bob".into(),
                        entity_ref: Some(bob.clone()),
                        semantic_role: Some("mention".into()),
                    },
                ],
            ),
            Some(Uuid::from_u128(11)),
        )
        .await
        .expect("observation o1");
    let o2 = runtime
        .material
        .record_observation_once(
            observation_input(
                subject,
                Some(session_b.session_id),
                "external source says Alice lives in Lyon",
                fixed_time("2026-09-02T00:00:00Z"),
                Some("object:source-b"),
                Vec::new(),
            ),
            Some(Uuid::from_u128(12)),
        )
        .await
        .expect("observation o2");
    let foreign_observation = runtime
        .material
        .record_observation_once(
            observation_input(
                foreign_subject,
                None,
                "foreign subject memory",
                fixed_time("2026-09-02T00:00:00Z"),
                Some("object:foreign"),
                vec![ResolvedEntityMention {
                    surface: "Alice".into(),
                    entity_ref: Some(alice.clone()),
                    semantic_role: Some("subject".into()),
                }],
            ),
            Some(Uuid::from_u128(13)),
        )
        .await
        .expect("foreign observation");

    let memory = runtime
        .require_memory()
        .expect("Memory-only Memory owner")
        .clone();
    let m1 = memory
        .form_memory(form_input(
            subject,
            operation(21),
            o1.occurrence.occurrence_id,
            "Alice lives in Paris",
            FormationMode::Grounded,
            vec![RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: o1.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            vec![alice.clone()],
            TemporalExtent::Unknown,
        ))
        .await
        .expect("grounded M1");
    let m2 = memory
        .form_memory(form_input(
            subject,
            operation(22),
            o1.occurrence.occurrence_id,
            "Independent sources place Alice in Lyon",
            FormationMode::Synthesized,
            vec![
                RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: o1.occurrence.occurrence_id,
                    locator: EvidenceLocator::WholeOccurrence,
                    support_role: SupportRole::Direct,
                }),
                RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: o2.occurrence.occurrence_id,
                    locator: EvidenceLocator::WholeOccurrence,
                    support_role: SupportRole::Corroborating,
                }),
            ],
            vec![alice.clone()],
            TemporalExtent::Unknown,
        ))
        .await
        .expect("synthesized M2");
    let m1_r2 = memory
        .revise_memory(nous_memory_service::ReviseMemoryInput {
            operation_id: operation(23),
            subject,
            memory_id: m1.object.memory_id,
            expected_object_epoch: m1.object.object_epoch,
            intent: RevisionIntent::Correct,
            formation_mode: FormationMode::Grounded,
            grounding_occurrence_id: Some(o1.occurrence.occurrence_id),
            semantic_role: "fixture_fact".into(),
            representation_text: "Alice lives in Paris (corrected wording)".into(),
            title: None,
            supports: vec![RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: o1.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            aboutness: vec![alice.clone()],
            valid_time: TemporalExtent::Unknown,
            formed_at: fixed_time("2026-09-03T00:00:00Z"),
            epistemic_class: EpistemicClass::Observed,
        })
        .await
        .expect("M1 correction revision");
    let m3 = memory
        .form_memory(form_input(
            subject,
            operation(24),
            o2.occurrence.occurrence_id,
            "Alice later lives in Lyon",
            FormationMode::Grounded,
            vec![RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: o2.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            vec![alice.clone()],
            TemporalExtent::Instant {
                at: fixed_time("2026-09-02T00:00:00Z"),
            },
        ))
        .await
        .expect("temporal successor M3");
    let foreign = memory
        .form_memory(form_input(
            foreign_subject,
            operation(25),
            foreign_observation.occurrence.occurrence_id,
            "Foreign subject memory",
            FormationMode::Grounded,
            vec![RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: foreign_observation.occurrence.occurrence_id,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            })],
            vec![alice.clone()],
            TemporalExtent::Unknown,
        ))
        .await
        .expect("foreign memory");

    let references = HashMap::from([
        (
            "m1".into(),
            CognitiveRef::MemoryRevision(m1.revision.memory_revision_id),
        ),
        (
            "m1-r2".into(),
            CognitiveRef::MemoryRevision(m1_r2.revision.memory_revision_id),
        ),
        (
            "m2".into(),
            CognitiveRef::MemoryRevision(m2.revision.memory_revision_id),
        ),
        (
            "m3".into(),
            CognitiveRef::MemoryRevision(m3.revision.memory_revision_id),
        ),
        (
            "foreign-m1".into(),
            CognitiveRef::MemoryRevision(foreign.revision.memory_revision_id),
        ),
    ]);
    let alice_result = runtime
        .query(query(
            subject,
            None,
            QueryTarget::AnyRelevantCognition,
            vec![Cue::Entity(nous_core::EntityCue {
                entity_ref: alice.clone(),
            })],
        ))
        .await
        .expect("Alice entity query");
    assert_oracle(&oracle, "entity-alice", &alice_result, &references);
    let bob_result = runtime
        .query(query(
            subject,
            None,
            QueryTarget::AnyRelevantCognition,
            vec![Cue::Entity(nous_core::EntityCue { entity_ref: bob })],
        ))
        .await
        .expect("Bob mention query");
    assert_oracle(&oracle, "entity-bob-mention-only", &bob_result, &references);
    let historical = runtime
        .query(query(
            subject,
            None,
            QueryTarget::Exact {
                reference: references["m1"].clone(),
            },
            Vec::new(),
        ))
        .await
        .expect("historical exact query");
    assert_oracle(&oracle, "exact-historical-m1-r1", &historical, &references);

    let session_a_revision = runtime
        .cognition
        .session(subject, session_a.session_id)
        .await
        .expect("session A after observation")
        .runtime_revision;
    let session_b_revision = runtime
        .cognition
        .session(subject, session_b.session_id)
        .await
        .expect("session B after observation")
        .runtime_revision;
    runtime
        .cognition
        .mutate_runtime(RuntimeMutation {
            subject,
            session: session_a.session_id,
            expected_runtime_revision: session_a_revision,
            checkpoints: vec![CheckpointWrite {
                owner_kind: "focus".into(),
                owner_key: "focus-a".into(),
                schema_version: 1,
                expected_revision: 0,
                payload: vec![1, 2, 3],
            }],
            foreground: Some(Some("focus-a".into())),
        })
        .await
        .expect("focus A");
    runtime
        .cognition
        .mutate_runtime(RuntimeMutation {
            subject,
            session: session_b.session_id,
            expected_runtime_revision: session_b_revision,
            checkpoints: vec![CheckpointWrite {
                owner_kind: "focus".into(),
                owner_key: "focus-b".into(),
                schema_version: 1,
                expected_revision: 0,
                payload: vec![4, 5, 6],
            }],
            foreground: Some(Some("focus-b".into())),
        })
        .await
        .expect("focus B");
    let focus_a = runtime
        .cognition
        .runtime_snapshot(subject, session_a.session_id, "focus")
        .await
        .expect("focus A snapshot");
    let focus_b = runtime
        .cognition
        .runtime_snapshot(subject, session_b.session_id, "focus")
        .await
        .expect("focus B snapshot");
    assert_eq!(focus_a.active_focus_key.as_deref(), Some("focus-a"));
    assert_eq!(focus_b.active_focus_key.as_deref(), Some("focus-b"));
    assert_eq!(focus_a.revision, session_a_revision + 1);
    assert_eq!(focus_b.revision, session_b_revision + 1);

    let use_a = UseFeedbackEvent {
        event_id: UseEventId(Uuid::from_u128(31)),
        reference: references["m1-r2"].clone(),
        use_kind: UseKind::Referenced,
        occurred_at: fixed_time("2026-09-03T01:00:00Z"),
        context: serde_json::json!({"fixture":"alpha"}),
    };
    let accepted_a = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_a.session_id),
            consumer_ref: "consumer:alpha:reference".into(),
            events: vec![use_a.clone()],
        })
        .await
        .expect("consumer A use");
    assert_eq!((accepted_a.0, accepted_a.1), (1, 0));
    let duplicate_a = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_a.session_id),
            consumer_ref: "consumer:alpha:reference".into(),
            events: vec![use_a],
        })
        .await
        .expect("consumer A retry");
    assert_eq!((duplicate_a.0, duplicate_a.1), (0, 1));
    let accepted_b = runtime
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_b.session_id),
            consumer_ref: "consumer:beta:reference".into(),
            events: vec![UseFeedbackEvent {
                event_id: UseEventId(Uuid::from_u128(32)),
                reference: references["m3"].clone(),
                use_kind: UseKind::Referenced,
                occurred_at: fixed_time("2026-09-03T02:00:00Z"),
                context: serde_json::json!({"fixture":"beta"}),
            }],
        })
        .await
        .expect("consumer B use");
    assert_eq!((accepted_b.0, accepted_b.1), (1, 0));

    let working_a = runtime
        .cognition
        .working_set(
            WorkingSetRequest {
                subject,
                session_id: session_a.session_id,
                consumer: ConsumerProfile {
                    consumer_id: "consumer:alpha:reference".into(),
                    context_budget: ContextBudget {
                        max_items: 4,
                        max_text_bytes: 4096,
                    },
                    accepted_modalities: vec![nous_core::Modality::Text],
                    materialization_policy: MaterializationPolicy::AvailableText,
                },
                query_results: Vec::new(),
                references: vec![references["m1-r2"].clone()],
            },
            &runtime,
        )
        .await
        .expect("consumer A working set");
    let working_b = runtime
        .cognition
        .working_set(
            WorkingSetRequest {
                subject,
                session_id: session_b.session_id,
                consumer: ConsumerProfile {
                    consumer_id: "consumer:beta:reference".into(),
                    context_budget: ContextBudget {
                        max_items: 4,
                        max_text_bytes: 4096,
                    },
                    accepted_modalities: vec![nous_core::Modality::Text],
                    materialization_policy: MaterializationPolicy::AvailableText,
                },
                query_results: Vec::new(),
                references: vec![references["m3"].clone()],
            },
            &runtime,
        )
        .await
        .expect("consumer B working set");
    assert_eq!(working_a.consumer_id, "consumer:alpha:reference");
    assert_eq!(working_b.consumer_id, "consumer:beta:reference");
    assert!(working_a.refs.contains(&references["m1-r2"]));
    assert!(!working_a.refs.contains(&references["m3"]));
    assert!(working_b.refs.contains(&references["m3"]));
    assert!(!working_b.refs.contains(&references["m1-r2"]));

    let runtime_a = runtime
        .query(query(
            subject,
            Some(session_a.session_id),
            QueryTarget::AnyRelevantCognition,
            Vec::new(),
        ))
        .await
        .expect("runtime A query");
    assert_oracle(&oracle, "runtime-session-a", &runtime_a, &references);
    let runtime_b = runtime
        .query(query(
            subject,
            Some(session_b.session_id),
            QueryTarget::AnyRelevantCognition,
            Vec::new(),
        ))
        .await
        .expect("runtime B query");
    assert_oracle(&oracle, "runtime-session-b", &runtime_b, &references);

    let r2_view = memory
        .revision(subject, m1_r2.revision.memory_revision_id)
        .await
        .expect("M1 R2 read");
    assert!(r2_view.supports.iter().any(|support| matches!(
        support,
        RevisionSupport::Evidence(value)
            if value.occurrence_id == o1.occurrence.occurrence_id
    )));
    let materialized = runtime
        .materialize(
            subject,
            MaterializeRequest {
                reference: CognitiveRef::Occurrence(o1.occurrence.occurrence_id),
                byte_range: None,
                max_bytes: 4096,
                resource_handle: None,
                resource: None,
            },
        )
        .await
        .expect("trace source material");
    assert!(String::from_utf8_lossy(&materialized.bytes).contains("Alice"));
    assert!(
        materialized
            .provenance
            .contains(&CognitiveRef::Occurrence(o1.occurrence.occurrence_id))
    );

    let current = memory
        .memory(subject, m1.object.memory_id, None)
        .await
        .expect("M1 current before lifecycle");
    runtime
        .serving
        .refresh(subject)
        .await
        .expect("serving before lifecycle");
    let serving_before: ServingRecord = runtime
        .store
        .serving_current(subject)
        .await
        .expect("serving records")
        .into_iter()
        .find(|record| record.family == "exact")
        .expect("exact serving before lifecycle");
    memory
        .suppress(
            subject,
            current.object.memory_id,
            operation(41),
            current.object.object_epoch,
        )
        .await
        .expect("suppress M1");
    let suppressed = memory
        .memory(subject, current.object.memory_id, None)
        .await
        .expect("suppressed M1");
    memory
        .restore(
            subject,
            suppressed.object.memory_id,
            operation(42),
            suppressed.object.object_epoch,
        )
        .await
        .expect("restore M1");
    let restored_query = runtime
        .query(query(
            subject,
            None,
            QueryTarget::Exact {
                reference: references["m1-r2"].clone(),
            },
            Vec::new(),
        ))
        .await
        .expect("restored exact query");
    assert_oracle(&oracle, "post-restore-m1", &restored_query, &references);
    let restored = memory
        .memory(subject, current.object.memory_id, None)
        .await
        .expect("restored M1");
    memory
        .purge_memory(
            subject,
            restored.object.memory_id,
            operation(43),
            restored.object.object_epoch,
        )
        .await
        .expect("purge M1");
    let purged_query = runtime
        .query(query(
            subject,
            None,
            QueryTarget::Exact {
                reference: references["m1-r2"].clone(),
            },
            Vec::new(),
        ))
        .await;
    assert!(matches!(purged_query, Err(nous_core::Error::NotFound(_))));
    assert!(oracle_labels(&oracle, "post-purge-m1", "MUST_RETURN").is_empty());
    runtime
        .serving
        .refresh(subject)
        .await
        .expect("serving after purge");
    let serving_after = runtime
        .store
        .serving_current(subject)
        .await
        .expect("serving after purge records")
        .into_iter()
        .find(|record| record.family == "exact")
        .expect("exact serving after purge");
    assert_ne!(serving_before.generation_id, serving_after.generation_id);

    drop(runtime);
    let reopened = open_memory_only_runtime(&url, &root).await;
    assert!(reopened.self_cognition.is_none());
    assert!(reopened.social.is_none());
    assert!(
        reopened
            .require_memory()
            .expect("reopened Memory")
            .memory(subject, m2.object.memory_id, None)
            .await
            .is_ok()
    );
    assert!(
        reopened
            .require_memory()
            .expect("reopened Memory")
            .memory(subject, m3.object.memory_id, None)
            .await
            .is_ok()
    );
    let reopened_a = reopened
        .cognition
        .session(subject, session_a.session_id)
        .await
        .expect("reopened session A");
    let reopened_b = reopened
        .cognition
        .session(subject, session_b.session_id)
        .await
        .expect("reopened session B");
    assert_eq!(reopened_a.runtime_revision, focus_a.revision + 1);
    assert_eq!(reopened_b.runtime_revision, focus_b.revision + 1);
    let purged_retry = reopened
        .cognition
        .use_feedback(UseFeedback {
            subject,
            session_id: Some(session_a.session_id),
            consumer_ref: "consumer:alpha:reference".into(),
            events: vec![UseFeedbackEvent {
                event_id: UseEventId(Uuid::from_u128(31)),
                reference: references["m1-r2"].clone(),
                use_kind: UseKind::Referenced,
                occurred_at: fixed_time("2026-09-03T01:00:00Z"),
                context: serde_json::json!({"fixture":"alpha"}),
            }],
        })
        .await
        .expect("purged use retry");
    assert_eq!((purged_retry.0, purged_retry.1), (0, 1));
}
