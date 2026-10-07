// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use nous_protocol::public::{
    memory_service_server::MemoryService as _, subject_service_server::SubjectService as _,
};
mod test_support;

use chrono::{Duration, SubsecRound, Utc};
use nous_core::{Cue, SourceClass, TemporalExtent};
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, RuntimeDirective,
};
use nous_memory::{
    EpisodePartitionInput, EpisodePartitionSegment, EpisodePartitionSource, EpisodeView,
};
use nous_retrieval::ServingOptions;
use nous_runtime::{
    CognitiveClock, MaintenanceDisposition, MaintenanceRequest, ManualCognitiveClock,
};
use sqlx::Row;
use std::sync::Arc;
use test_support::{LongitudinalEmbedding, database};

async fn create_subject(runtime: &NousRuntime) -> nous_core::SubjectId {
    runtime
        .subjects
        .create_subject(nous_subject::CreateSubject {
            subject_id: None,
            operation_id: nous_core::OperationId::new(),
            cognitive_seed: nous_subject::CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: None,
        })
        .await
        .expect("create Subject")
        .subject_id
}

async fn runtime_with_clock(
    url: &str,
    root: &tempfile::TempDir,
    clock: Arc<dyn CognitiveClock>,
) -> NousRuntime {
    runtime_with_clock_serving(url, root, clock, false).await
}

async fn runtime_with_clock_serving(
    url: &str,
    root: &tempfile::TempDir,
    clock: Arc<dyn CognitiveClock>,
    serving: bool,
) -> NousRuntime {
    NousRuntime::open_with_clock(RuntimeOptions {
        postgres_url: url.into(), max_connections: 8,
        acquire_timeout_ms: 15000,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        serving_options: ServingOptions { root: root.path().join("serving"), lexical: serving, dense: serving, topology: false, memory_enabled: true },
        embedding: serving.then(|| Arc::new(LongitudinalEmbedding) as Arc<dyn nous_retrieval::TextEmbeddingProvider>), stored_embedding: None,
        core_descriptors: vec![],
        deployment_document: serde_json::json!({"serving":{"lexical":{"enabled":serving},"dense":{"enabled":serving},"topology":{"enabled":false}}}),
    }, clock).await.expect("open clock-injected runtime")
}

fn observation(
    subject: nous_core::SubjectId,
    session: Option<nous_core::SessionId>,
) -> ObservationInput {
    ObservationInput {
        subject,
        session,
        occurrence: OccurrenceDescriptor {
            source_class: SourceClass::Message,
            external_object_ref: None,
            occurred_time: TemporalExtent::Unknown,
            observed_at: None,
            conversation_ref: None,
            actor_entity_ref: None,
            context: serde_json::json!({}),
        },
        material: ObservationMaterial::InlineText {
            text: "durable experience".into(),
            media_type: "text/plain".into(),
        },
        entities: vec![],
        runtime: RuntimeDirective::default(),
    }
}

#[tokio::test]
async fn experience_capture_cursor_idle_and_reopen_are_stable() {
    let (root, url, _postgres) = database().await;
    let start = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(start));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let first_session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let input = observation(subject, Some(first_session.session_id));
    let key = uuid::Uuid::new_v4();
    let accepted = rt
        .material
        .record_observation_once(input.clone(), Some(key))
        .await
        .unwrap();
    clock.advance_by(subject, Duration::seconds(30)).unwrap();
    let replay = rt
        .material
        .record_observation_once(input, Some(key))
        .await
        .unwrap();
    assert_eq!(
        accepted.occurrence.occurrence_id,
        replay.occurrence.occurrence_id
    );
    assert_eq!(replay.occurrence.observed_at, start);
    rt.material
        .record_observation(observation(subject, None))
        .await
        .unwrap();
    let second_session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    rt.material
        .record_observation(observation(subject, Some(second_session.session_id)))
        .await
        .unwrap();
    let rows = sqlx::query(
        "SELECT recorded_seq FROM experience_items WHERE subject_id=$1 ORDER BY recorded_seq",
    )
    .bind(subject.0)
    .fetch_all(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[1].get::<i64, _>("recorded_seq") > rows[0].get::<i64, _>("recorded_seq") + 1);
    let progress = rt
        .cognition
        .segment_experience(subject, "interaction", 128, false)
        .await
        .unwrap();
    assert_eq!(progress.processed_count, 2);
    assert!(progress.ready.is_empty());
    assert!(progress.next_due.is_some());
    let repeated = rt
        .cognition
        .segment_experience(subject, "interaction", 128, false)
        .await
        .unwrap();
    assert_eq!(repeated.processed_count, 0);
    clock.advance_by(subject, Duration::minutes(30)).unwrap();
    drop(rt);
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let progress = rt
        .cognition
        .segment_experience(subject, "interaction", 128, false)
        .await
        .unwrap();
    assert_eq!(progress.ready.len(), 1);
    assert_eq!(progress.ready[0].members.len(), 2);
    assert_eq!(
        progress.ready[0].boundary_reason.as_deref(),
        Some("hard_idle")
    );
    assert_eq!(progress.processed_count, 0);
    let organized = rt.organize_experience(subject, 128, false).await.unwrap();
    assert_eq!(organized.episodes.len(), 1);
    assert_eq!(organized.episodes[0].revision.formed_at, clock.now(subject));
    assert!(
        rt.organize_experience(subject, 128, false)
            .await
            .unwrap()
            .episodes
            .is_empty()
    );
    assert!(
        rt.cognition
            .maintenance_needs(subject)
            .await
            .unwrap()
            .iter()
            .any(|need| need.kind == "journal_review")
    );
    assert_owner_timestamp_replay(&rt, &clock, subject, accepted.occurrence.occurrence_id).await;
}

#[tokio::test]
async fn maintenance_claims_use_execution_leases_and_keep_new_triggers() {
    let (root, url, _postgres) = database().await;
    let start = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(start));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let request = MaintenanceRequest {
        subject,
        kind: "journal_review".into(),
        scope_kind: "track".into(),
        scope_ref: "interaction".into(),
        trigger_authority_seq: 1,
        due_at: start + Duration::hours(1),
        priority: 30,
    };
    let id = rt
        .cognition
        .enqueue_maintenance(request.clone())
        .await
        .unwrap();
    let kinds = vec!["journal_review".into()];
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, None)
            .await
            .unwrap()
            .is_empty()
    );
    clock.advance_by(subject, Duration::hours(1)).unwrap();
    let (a, b) = tokio::join!(
        rt.cognition.lease_maintenance(subject, &kinds, 1, 60, None),
        rt.cognition.lease_maintenance(subject, &kinds, 1, 60, None)
    );
    let leased: Vec<_> = a.unwrap().into_iter().chain(b.unwrap()).collect();
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].need_id, id);
    clock.advance_by(subject, Duration::days(30)).unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, None)
            .await
            .unwrap()
            .is_empty()
    );
    let newer = MaintenanceRequest {
        trigger_authority_seq: 2,
        ..request
    };
    assert_eq!(rt.cognition.enqueue_maintenance(newer).await.unwrap(), id);
    rt.cognition
        .acknowledge_maintenance(&leased[0], MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    rt.cognition
        .acknowledge_maintenance(&leased[0], MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    assert!(
        rt.cognition
            .acknowledge_maintenance(&leased[0], MaintenanceDisposition::Obsolete)
            .await
            .is_err()
    );
    let reclaimed = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].trigger_authority_seq, 2);
    sqlx::query("UPDATE maintenance_needs SET lease_until=clock_timestamp()-interval '1 second' WHERE need_id=$1")
        .bind(id).execute(rt.store.pool()).await.unwrap();
    let retry = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap();
    assert_eq!(retry[0].attempt_count, 3);
    assert!(
        rt.cognition
            .acknowledge_maintenance(&reclaimed[0], MaintenanceDisposition::Satisfied)
            .await
            .is_err()
    );
    rt.cognition
        .acknowledge_maintenance(&retry[0], MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    assert!(
        rt.cognition
            .maintenance_needs(subject)
            .await
            .unwrap()
            .is_empty()
    );
    assert_subject_pagination(&rt).await;
}

async fn assert_owner_timestamp_replay(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
    occurrence: nous_core::OccurrenceId,
) {
    let input = test_support::form_input(
        subject,
        occurrence,
        nous_core::OperationId::new(),
        "owner-timed grounded memory",
    );
    let at = clock.now(subject);
    let memory = rt
        .require_memory()
        .unwrap()
        .form_memory(input.clone())
        .await
        .unwrap();
    assert_eq!(memory.revision.formed_at, at);
    assert_eq!(memory.revision.recorded_at, at);
    clock.advance_by(subject, Duration::days(1)).unwrap();
    rt.require_memory()
        .unwrap()
        .revise_memory(nous_memory::ReviseMemoryInput {
            producer: None,
            operation_id: nous_core::OperationId::new(),
            subject,
            memory_id: memory.object.memory_id,
            expected_object_epoch: memory.object.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            formation_mode: input.formation_mode,
            grounding_occurrence_id: input.grounding_occurrence_id,
            semantic_role: input.semantic_role.clone(),
            representation_text: "corrected owner-timed memory".into(),
            title: None,
            basis: input.basis.clone(),
            aboutness: vec![],
            valid_time: TemporalExtent::Unknown,
            epistemic_class: input.epistemic_class,
        })
        .await
        .unwrap();
    let replay = rt
        .require_memory()
        .unwrap()
        .form_memory(input)
        .await
        .unwrap();
    assert_eq!(
        replay.revision.memory_revision_id,
        memory.revision.memory_revision_id
    );
    assert_eq!(replay.revision.formed_at, at);
    assert_eq!(replay.revision.recorded_at, at);
    assert_schema_clock(rt, clock, subject, occurrence).await;
}

async fn partition_request(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    sources: &[EpisodeView],
    occurrences: &[nous_core::OccurrenceId],
    groups: &[Vec<usize>],
) -> EpisodePartitionInput {
    EpisodePartitionInput {
        operation_id: nous_core::OperationId::new(),
        subject,
        expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
        sources: sources
            .iter()
            .map(|source| EpisodePartitionSource {
                revision: source.revision.episode_revision_id,
                expected_epoch: source.object.object_epoch,
            })
            .collect(),
        ordered_occurrences: occurrences.to_vec(),
        segments: groups
            .iter()
            .enumerate()
            .map(|(index, indices)| EpisodePartitionSegment {
                member_indices: indices.clone(),
                title: Some(format!("segment {index}")),
                boundary_explanation: "supported local organization".into(),
            })
            .collect(),
        producer: None,
    }
}

#[tokio::test]
async fn episode_partition_split_repartition_merge_and_retry_are_atomic() {
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let mut occurrences = vec![];
    for index in 0..4 {
        let mut input = observation(subject, Some(session.session_id));
        input.material = ObservationMaterial::InlineText {
            text: format!("experience {index}"),
            media_type: "text/plain".into(),
        };
        occurrences.push(
            rt.material
                .record_observation(input)
                .await
                .unwrap()
                .occurrence
                .occurrence_id,
        );
        clock.advance_by(subject, Duration::minutes(1)).unwrap();
    }
    let original = rt
        .organize_experience(subject, 128, true)
        .await
        .unwrap()
        .episodes;
    let memory = rt.require_memory().unwrap();
    let split_request = partition_request(
        &rt,
        subject,
        &original,
        &occurrences,
        &[vec![0, 1], vec![2, 3]],
    )
    .await;
    let split = memory
        .apply_episode_partition(split_request.clone())
        .await
        .unwrap();
    assert_eq!(split.len(), 2);
    assert!(
        split
            .iter()
            .all(|part| part.object.episode_id != original[0].object.episode_id)
    );
    let withdrawn = memory
        .episode(subject, original[0].object.episode_id, None)
        .await
        .unwrap();
    assert_eq!(
        withdrawn.object.acceptance_state,
        nous_memory::AcceptanceState::Withdrawn
    );
    assert!(
        memory
            .reaccept_episode(
                subject,
                withdrawn.object.episode_id,
                nous_core::OperationId::new(),
                withdrawn.object.object_epoch
            )
            .await
            .is_err()
    );
    assert!(split.iter().all(|part| {
        part.relations
            .iter()
            .any(|relation| relation.relation == "split_from")
    }));
    let replay = memory.apply_episode_partition(split_request).await.unwrap();
    assert_eq!(
        replay[0].revision.episode_revision_id,
        split[0].revision.episode_revision_id
    );
    assert_partition_rollback(&rt, subject, &split, &occurrences).await;
    let repartition_request = partition_request(
        &rt,
        subject,
        &split,
        &occurrences,
        &[vec![0], vec![1, 2, 3]],
    )
    .await;
    let repartition = memory
        .apply_episode_partition(repartition_request)
        .await
        .unwrap();
    assert!(repartition.iter().all(|part| {
        part.relations
            .iter()
            .filter(|relation| relation.relation == "derived_from")
            .count()
            == 2
    }));
    let merge_request = partition_request(
        &rt,
        subject,
        &repartition,
        &occurrences,
        &[vec![0, 1, 2, 3]],
    )
    .await;
    let merged = memory.apply_episode_partition(merge_request).await.unwrap();
    assert_eq!(merged.len(), 1);
    assert!(
        merged[0]
            .relations
            .iter()
            .filter(|relation| relation.relation == "merged_from")
            .count()
            == 2
    );
    assert_partition_revision(&rt, subject, &merged, &occurrences).await;
}

async fn assert_partition_rollback(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    sources: &[EpisodeView],
    occurrences: &[nous_core::OccurrenceId],
) {
    let mut invalid =
        partition_request(rt, subject, sources, occurrences, &[vec![0, 1], vec![2, 3]]).await;
    invalid.segments[1].boundary_explanation = "x".repeat(20000);
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM episode_objects WHERE subject_id=$1")
            .bind(subject.0)
            .fetch_one(rt.store.pool())
            .await
            .unwrap();
    let watermark = rt.store.authority_seq(subject).await.unwrap();
    assert!(
        rt.require_memory()
            .unwrap()
            .apply_episode_partition(invalid)
            .await
            .is_err()
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM episode_objects WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(watermark, rt.store.authority_seq(subject).await.unwrap());
    for source in sources {
        let current = rt
            .require_memory()
            .unwrap()
            .episode(subject, source.object.episode_id, None)
            .await
            .unwrap();
        assert_eq!(
            current.object.acceptance_state,
            nous_memory::AcceptanceState::Accepted
        );
        assert_eq!(
            current.object.current_revision_id,
            source.object.current_revision_id
        );
    }
}

async fn assert_partition_revision(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    merged: &[EpisodeView],
    occurrences: &[nous_core::OccurrenceId],
) {
    let memory = rt.require_memory().unwrap();
    let mut revise = partition_request(rt, subject, merged, occurrences, &[vec![0, 1, 2, 3]]).await;
    revise.segments[0].title = Some("reinterpreted scope".into());
    let revised = memory
        .apply_episode_partition(revise.clone())
        .await
        .unwrap();
    assert_eq!(revised[0].object.episode_id, merged[0].object.episode_id);
    assert_ne!(
        revised[0].revision.episode_revision_id,
        merged[0].revision.episode_revision_id
    );
    assert_eq!(
        memory
            .episode_history(subject, revised[0].object.episode_id)
            .await
            .unwrap()
            .len(),
        2
    );
    let mut unchanged =
        partition_request(rt, subject, &revised, occurrences, &[vec![0, 1, 2, 3]]).await;
    unchanged.segments[0].title = revised[0].revision.title.clone();
    let watermark = rt.store.authority_seq(subject).await.unwrap();
    let no_change = memory.apply_episode_partition(unchanged).await.unwrap();
    assert_eq!(
        no_change[0].revision.episode_revision_id,
        revised[0].revision.episode_revision_id
    );
    assert_eq!(rt.store.authority_seq(subject).await.unwrap(), watermark);
    revise.operation_id = nous_core::OperationId::new();
    assert!(memory.apply_episode_partition(revise).await.is_err());
}

#[tokio::test]
async fn journal_lineage_revalidation_and_receipt_are_exact() {
    use nous_core::{
        BasisRole, CognitionDependency, CognitiveRef, IntegrityState, OperationId, RevisionBasis,
    };
    use nous_memory::{JournalInput, JournalPoint, JournalPointRole};
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock_serving(&url, &root, clock.clone(), true).await;
    let subject = create_subject(&rt).await;
    let episode = journal_source_episode(&rt, &clock, subject).await;
    assert_maintenance_planning(&rt, &clock, subject).await;
    let memory = rt.require_memory().unwrap();
    let basis = RevisionBasis::CognitionDependency(CognitionDependency {
        epistemic_relation: None,
        target_revision: CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
        basis_role: BasisRole::Direct,
    });
    let input = JournalInput {
        operation_id: OperationId::new(),
        subject,
        expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
        target: None,
        sources: vec![EpisodePartitionSource {
            revision: episode.revision.episode_revision_id,
            expected_epoch: episode.object.object_epoch,
        }],
        title: Some("Experience summary".into()),
        narrative: "Two sources contributed.".into(),
        points: vec![JournalPoint {
            role: JournalPointRole::Summary,
            text: "Point-only detail: two independent sources.".into(),
            basis: vec![basis],
        }],
        producer: None,
    };
    let journal = memory.commit_journal(input.clone()).await.unwrap();
    let recall =
        assert_longitudinal_materialization(&rt, &clock, subject, &episode, &journal).await;
    let dependencies = create_journal_dependents(&rt, subject, &episode, &journal).await;
    let basis = RevisionBasis::CognitionDependency(CognitionDependency {
        epistemic_relation: None,
        target_revision: CognitiveRef::JournalRevision(journal.revision.journal_revision_id),
        basis_role: BasisRole::Direct,
    });
    let provenance = memory
        .provenance_summary(subject, std::slice::from_ref(&basis))
        .await
        .unwrap();
    assert_eq!(provenance.roots.len(), 2);
    assert_eq!(provenance.normalized_inputs.len(), 1);
    let mut invalid = input.clone();
    invalid.operation_id = OperationId::new();
    invalid.expected_authority_seq = rt.store.authority_seq(subject).await.unwrap();
    invalid.points[0].basis = vec![basis];
    assert!(memory.commit_journal(invalid).await.is_err());
    memory
        .suppress_episode(
            subject,
            episode.object.episode_id,
            OperationId::new(),
            episode.object.object_epoch,
        )
        .await
        .unwrap();
    let invalidated = memory
        .journal(subject, journal.object.journal_id, None)
        .await
        .unwrap();
    assert!(matches!(
        invalidated.object.integrity_state,
        IntegrityState::RevalidationRequired
    ));
    assert_eq!(
        invalidated.object.object_epoch,
        journal.object.object_epoch + 1
    );
    assert_eq!(invalidated.revision.narrative, journal.revision.narrative);
    assert!(rt.query(recall).await.unwrap().results.is_empty());
    assert_invalidated_dependents(&rt, subject, &dependencies).await;
    let needs = rt.cognition.maintenance_needs(subject).await.unwrap();
    assert!(needs.iter().any(|need| need.kind == "journal_revalidate"
        && need.scope_ref == journal.object.journal_id.0.to_string()));
    clock.advance_by(subject, Duration::days(1)).unwrap();
    let replay = memory.commit_journal(input.clone()).await.unwrap();
    assert_eq!(
        replay.revision.journal_revision_id,
        journal.revision.journal_revision_id
    );
    assert_eq!(replay.revision.formed_at, journal.revision.formed_at);
    assert_eq!(replay.revision.recorded_at, journal.revision.recorded_at);
    let revised = assert_journal_revalidation(&rt, input, &episode, &invalidated).await;
    assert_journal_protocol(&rt, subject, &revised).await;
    assert_journal_consolidation(&rt, subject, &episode, &revised).await;
    let uses = assert_longitudinal_use(&rt, subject, &episode, &revised).await;
    assert_journal_purge(&rt, subject, &revised, &episode, uses, &dependencies).await;
}

async fn assert_longitudinal_materialization(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
    episode: &EpisodeView,
    journal: &nous_memory::JournalView,
) -> nous_core::CognitiveQuery {
    use nous_core::{
        CognitiveQuery, CognitiveQueryExpr, CognitiveRef, QueryOperation, QueryTarget, ResultNeed,
    };
    let bound = rt
        .store
        .bind_exact_reference(subject, &CognitiveRef::Journal(journal.object.journal_id))
        .await
        .unwrap();
    assert_eq!(
        bound.0,
        CognitiveRef::JournalRevision(journal.revision.journal_revision_id)
    );
    assert_eq!(bound.1, Some(journal.object.object_epoch));
    assert!(bound.2);
    let refs = [
        CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
        CognitiveRef::JournalRevision(journal.revision.journal_revision_id),
    ];
    let query = CognitiveQuery {
        projection: Default::default(),
        temporal_frame: Default::default(),

        work_context: None,
        api_version: nous_core::API_VERSION,
        subject,
        session: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            operation: QueryOperation::Atom,
            targets: refs
                .iter()
                .cloned()
                .map(|reference| QueryTarget::Exact { reference })
                .collect(),
            cues: vec![Cue::Text(nous_core::TextCue {
                text: "Inspect the selected experience".into(),
            })],
            constraints: Default::default(),
            preferences: vec![],
            children: vec![],
        },
        exploration: Default::default(),
        resources: Default::default(),
        result_need: ResultNeed {
            limit: 8,
            need_evidence: true,
            ..Default::default()
        },
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: Default::default(),
    };
    let result = rt.query(query.clone()).await.unwrap();
    assert_eq!(result.results.len(), 2);
    for reference in &refs {
        let hit = result
            .results
            .iter()
            .find(|hit| &hit.reference == reference)
            .unwrap();
        assert_eq!(hit.revision.as_ref(), Some(reference));
        assert!(!hit.evidence.is_empty());
        let text = hit.representation.as_ref().unwrap();
        assert!(text.len() <= 65536);
        if reference == &refs[0] {
            assert!(text.contains(&"x".repeat(2032)));
            assert!(text.contains("object:source-b"));
            assert!(!text.contains('界'));
        } else {
            assert!(text.contains("Two sources contributed."));
            assert!(text.contains("Point-only detail: two independent sources."));
        }
    }
    let mut filtered = query.clone();
    filtered.expression.constraints.source_classes_include = vec![SourceClass::Message];
    assert_eq!(rt.query(filtered.clone()).await.unwrap().results.len(), 2);
    filtered.expression.constraints.source_classes_exclude = vec![SourceClass::Message];
    assert!(rt.query(filtered).await.unwrap().results.is_empty());
    assert_longitudinal_lanes(rt, &query, &refs).await;
    clock.advance_by(subject, Duration::days(730)).unwrap();
    assert_longitudinal_lanes(rt, &query, &refs).await;
    let projection = rt
        .store
        .text_projection_input(
            subject,
            "lexical",
            "",
            true,
            rt.configuration
                .snapshot_for_subject(subject)
                .unwrap()
                .get(nous_memory::EPISODE_SYNOPSIS)
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(refs.iter().all(|reference| {
        projection
            .sources
            .iter()
            .any(|source| &source.reference == reference)
    }));
    let source = projection
        .sources
        .iter()
        .find(|source| source.reference == refs[1])
        .unwrap();
    assert!(source.text.as_ref().unwrap().contains("Point-only detail"));
    query
}

async fn journal_source_episode(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
) -> EpisodeView {
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    for source in ["object:source-a", "object:source-b"] {
        let mut input = observation(subject, Some(session.session_id));
        input.material = ObservationMaterial::InlineText {
            text: if source == "object:source-a" {
                format!("{source}{}界", "x".repeat(2047 - source.len()))
            } else {
                source.into()
            },
            media_type: "text/plain".into(),
        };
        input.occurrence.external_object_ref = Some(nous_core::ObjectRef::new(source).unwrap());
        rt.material
            .record_observation_once(input, Some(uuid::Uuid::new_v4()))
            .await
            .unwrap();
        clock.advance_by(subject, Duration::seconds(1)).unwrap();
    }
    rt.organize_experience(subject, 128, true)
        .await
        .unwrap()
        .episodes
        .remove(0)
}

async fn assert_journal_revalidation(
    rt: &NousRuntime,
    mut input: nous_memory::JournalInput,
    episode: &EpisodeView,
    invalidated: &nous_memory::JournalView,
) -> nous_memory::JournalView {
    let subject = input.subject;
    let memory = rt.require_memory().unwrap();
    let restored = memory
        .restore_episode(
            subject,
            episode.object.episode_id,
            nous_core::OperationId::new(),
            episode.object.object_epoch + 1,
        )
        .await
        .unwrap();
    input.operation_id = nous_core::OperationId::new();
    input.expected_authority_seq = rt.store.authority_seq(subject).await.unwrap();
    input.sources[0].expected_epoch = restored.object.object_epoch;
    input.target = Some(nous_memory::JournalTarget {
        journal_id: invalidated.object.journal_id,
        expected_revision: invalidated.revision.journal_revision_id,
        expected_epoch: invalidated.object.object_epoch,
        intent: "revalidate".into(),
    });
    let revised = memory.commit_journal(input.clone()).await.unwrap();
    assert_eq!(revised.object.journal_id, invalidated.object.journal_id);
    assert_ne!(
        revised.revision.journal_revision_id,
        invalidated.revision.journal_revision_id
    );
    assert!(matches!(
        revised.object.integrity_state,
        nous_core::IntegrityState::Valid
    ));
    assert_eq!(
        memory
            .journal_history(subject, revised.object.journal_id)
            .await
            .unwrap()
            .len(),
        2
    );
    input.operation_id = nous_core::OperationId::new();
    input.expected_authority_seq = rt.store.authority_seq(subject).await.unwrap();
    assert!(memory.commit_journal(input).await.is_err());
    revised
}

async fn assert_longitudinal_use(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    episode: &EpisodeView,
    journal: &nous_memory::JournalView,
) -> nous_runtime::UseFeedback {
    let references = vec![
        nous_core::CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
        nous_core::CognitiveRef::JournalRevision(journal.revision.journal_revision_id),
    ];
    let context = rt
        .cognition
        .create_work_context(nous_runtime::CreateWorkContextInput {
            operation_id: nous_core::OperationId::new(),
            subject,
            purpose: "Continue longitudinal cognition".into(),
            unresolved_questions: vec![],
            constraints: serde_json::json!({}),
            resume_conditions: vec![],
            budget_summary: serde_json::json!({}),
            context_text: String::new(),
            entity_anchors: vec![],
            tag_anchors: vec![],
            cognition_anchors: references.clone(),
        })
        .await
        .unwrap();
    assert_eq!(context.cognition_anchors, references);
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let input = nous_runtime::UseFeedback {
        subject,
        session_id: Some(session.session_id),
        consumer_ref: "consumer:test:longitudinal".into(),
        events: references
            .into_iter()
            .map(|reference| nous_runtime::UseFeedbackEvent {
                query_id: None,
                event_id: nous_core::UseEventId::new(),
                reference,
                use_kind: nous_runtime::UseKind::Referenced,
                occurred_at: rt.cognition.now(subject),
                context: serde_json::json!({}),
            })
            .collect(),
    };
    let result = rt.cognition.use_feedback(input.clone()).await.unwrap();
    assert_eq!((result.0, result.1), (2, 0));
    let result = rt.cognition.use_feedback(input.clone()).await.unwrap();
    assert_eq!((result.0, result.1), (0, 2));
    assert_eq!(
        rt.cognition
            .session(subject, session.session_id)
            .await
            .unwrap()
            .resident
            .len(),
        2
    );
    input
}

async fn assert_journal_protocol(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    journal: &nous_memory::JournalView,
) {
    use nous_protocol::{
        kernel::kernel_maintenance_service_server::KernelMaintenanceService, public as p,
    };
    let service = nous_kernel::transport::KernelService(rt.clone());
    let current = service
        .get_journal(tonic::Request::new(p::ObjectRequest {
            subject_id: subject.0.to_string(),
            id: journal.object.journal_id.0.to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        current.journal.unwrap().current_revision.unwrap().points[0]
            .basis
            .len(),
        1
    );
    let listed = service
        .list_journals(tonic::Request::new(p::ListRequest {
            subject_id: subject.0.to_string(),
            page: Some(p::Page {
                page_size: 1,
                page_token: String::new(),
            }),
            status: "accepted".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        listed.items[0].journal_id,
        journal.object.journal_id.0.to_string()
    );
    let request = p::ListJournalRevisionsRequest {
        subject_id: subject.0.to_string(),
        journal_id: journal.object.journal_id.0.to_string(),
        page: Some(p::Page {
            page_size: 1,
            page_token: String::new(),
        }),
    };
    let first = service
        .list_journal_revisions(tonic::Request::new(request.clone()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(first.items.len(), 1);
    assert!(!first.next_page_token.is_empty());
    let mut next = request;
    next.page.as_mut().unwrap().page_token = first.next_page_token.clone();
    let second = service
        .list_journal_revisions(tonic::Request::new(next))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        second.items[0].journal_revision_id,
        journal.revision.journal_revision_id.0.to_string()
    );
    assert!(second.next_page_token.is_empty());
    assert!(
        service
            .list_journals(tonic::Request::new(p::ListRequest {
                status: String::new(),
                subject_id: subject.0.to_string(),
                page: Some(p::Page {
                    page_size: 1,
                    page_token: first.next_page_token
                })
            }))
            .await
            .is_err()
    );
    let policy = service
        .get_maintenance_policy(tonic::Request::new(p::SubjectRequest {
            subject_id: subject.0.to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(policy.enabled);
    assert_eq!(policy.max_operations, 4);
}

async fn assert_maintenance_planning(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
) {
    use nous_protocol::{
        kernel as k, kernel::kernel_maintenance_service_server::KernelMaintenanceService,
    };
    let service = nous_kernel::transport::KernelService(rt.clone());
    clock.advance_by(subject, Duration::seconds(300)).unwrap();
    for kind in ["episode_resegment", "journal_review", "memory_consolidate"] {
        let claim = service
            .claim_maintenance(tonic::Request::new(k::ClaimMaintenanceRequest {
                subject_id: subject.0.to_string(),
                allowed_kinds: vec![kind.into()],
                limit: 1,
                lease_seconds: 60,
                model_execution_digest: None,
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(claim.needs.len(), 1);
        let claimed = claim.needs[0].clone();
        let plan = service
            .plan_maintenance(tonic::Request::new(k::PlanMaintenanceRequest {
                claimed: Some(claimed.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.members.len(), 2);
        assert!(plan.members[0].recorded_seq < plan.members[1].recorded_seq);
        assert!(plan.members[0].text.starts_with("object:source-a"));
        assert_eq!(plan.members[0].text.len(), 2047);
        assert!(!plan.members[0].text.contains(char::REPLACEMENT_CHARACTER));
        assert!(plan.basis.len() >= 3);
        if kind == "memory_consolidate" {
            assert!(plan.consolidation_source.is_some());
            assert_eq!(plan.provenance_roots.len(), 2);
            assert_eq!(plan.max_consolidation_actions, 8);
        }
        let finish = k::FinishMaintenanceRequest {
            claimed: Some(claimed),
            disposition: "satisfied".into(),
            retry_delay_seconds: 0,
            next_due: None,
            problem_code: None,
        };
        service
            .finish_maintenance(tonic::Request::new(finish.clone()))
            .await
            .unwrap();
        service
            .finish_maintenance(tonic::Request::new(finish))
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn ended_work_context_wakes_and_closes_experience_before_idle_deadline() {
    use nous_core::OperationId;
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let context = rt
        .cognition
        .create_work_context(nous_runtime::CreateWorkContextInput {
            operation_id: OperationId::new(),
            subject,
            purpose: "Bounded work".into(),
            unresolved_questions: vec![],
            constraints: serde_json::json!({}),
            resume_conditions: vec![],
            budget_summary: serde_json::json!({}),
            context_text: String::new(),
            entity_anchors: vec![],
            tag_anchors: vec![],
            cognition_anchors: vec![],
        })
        .await
        .unwrap();
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    rt.cognition
        .set_active_work_context(
            subject,
            session.session_id,
            Some(context.work_context_id),
            session.runtime_revision,
            OperationId::new(),
        )
        .await
        .unwrap();
    rt.material
        .record_observation_once(
            observation(subject, Some(session.session_id)),
            Some(uuid::Uuid::new_v4()),
        )
        .await
        .unwrap();
    let initial = rt.organize_experience(subject, 128, false).await.unwrap();
    assert!(initial.episodes.is_empty());
    let claimed = rt
        .cognition
        .lease_maintenance(subject, &["episode_segment".into()], 1, 60, None)
        .await
        .unwrap();
    rt.cognition
        .acknowledge_maintenance(
            &claimed[0],
            MaintenanceDisposition::Pending {
                due_at: initial.next_due.unwrap(),
                problem_code: None,
            },
        )
        .await
        .unwrap();
    rt.cognition
        .end_work_context(
            subject,
            context.work_context_id,
            context.revision,
            OperationId::new(),
        )
        .await
        .unwrap();
    let needs = rt
        .cognition
        .lease_maintenance(subject, &["episode_segment".into()], 1, 60, None)
        .await
        .unwrap();
    assert_eq!(needs.len(), 1);
    let closed = rt.organize_experience(subject, 128, false).await.unwrap();
    assert_eq!(closed.episodes.len(), 1);
    assert_eq!(
        closed.episodes[0].revision.boundary_explanation,
        "work_context_ended"
    );
}

fn consolidation_producer() -> nous_core::ProducerSignature {
    nous_core::ProducerSignature {
        model_role: None,
        model_profile: None,
        execution_profile: None,
        inference_controls_digest: None,
        role_policy_digest: None,
        prompt_id: None,
        prompt_digest: None,

        signature_hash: String::new(),
        provider_class: "semantic-stub".into(),
        operation: nous_core::CapabilityOperation::MemoryConsolidationText,
        implementation: "longitudinal-owner-test".into(),
        model_identity: Some("stub".into()),
        model_revision: None,
        output_schema_digest: Some("a".repeat(64)),
        preprocessing_identity: "memory/consolidation.md".into(),
        preprocessing_revision: "b".repeat(64),
        config_digest: "c".repeat(64),
    }
}
fn consolidation_memory(episode: &EpisodeView) -> nous_memory::ExplicitMemoryInput {
    let nous_core::CognitiveRef::Occurrence(occurrence) = episode.members[0].reference else {
        panic!("expected occurrence")
    };
    nous_memory::ExplicitMemoryInput {
        producer: Some(consolidation_producer()),
        operation_id: nous_core::OperationId::new(),
        subject: episode.object.subject_id,
        tags: Vec::new(),
        cognitive_role: nous_memory::CognitiveRole::Declarative,
        formation_mode: nous_memory::FormationMode::Grounded,
        grounding_occurrence_id: Some(occurrence),
        semantic_role: "statement".into(),
        representation_text: "A reusable observed fact.".into(),
        title: None,
        basis: vec![nous_core::RevisionBasis::Evidence(nous_core::EvidenceRef {
            epistemic_relation: None,
            occurrence_id: occurrence,
            locator: nous_core::EvidenceLocator::WholeOccurrence,
            basis_role: nous_core::BasisRole::Direct,
        })],
        aboutness: vec![],
        valid_time: TemporalExtent::Unknown,
        epistemic_class: nous_core::EpistemicClass::Derived,
    }
}
fn consolidation_schema(episode: &EpisodeView) -> nous_memory::CreateSchemaInput {
    nous_memory::CreateSchemaInput {
        producer: Some(consolidation_producer()),
        operation_id: nous_core::OperationId::new(),
        subject: episode.object.subject_id,
        title: Some("Recurring pattern".into()),
        structural_claim: "Two independent sources describe a recurring pattern.".into(),
        applicability_scope: nous_memory::SchemaScope {
            description: "The observed contexts".into(),
            aboutness: vec![],
            tags: vec![],
            valid_time: TemporalExtent::Unknown,
        },
        boundary_definition: "Applies to these observed contexts.".into(),
        formation_kind: nous_memory::SchemaFormationKind::Synthesized,
        evidence_links: episode
            .basis
            .iter()
            .cloned()
            .map(|basis| nous_memory::SchemaEvidenceLinkInput {
                role: nous_memory::SchemaEvidenceRole::Support,
                basis,
            })
            .collect(),
    }
}

#[tokio::test]
async fn typed_cognition_actions_preserve_prior_commits_and_exact_schema_replay() {
    use nous_core::OperationId;
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock_serving(&url, &root, clock.clone(), true).await;
    let subject = create_subject(&rt).await;
    let episode = journal_source_episode(&rt, &clock, subject).await;
    let owner = rt.require_memory().unwrap();
    let first_input = consolidation_memory(&episode);
    let first = owner.form_memory(first_input.clone()).await.unwrap();
    let second = owner
        .form_memory(consolidation_memory(&episode))
        .await
        .unwrap();
    let schema_input = consolidation_schema(&episode);
    let schema = owner.create_schema(schema_input.clone()).await.unwrap();
    let mut invalid = consolidation_schema(&episode);
    invalid.boundary_definition.clear();
    assert!(owner.create_schema(invalid).await.is_err());
    assert_eq!(
        owner
            .form_memory(first_input.clone())
            .await
            .unwrap()
            .revision
            .memory_revision_id,
        first.revision.memory_revision_id
    );
    owner
        .link_revisions(
            subject,
            OperationId::new(),
            first.revision.memory_revision_id,
            second.revision.memory_revision_id,
            nous_memory::MemoryRelation::Elaborates,
        )
        .await
        .unwrap();
    let results = [
        Some(nous_core::CognitiveRef::MemoryRevision(
            first.revision.memory_revision_id,
        )),
        Some(nous_core::CognitiveRef::MemoryRevision(
            second.revision.memory_revision_id,
        )),
        Some(nous_core::CognitiveRef::CognitiveSchemaRevision(
            schema.revision.schema_revision_id,
        )),
    ];
    clock.advance_by(subject, Duration::days(1)).unwrap();
    assert_consolidation_context(&rt, subject, &results).await;
    let revised_input = nous_memory::ReviseSchemaInput {
        formation_kind: schema.revision.formation_kind,
        producer: Some(consolidation_producer()),
        evidence_links: schema_input.evidence_links.clone(),
        operation_id: OperationId::new(),
        subject,
        schema_id: schema.schema.schema_id,
        expected_object_epoch: schema.schema.object_epoch,
        intent: nous_memory::RevisionIntent::Rephrase,
        title: schema.revision.title.clone(),
        structural_claim: "A clarified recurring pattern.".into(),
        applicability_scope: schema.revision.applicability_scope.clone(),
        boundary_definition: schema.revision.boundary_definition.clone(),
        copy_link_ids: Vec::new(),
    };
    let revised = owner.revise_schema(revised_input.clone()).await.unwrap();
    assert!(revised.revision.producer_signature_id.is_some());
    assert_ne!(
        revised.revision.schema_revision_id,
        schema.revision.schema_revision_id
    );
    assert_eq!(
        owner
            .create_schema(schema_input)
            .await
            .unwrap()
            .revision
            .schema_revision_id,
        schema.revision.schema_revision_id
    );
    let mut next = revised_input.clone();
    next.operation_id = OperationId::new();
    next.expected_object_epoch = revised.schema.object_epoch;
    owner.revise_schema(next).await.unwrap();
    assert_eq!(
        owner
            .revise_schema(revised_input)
            .await
            .unwrap()
            .revision
            .schema_revision_id,
        revised.revision.schema_revision_id
    );
}

async fn assert_journal_consolidation(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    episode: &EpisodeView,
    journal: &nous_memory::JournalView,
) {
    let mut input = consolidation_schema(episode);
    input
        .evidence_links
        .push(nous_memory::SchemaEvidenceLinkInput {
            role: nous_memory::SchemaEvidenceRole::Support,
            basis: nous_core::RevisionBasis::CognitionDependency(nous_core::CognitionDependency {
                epistemic_relation: None,
                target_revision: nous_core::CognitiveRef::JournalRevision(
                    journal.revision.journal_revision_id,
                ),
                basis_role: nous_core::BasisRole::Direct,
            }),
        });
    let schema = rt
        .require_memory()
        .unwrap()
        .create_schema(input)
        .await
        .unwrap();
    let revision = schema.revision.schema_revision_id;
    let row = sqlx::query(
        "SELECT formed_at,recorded_at FROM cognitive_schema_revisions WHERE schema_revision_id=$1",
    )
    .bind(revision.0)
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(
        row.get::<chrono::DateTime<Utc>, _>("formed_at"),
        rt.cognition.now(subject)
    );
    assert_eq!(
        row.get::<chrono::DateTime<Utc>, _>("recorded_at"),
        rt.cognition.now(subject)
    );
}

async fn assert_longitudinal_lanes(
    rt: &NousRuntime,
    exact: &nous_core::CognitiveQuery,
    refs: &[nous_core::CognitiveRef; 2],
) {
    use nous_core::{Cue, EvidenceFamily, ResultDomain, TextCue};
    for (domain, text, expected) in [
        (ResultDomain::Episode, "object source", &refs[0]),
        (ResultDomain::Journal, "Point-only detail", &refs[1]),
    ] {
        let mut query = exact.clone();
        query.expression.targets.clear();
        query.projection.domains = vec![domain];
        query.expression.cues = vec![Cue::Text(TextCue { text: text.into() })];
        query.result_need.limit = 1;
        let result = rt.query(query).await.unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(&result.results[0].reference, expected);
        assert!(
            result.results[0]
                .match_evidence
                .families
                .contains(&EvidenceFamily::Lexical)
        );
        assert!(
            result.results[0]
                .match_evidence
                .families
                .contains(&EvidenceFamily::Dense)
        );
    }
    assert_longitudinal_query_protocol(rt, exact.subject, refs).await;
    let mut scoped = exact.clone();
    scoped.projection.domains = vec![ResultDomain::Memory];
    assert!(rt.query(scoped.clone()).await.unwrap().results.is_empty());
    scoped.projection.domains = vec![ResultDomain::Journal];
    assert_eq!(
        rt.query(scoped).await.unwrap().results[0].reference,
        refs[1]
    );
}

async fn assert_longitudinal_query_protocol(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    refs: &[nous_core::CognitiveRef; 2],
) {
    use k::kernel_query_service_server::KernelQueryService as Kernel;
    use nous_protocol::{kernel as k, public as p};
    let service = nous_kernel::transport::KernelService(rt.clone());
    for (domain, text, expected) in [
        ("episode", "object source", &refs[0]),
        ("journal", "Point-only detail", &refs[1]),
    ] {
        let prepared = Kernel::prepare_query(
            &service,
            tonic::Request::new(k::PrepareQueryRequest {
                query: Some(p::QueryRequest {
                    subject_id: subject.0.to_string(),
                    expression: Some(p::QueryExpr {
                        operation: "atom".into(),
                        cues: vec![p::Cue {
                            cue: Some(p::cue::Cue::Text(text.into())),
                        }],
                        modifiers: Some(p::QueryModifiers {
                            projection: Some(p::ResultProjection {
                                domains: vec![domain.into()],
                            }),
                            limit: Some(1),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                reserve_execution: true,
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let response = Kernel::query(
            &service,
            tonic::Request::new(k::KernelQueryRequest {
                preparation_token: prepared.preparation_token.unwrap(),
                subject_id: subject.0.to_string(),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner()
        .response
        .unwrap();
        assert_eq!(response.hits.len(), 1);
        let reference = response.hits[0].reference.as_ref().unwrap();
        assert_eq!(
            format!("{}:{}", reference.kind, reference.value),
            expected.to_string()
        );
    }
}

async fn create_journal_dependents(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    episode: &EpisodeView,
    journal: &nous_memory::JournalView,
) -> Vec<nous_core::CognitiveRef> {
    use nous_core::{BasisRole, CognitionDependency, CognitiveRef, OperationId, RevisionBasis};
    use nous_memory::FormationMode;
    let journal_ref = CognitiveRef::JournalRevision(journal.revision.journal_revision_id);
    let mut content = consolidation_memory(episode);
    content.formation_mode = FormationMode::Synthesized;
    content.grounding_occurrence_id = None;
    content.basis = [
        journal_ref.clone(),
        CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
    ]
    .into_iter()
    .map(|target_revision| {
        RevisionBasis::CognitionDependency(CognitionDependency {
            epistemic_relation: None,
            target_revision,
            basis_role: BasisRole::Direct,
        })
    })
    .collect();
    let memory = rt.require_memory().unwrap();
    let mut refs = Vec::new();
    for _ in 0..2 {
        let mut input = content.clone();
        input.operation_id = OperationId::new();
        let result = memory.form_memory(input).await.unwrap();
        refs.push(CognitiveRef::MemoryRevision(
            result.revision.memory_revision_id,
        ));
    }
    let mut schema = consolidation_schema(episode);
    schema.evidence_links = refs
        .iter()
        .cloned()
        .map(|target_revision| nous_memory::SchemaEvidenceLinkInput {
            role: nous_memory::SchemaEvidenceRole::Support,
            basis: RevisionBasis::CognitionDependency(CognitionDependency {
                epistemic_relation: None,
                target_revision,
                basis_role: BasisRole::Direct,
            }),
        })
        .collect();
    let result = memory.create_schema(schema).await.unwrap();
    refs.push(CognitiveRef::CognitiveSchemaRevision(
        result.revision.schema_revision_id,
    ));
    let episode = memory
        .create_episode(nous_memory::EpisodeInput {
            operation_id: OperationId::new(),
            subject,
            track_key: "manual-dependent".into(),
            title: None,
            parent_episode_revision_id: None,
            experience_time: TemporalExtent::Unknown,
            boundary_explanation: "Supported cognitive continuation.".into(),
            producer_signature_id: None,
            members: vec![nous_memory::EpisodeMemberInput {
                reference: refs[0].clone(),
                role: "context".into(),
            }],
            basis: vec![RevisionBasis::CognitionDependency(CognitionDependency {
                epistemic_relation: None,
                target_revision: refs[0].clone(),
                basis_role: BasisRole::Direct,
            })],
        })
        .await
        .unwrap();
    refs.push(CognitiveRef::EpisodeRevision(
        episode.revision.episode_revision_id,
    ));
    refs
}

async fn assert_invalidated_dependents(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    refs: &[nous_core::CognitiveRef],
) {
    for reference in refs {
        let query = match reference {
            nous_core::CognitiveRef::MemoryRevision(_) => {
                "SELECT o.integrity_state,o.object_epoch FROM memory_objects o JOIN memory_revisions r USING(memory_id) WHERE o.subject_id=$1 AND r.memory_revision_id=$2"
            }
            nous_core::CognitiveRef::CognitiveSchemaRevision(_) => {
                "SELECT o.integrity_state,o.object_epoch FROM cognitive_schemas o JOIN cognitive_schema_revisions r USING(schema_id) WHERE o.subject_id=$1 AND r.schema_revision_id=$2"
            }
            nous_core::CognitiveRef::EpisodeRevision(_) => {
                "SELECT o.integrity_state,o.object_epoch FROM episode_objects o JOIN episode_revisions r USING(episode_id) WHERE o.subject_id=$1 AND r.episode_revision_id=$2"
            }
            _ => panic!("unexpected dependency kind"),
        };
        let id = reference
            .to_string()
            .split_once(':')
            .unwrap()
            .1
            .parse::<uuid::Uuid>()
            .unwrap();
        let row = sqlx::query(query)
            .bind(subject.0)
            .bind(id)
            .fetch_one(rt.store.pool())
            .await
            .unwrap();
        assert_eq!(
            row.get::<String, _>("integrity_state"),
            "revalidation_required"
        );
        assert_eq!(row.get::<i64, _>("object_epoch"), 2);
    }
    let schemas = refs
        .iter()
        .filter_map(|reference| match reference {
            nous_core::CognitiveRef::CognitiveSchemaRevision(id) => Some(id.0.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let edges: i64 = sqlx::query_scalar("SELECT count(*) FROM cognition_dependency_invalidations WHERE subject_id=$1 AND dependent_kind='cognitive_schema_revision' AND dependent_ref=ANY($2::text[]) AND invalidated_by_kind='memory_revision'")
        .bind(subject.0).bind(schemas).fetch_one(rt.store.pool()).await.unwrap();
    assert_eq!(edges, 2);
}

async fn assert_journal_purge(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    journal: &nous_memory::JournalView,
    episode: &EpisodeView,
    uses: nous_runtime::UseFeedback,
    dependencies: &[nous_core::CognitiveRef],
) {
    use nous_core::OperationId;
    let memory = rt.require_memory().unwrap();
    let fresh_dependencies = create_journal_dependents(rt, subject, episode, journal).await;
    let before = rt.store.authority_seq(subject).await.unwrap();
    memory
        .purge_journal(
            subject,
            journal.object.journal_id,
            OperationId::new(),
            journal.object.object_epoch,
        )
        .await
        .unwrap();
    assert!(
        memory
            .journal(subject, journal.object.journal_id, None)
            .await
            .is_err()
    );
    assert!(
        memory
            .episode_revision(subject, episode.revision.episode_revision_id)
            .await
            .is_ok()
    );
    assert_eq!(rt.cognition.use_feedback(uses).await.unwrap().1, 2);
    assert_invalidated_dependents(rt, subject, dependencies).await;
    assert_eq!(rt.store.authority_seq(subject).await.unwrap(), before + 1);
    assert_invalidated_dependents(rt, subject, &fresh_dependencies).await;
}

#[tokio::test]
async fn episode_media_synopsis_tracks_ready_derivation_without_revising_authority() {
    use nous_core::CognitiveRef;
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock_serving(&url, &root, clock.clone(), true).await;
    let subject = create_subject(&rt).await;
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let artifact = rt
        .material
        .upload_stream(
            subject,
            nous_material::UploadMetadata {
                media_type: "image/png".into(),
                metadata: serde_json::json!({}),
            },
            futures::stream::iter([Ok(vec![137, 80, 78, 71, 13, 10, 26, 10])]),
        )
        .await
        .unwrap();
    let mut input = observation(subject, Some(session.session_id));
    input.material = ObservationMaterial::ArtifactRef {
        artifact_id: artifact.artifact_id,
    };
    let observed = rt.material.record_observation(input).await.unwrap();
    let region = observed.source_region.unwrap().source_region_id;
    let episode = rt
        .organize_experience(subject, 128, true)
        .await
        .unwrap()
        .episodes
        .remove(0);
    sqlx::query("INSERT INTO coverage_needs(coverage_need_id,subject_id,source_region_id,representation_kind,capability_operation,requirement,state,updated_at) VALUES($1,$2,$3,'image_description','image_interpretation','preferred','missing',$4)")
        .bind(uuid::Uuid::new_v4()).bind(subject.0).bind(region.0).bind(rt.cognition.now(subject)).execute(rt.store.pool()).await.unwrap();
    let query = media_episode_query(subject, "cobalt");
    assert!(rt.query(query.clone()).await.unwrap().results.is_empty());
    let synopsis = format!(
        "cobalt harbor {}界",
        "x".repeat(2047 - "cobalt harbor ".len())
    );
    let first = persist_media_synopsis(&rt, subject, region, &synopsis, None, "1").await;
    let results = rt.query(query.clone()).await.unwrap().results;
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].reference,
        CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id)
    );
    let text = results[0].representation.as_ref().unwrap();
    assert!(text.contains("cobalt harbor"));
    assert!(!text.contains('界'));
    assert!(results[0].evidence.iter().any(|basis| basis.reference
        == CognitiveRef::DerivedRepresentation(first)
        && basis.basis_role == "interpretation"));
    let fragments = rt
        .store
        .episode_member_text_input(
            subject,
            &[episode.revision.episode_revision_id.0],
            rt.configuration
                .snapshot_for_subject(subject)
                .unwrap()
                .get(nous_memory::EPISODE_SYNOPSIS)
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        matches!(&fragments[&episode.revision.episode_revision_id.0][0], nous_persistence::TextProjectionFragment::Text { reference:CognitiveRef::DerivedRepresentation(id),text } if *id==first && text.len()==2047)
    );
    assert_synopsis_policy(&rt, subject).await;
    let historical_cut = rt.cognition.now(subject);
    clock.advance_by(subject, Duration::seconds(10)).unwrap();
    let second =
        persist_media_synopsis(&rt, subject, region, "amber inlet", Some(first), "2").await;
    assert_historical_synopsis(&rt, subject, historical_cut, first, second).await;
    assert!(rt.query(query).await.unwrap().results.is_empty());
    let result = rt
        .query(media_episode_query(subject, "amber"))
        .await
        .unwrap();
    assert_eq!(
        result.results[0].reference,
        CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id)
    );
    assert!(
        result.results[0]
            .representation
            .as_ref()
            .unwrap()
            .contains("amber inlet")
    );
    let current = rt
        .require_memory()
        .unwrap()
        .episode(subject, episode.object.episode_id, None)
        .await
        .unwrap();
    assert_eq!(current.object.object_epoch, episode.object.object_epoch);
    assert_eq!(
        current.revision.episode_revision_id,
        episode.revision.episode_revision_id
    );
    assert_eq!(
        rt.require_memory()
            .unwrap()
            .provenance_summary(subject, &current.basis)
            .await
            .unwrap()
            .roots
            .len(),
        1
    );
}

async fn assert_synopsis_policy(rt: &NousRuntime, subject: nous_core::SubjectId) {
    let mut budget = rt
        .configuration
        .snapshot_for_subject(subject)
        .unwrap()
        .get(nous_memory::EPISODE_SYNOPSIS)
        .unwrap();
    budget.fragment_max_bytes = 7;
    rt.configuration
        .set_subject_override(
            nous_core::OperationId::new(),
            subject,
            nous_memory::EPISODE_SYNOPSIS.path(),
            serde_json::to_value(budget).unwrap(),
        )
        .await
        .unwrap();
    assert!(
        rt.query(media_episode_query(subject, "harbor"))
            .await
            .unwrap()
            .results
            .is_empty()
    );
    let results = rt
        .query(media_episode_query(subject, "cobalt"))
        .await
        .unwrap()
        .results;
    assert_eq!(results.len(), 1);
    assert!(
        !results[0]
            .representation
            .as_ref()
            .unwrap()
            .contains("harbor")
    );
    rt.configuration
        .clear_subject_override(
            nous_core::OperationId::new(),
            subject,
            nous_memory::EPISODE_SYNOPSIS.path(),
        )
        .await
        .unwrap();
    assert_eq!(
        rt.query(media_episode_query(subject, "harbor"))
            .await
            .unwrap()
            .results
            .len(),
        1
    );
}

fn media_episode_query(subject: nous_core::SubjectId, text: &str) -> nous_core::CognitiveQuery {
    use nous_core::{CognitiveQuery, CognitiveQueryExpr, Cue, TextCue};
    let mut query = CognitiveQuery {
        projection: Default::default(),
        temporal_frame: Default::default(),

        work_context: None,
        api_version: nous_core::API_VERSION,
        subject,
        session: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            cues: vec![Cue::Text(TextCue { text: text.into() })],
            ..Default::default()
        },
        exploration: Default::default(),
        resources: Default::default(),
        result_need: Default::default(),
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: Default::default(),
    };
    query.projection.domains = vec![nous_core::ResultDomain::Episode];
    query.capabilities.text_embedding = nous_core::RequirementStrength::Forbidden;
    query
}

async fn persist_media_synopsis(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    region: nous_core::SourceRegionId,
    text: &str,
    supersedes: Option<nous_core::DerivedRepresentationId>,
    model_revision: &str,
) -> nous_core::DerivedRepresentationId {
    let mut producer = consolidation_producer();
    producer.operation = nous_core::CapabilityOperation::ImageInterpretation;
    producer.implementation = "longitudinal-media-test".into();
    producer.model_revision = Some(model_revision.into());
    rt.material
        .persist_derived_representation(nous_material::DerivedRepresentation {
            derived_representation_id: nous_core::DerivedRepresentationId::new(),
            subject_id: subject,
            inputs: vec![nous_material::DerivationInput {
                ordinal: 0,
                reference: nous_core::CognitiveRef::SourceRegion(region),
                role: "source".into(),
            }],
            strategy: "direct_multimodal".into(),
            representation_kind: nous_core::RepresentationKind::ImageDescription,
            producer,
            revision: 1,
            payload_text: Some(text.into()),
            payload_json: None,
            payload_artifact_id: None,
            quality: serde_json::json!({}),
            created_at: Utc::now(),
            supersedes,
        })
        .await
        .unwrap()
        .derived_representation_id
}

async fn assert_consolidation_context(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    results: &[Option<nous_core::CognitiveRef>],
) {
    use nous_protocol::{
        kernel as k, kernel::kernel_maintenance_service_server::KernelMaintenanceService,
    };
    rt.cognition
        .use_feedback(nous_runtime::UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:test:context".into(),
            events: [
                nous_runtime::UseKind::Presented,
                nous_runtime::UseKind::Referenced,
            ]
            .into_iter()
            .map(|use_kind| nous_runtime::UseFeedbackEvent {
                query_id: None,
                event_id: nous_core::UseEventId::new(),
                reference: results[0].clone().unwrap(),
                use_kind,
                occurred_at: rt.cognition.now(subject),
                context: serde_json::json!({}),
            })
            .collect(),
        })
        .await
        .unwrap();
    let service = nous_kernel::transport::KernelService(rt.clone());
    let claim = service
        .claim_maintenance(tonic::Request::new(k::ClaimMaintenanceRequest {
            subject_id: subject.0.to_string(),
            allowed_kinds: vec!["memory_consolidate".into()],
            limit: 1,
            lease_seconds: 60,
            model_execution_digest: None,
        }))
        .await
        .unwrap()
        .into_inner();
    let need = claim.needs[0].clone();
    let plan = service
        .plan_maintenance(tonic::Request::new(k::PlanMaintenanceRequest {
            claimed: Some(need.clone()),
        }))
        .await
        .unwrap()
        .into_inner();
    let memory = plan
        .candidates
        .iter()
        .find(|candidate| candidate.key == results[0].as_ref().unwrap().to_string())
        .unwrap();
    assert_eq!(memory.r#use.len(), 1);
    assert_eq!(memory.r#use[0].kind, "referenced");
    assert_eq!(memory.r#use[0].count, 1);
    let schema = plan
        .candidates
        .iter()
        .find(|candidate| candidate.key == results[2].as_ref().unwrap().to_string())
        .unwrap();
    assert!(schema.text.contains("Applicability: The observed contexts"));
    assert!(
        schema
            .text
            .contains("Boundary: Applies to these observed contexts.")
    );
    assert!(schema.text.contains("Tags:"));
    assert!(schema.text.len() <= 4096);
    assert_eq!(schema.formation_mode, "synthesized");
    assert!(plan.candidates.iter().all(|candidate| {
        matches!(
            candidate
                .target
                .as_ref()
                .unwrap()
                .reference
                .as_ref()
                .unwrap()
                .kind
                .as_str(),
            "memory_revision" | "cognitive_schema_revision"
        )
    }));
    for candidate in &plan.candidates {
        let target = candidate.target.as_ref().unwrap();
        assert!(!target.object_id.is_empty());
        let own = target.reference.as_ref().unwrap();
        for key in &candidate.eligible_basis_keys {
            let basis = plan
                .basis
                .iter()
                .find(|basis| &basis.key == key)
                .unwrap()
                .basis
                .as_ref()
                .unwrap();
            if let Some(nous_protocol::public::revision_basis::Basis::CognitionDependency(
                dependency,
            )) = &basis.basis
            {
                assert_ne!(dependency.target_revision.as_ref().unwrap(), own);
            }
        }
    }
    assert_consolidation_policy(rt, subject, &service, &need).await;
    service
        .finish_maintenance(tonic::Request::new(k::FinishMaintenanceRequest {
            claimed: Some(need),
            disposition: "satisfied".into(),
            retry_delay_seconds: 0,
            ..Default::default()
        }))
        .await
        .unwrap();
}

async fn assert_consolidation_policy(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    service: &nous_kernel::transport::KernelService,
    need: &nous_protocol::kernel::MaintenanceNeed,
) {
    use nous_protocol::{
        kernel as k, kernel::kernel_maintenance_service_server::KernelMaintenanceService,
    };
    let mut policy = rt
        .configuration
        .snapshot_for_subject(subject)
        .unwrap()
        .get(nous_memory::CONSOLIDATION_CONTEXT)
        .unwrap();
    policy.candidate_text_chars = 64;
    policy.basis_limit = 1;
    policy.provenance_root_limit = 1;
    rt.configuration
        .set_subject_override(
            nous_core::OperationId::new(),
            subject,
            nous_memory::CONSOLIDATION_CONTEXT.path(),
            serde_json::to_value(policy).unwrap(),
        )
        .await
        .unwrap();
    let bounded = service
        .plan_maintenance(tonic::Request::new(k::PlanMaintenanceRequest {
            claimed: Some(need.clone()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!bounded.candidates.is_empty());
    assert!(
        bounded
            .candidates
            .iter()
            .all(|c| c.text.chars().count() <= 64)
    );
    assert_eq!(bounded.basis.len(), 1);
    assert!(bounded.basis_catalog_partial);
    assert!(bounded.provenance_roots.len() <= 1);
}

async fn manual_episode_sources(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
) -> (nous_core::OccurrenceId, EpisodeView, EpisodeView) {
    use nous_core::{BasisRole, CognitionDependency, CognitiveRef, OperationId, RevisionBasis};
    use nous_memory::{EpisodeInput, EpisodeMemberInput};
    let occurrence = rt
        .material
        .record_observation_once(observation(subject, None), Some(uuid::Uuid::new_v4()))
        .await
        .unwrap()
        .occurrence
        .occurrence_id;
    let memory = rt.require_memory().unwrap();
    let evidence = RevisionBasis::Evidence(nous_core::EvidenceRef {
        epistemic_relation: None,
        occurrence_id: occurrence,
        locator: nous_core::EvidenceLocator::WholeOccurrence,
        basis_role: BasisRole::Direct,
    });
    let first = memory
        .create_episode(EpisodeInput {
            operation_id: OperationId::new(),
            subject,
            track_key: "manual".into(),
            title: Some("Manual evidence".into()),
            parent_episode_revision_id: None,
            experience_time: TemporalExtent::Unknown,
            boundary_explanation: "Unbound observation.".into(),
            producer_signature_id: None,
            members: vec![EpisodeMemberInput {
                reference: CognitiveRef::Occurrence(occurrence),
                role: "evidence".into(),
            }],
            basis: vec![evidence],
        })
        .await
        .unwrap();
    let committed = memory
        .form_memory(consolidation_memory(&first))
        .await
        .unwrap();
    let reference = CognitiveRef::MemoryRevision(committed.revision.memory_revision_id);
    clock.advance_by(subject, Duration::seconds(1)).unwrap();
    let second = memory
        .create_episode(EpisodeInput {
            operation_id: OperationId::new(),
            subject,
            track_key: "manual".into(),
            title: Some("Cognitive continuation".into()),
            parent_episode_revision_id: None,
            experience_time: TemporalExtent::Unknown,
            boundary_explanation: "Memory member.".into(),
            producer_signature_id: None,
            members: vec![EpisodeMemberInput {
                reference: reference.clone(),
                role: "context".into(),
            }],
            basis: vec![RevisionBasis::CognitionDependency(CognitionDependency {
                epistemic_relation: None,
                target_revision: reference,
                basis_role: BasisRole::Direct,
            })],
        })
        .await
        .unwrap();
    (occurrence, first, second)
}

async fn assert_manual_consolidation_plans(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
    first: &EpisodeView,
    second: &EpisodeView,
) {
    use nous_protocol::{
        kernel as k, kernel::kernel_maintenance_service_server::KernelMaintenanceService,
    };
    clock.advance_by(subject, Duration::seconds(300)).unwrap();
    let service = nous_kernel::transport::KernelService(rt.clone());
    let claims = service
        .claim_maintenance(tonic::Request::new(k::ClaimMaintenanceRequest {
            subject_id: subject.0.to_string(),
            allowed_kinds: vec!["memory_consolidate".into()],
            limit: 8,
            lease_seconds: 60,
            model_execution_digest: None,
        }))
        .await
        .unwrap()
        .into_inner();
    for episode in [&first, &second] {
        let claimed = claims
            .needs
            .iter()
            .find(|need| need.scope_ref == episode.revision.episode_revision_id.0.to_string())
            .unwrap();
        let plan = service
            .plan_maintenance(tonic::Request::new(k::PlanMaintenanceRequest {
                claimed: Some(claimed.clone()),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.provenance_roots.len(), 1);
        if episode.object.episode_id == first.object.episode_id {
            assert_eq!(plan.members.len(), 1);
            assert_eq!(plan.members[0].text, "durable experience");
            assert!(plan.members[0].session_id.is_none());
            assert!(plan.members[0].recorded_seq.is_none());
        }
    }
}

#[tokio::test]
async fn manual_episode_planning_and_complete_journal_revalidation() {
    use nous_core::{BasisRole, CognitionDependency, CognitiveRef, OperationId, RevisionBasis};
    use nous_memory::{EpisodeMemberInput, JournalInput, JournalPoint, JournalPointRole};
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let (occurrence, first, second) = manual_episode_sources(&rt, &clock, subject).await;
    assert_manual_consolidation_plans(&rt, &clock, subject, &first, &second).await;
    let memory = rt.require_memory().unwrap();
    let journal = memory
        .commit_journal(JournalInput {
            operation_id: OperationId::new(),
            subject,
            expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
            target: None,
            sources: [&first, &second]
                .into_iter()
                .map(|episode| EpisodePartitionSource {
                    revision: episode.revision.episode_revision_id,
                    expected_epoch: episode.object.object_epoch,
                })
                .collect(),
            title: None,
            narrative: "Manual sources.".into(),
            points: vec![JournalPoint {
                role: JournalPointRole::Summary,
                text: "Supported continuation.".into(),
                basis: vec![RevisionBasis::CognitionDependency(CognitionDependency {
                    epistemic_relation: None,
                    target_revision: CognitiveRef::EpisodeRevision(
                        second.revision.episode_revision_id,
                    ),
                    basis_role: BasisRole::Direct,
                })],
            }],
            producer: None,
        })
        .await
        .unwrap();
    let consolidation = memory
        .plan_consolidation_scope(
            subject,
            "journal_revision",
            &journal.revision.journal_revision_id.0.to_string(),
        )
        .await
        .unwrap();
    assert_eq!(consolidation.status, "ready");
    assert_eq!(consolidation.occurrences, vec![occurrence]);
    let revised = memory
        .revise_episode(nous_memory::ReviseEpisodeInput {
            operation_id: OperationId::new(),
            subject,
            episode_id: second.object.episode_id,
            expected_object_epoch: second.object.object_epoch,
            intent: "reinterpret".into(),
            title: Some("Revised continuation".into()),
            parent_episode_revision_id: None,
            experience_time: second.revision.experience_time.clone(),
            boundary_explanation: second.revision.boundary_explanation.clone(),
            producer_signature_id: None,
            members: second
                .members
                .iter()
                .map(|member| EpisodeMemberInput {
                    reference: member.reference.clone(),
                    role: member.role.clone(),
                })
                .collect(),
            basis: second.basis.clone(),
        })
        .await
        .unwrap();
    let scope = journal.object.journal_id.0.to_string();
    let waiting = memory
        .plan_journal_review(subject, "journal_revalidate", &scope)
        .await
        .unwrap();
    assert_eq!(waiting.episodes.len(), 1);
    assert_eq!(
        waiting.episodes[0].object.episode_id,
        first.object.episode_id
    );
    assert_eq!(waiting.status, "deferred");
    assert_eq!(waiting.problem.as_deref(), Some("journal_sources_settling"));
    clock.advance_by(subject, Duration::seconds(300)).unwrap();
    let ready = memory
        .plan_journal_review(subject, "journal_revalidate", &scope)
        .await
        .unwrap();
    assert_eq!(ready.status, "ready");
    assert_eq!(ready.episodes.len(), 2);
    assert!(ready.episodes.iter().any(
        |episode| episode.revision.episode_revision_id == revised.revision.episode_revision_id
    ));
    assert_eq!(ready.occurrences, vec![occurrence]);
    let consolidation = memory
        .plan_consolidation_scope(
            subject,
            "journal_revision",
            &journal.revision.journal_revision_id.0.to_string(),
        )
        .await
        .unwrap();
    assert_eq!(consolidation.status, "obsolete");
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one regression spans capture, planning and atomic repair of historical chronology"
)]
async fn late_experience_uses_chronology_and_repairs_old_local_partitions() {
    use nous_core::CognitiveRef;
    use nous_protocol::{
        kernel as k, kernel::kernel_maintenance_service_server::KernelMaintenanceService,
    };
    let (root, url, _postgres) = database().await;
    let historical = chrono::DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let clock = Arc::new(ManualCognitiveClock::new(historical + Duration::days(730)));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    assert_eq!(
        rt.subjects.subject(subject).await.unwrap().created_at,
        clock.now(subject)
    );
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let mut occurrences = Vec::new();
    for (label, offset) in [("A", 0), ("C", 20)] {
        let mut input = observation(subject, Some(session.session_id));
        input.occurrence.observed_at = Some(historical + Duration::minutes(offset));
        input.material = ObservationMaterial::InlineText {
            text: label.into(),
            media_type: "text/plain".into(),
        };
        occurrences.push(
            rt.material
                .record_observation(input)
                .await
                .unwrap()
                .occurrence
                .occurrence_id,
        );
    }
    let original = rt
        .organize_experience(subject, 256, true)
        .await
        .unwrap()
        .episodes;
    assert_eq!(original.len(), 1);
    let mut input = observation(subject, Some(session.session_id));
    input.occurrence.observed_at = Some(historical + Duration::minutes(10));
    input.material = ObservationMaterial::InlineText {
        text: "B".into(),
        media_type: "text/plain".into(),
    };
    let late = rt
        .material
        .record_observation(input)
        .await
        .unwrap()
        .occurrence
        .occurrence_id;
    assert!(
        rt.organize_experience(subject, 256, true)
            .await
            .unwrap()
            .episodes
            .is_empty()
    );
    let service = nous_kernel::transport::KernelService(rt.clone());
    let kinds = vec!["episode_resegment".into()];
    let needs = rt
        .cognition
        .lease_maintenance(subject, &kinds, 8, 60, None)
        .await
        .unwrap();
    let need = needs
        .iter()
        .find(|need| need.scope_kind == "track")
        .unwrap();
    let plan = service
        .plan_maintenance(tonic::Request::new(k::PlanMaintenanceRequest {
            claimed: Some(k::MaintenanceNeed {
                need_id: need.need_id.to_string(),
                subject_id: subject.0.to_string(),
                kind: need.kind.clone(),
                scope_kind: need.scope_kind.clone(),
                scope_ref: need.scope_ref.clone(),
                trigger_authority_seq: need.trigger_authority_seq,
                due_at: Some(test_timestamp(need.due_at)),
                lease_token: need.lease_token.map(|id| id.to_string()),
                created_at: Some(test_timestamp(need.created_at)),
                updated_at: Some(test_timestamp(need.updated_at)),
                ..Default::default()
            }),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(plan.status, "ready");
    let chronology = vec![occurrences[0], late, occurrences[1]];
    assert_eq!(
        plan.members
            .iter()
            .map(|member| member.occurrence_id.clone())
            .collect::<Vec<_>>(),
        chronology
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        plan.members
            .iter()
            .map(|member| member.text.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "B", "C"]
    );
    assert!(plan.members[1].recorded_seq > plan.members[2].recorded_seq);
    let partition = partition_request(&rt, subject, &original, &chronology, &[vec![0, 1, 2]]).await;
    let committed = rt
        .require_memory()
        .unwrap()
        .apply_episode_partition(partition)
        .await
        .unwrap();
    assert_eq!(
        committed[0]
            .members
            .iter()
            .map(|member| member.reference.clone())
            .collect::<Vec<_>>(),
        chronology
            .into_iter()
            .map(CognitiveRef::Occurrence)
            .collect::<Vec<_>>()
    );
    rt.cognition
        .acknowledge_maintenance(need, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
}

fn test_timestamp(at: chrono::DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: at.timestamp(),
        nanos: at.timestamp_subsec_nanos() as i32,
    }
}

#[tokio::test]
async fn drafts_commit_replay_and_ack_reclaim_runtime_rows() {
    use nous_core::{CognitiveRef, OperationId};
    use nous_memory::{EpisodeInput, EpisodeMemberInput};
    let (root, url, _postgres) = database().await;
    let at = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(at));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let mut occurrences = Vec::new();
    for offset in [-20, 0, -10] {
        let mut input = observation(subject, Some(session.session_id));
        input.occurrence.observed_at = Some(at + Duration::minutes(offset));
        occurrences.push(
            rt.material
                .record_observation(input)
                .await
                .unwrap()
                .occurrence
                .occurrence_id,
        );
    }
    let progress = rt
        .cognition
        .segment_experience(subject, "interaction", 256, true)
        .await
        .unwrap();
    let draft = &progress.ready[0];
    assert_eq!(
        draft.members,
        vec![occurrences[0], occurrences[2], occurrences[1]]
    );
    let input = EpisodeInput {
        operation_id: OperationId(uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_OID,
            format!("episode-draft:{}", draft.draft_id).as_bytes(),
        )),
        subject,
        track_key: draft.track_key.clone(),
        title: None,
        parent_episode_revision_id: None,
        experience_time: TemporalExtent::Interval {
            start: Some(draft.observed_start),
            end: Some(draft.observed_end),
        },
        boundary_explanation: draft.boundary_reason.clone().unwrap(),
        producer_signature_id: None,
        members: draft
            .members
            .iter()
            .map(|id| EpisodeMemberInput {
                reference: CognitiveRef::Occurrence(*id),
                role: "experience".into(),
            })
            .collect(),
        basis: draft
            .members
            .iter()
            .map(|id| {
                nous_core::RevisionBasis::Evidence(nous_core::EvidenceRef {
                    epistemic_relation: None,
                    occurrence_id: *id,
                    locator: nous_core::EvidenceLocator::WholeOccurrence,
                    basis_role: nous_core::BasisRole::Direct,
                })
            })
            .collect(),
    };
    let committed = rt
        .require_memory()
        .unwrap()
        .create_episode(input)
        .await
        .unwrap();
    // Reopen after the owner commit, before Runtime acknowledgement.
    drop(rt);
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    clock.advance_by(subject, Duration::days(1)).unwrap();
    let resumed = rt.organize_experience(subject, 256, true).await.unwrap();
    assert_eq!(
        resumed.episodes[0].revision.episode_revision_id,
        committed.revision.episode_revision_id
    );
    assert_eq!(resumed.episodes[0].revision.formed_at, at);
    rt.cognition
        .acknowledge_episode_draft(
            subject,
            draft.draft_id,
            committed.revision.episode_revision_id,
        )
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM episode_drafts)+(SELECT count(*) FROM episode_draft_members)",
    )
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(count, 0);
    let dangling: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM segmentation_cursors WHERE open_draft_id IS NOT NULL)",
    )
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert!(!dangling);
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one regression follows runtime state across acknowledgement, replay and expiration"
)]
async fn maintenance_terminal_retention_workflow_cleanup_and_execution_backoff() {
    let (root, url, _postgres) = database().await;
    let at = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(at));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let request = MaintenanceRequest {
        subject,
        kind: "journal_review".into(),
        scope_kind: "track".into(),
        scope_ref: "interaction".into(),
        trigger_authority_seq: 0,
        due_at: at,
        priority: 30,
    };
    let need_id = rt
        .cognition
        .enqueue_maintenance(request.clone())
        .await
        .unwrap();
    let kinds = vec![request.kind.clone()];
    let claimed = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    let workflow = rt.store.reserve_model_workflow(subject, "memory", "maintenance-test", "input", &serde_json::json!({"maintenance_claim":{"need_id":need_id.to_string(),"lease_token":claimed.lease_token.unwrap().to_string(),"trigger":claimed.trigger_authority_seq,"trigger_revision":claimed.trigger_revision}}), 360).await.unwrap();
    let explicit = rt
        .store
        .reserve_model_workflow(
            subject,
            "memory",
            "explicit-test",
            "input",
            &serde_json::json!({}),
            360,
        )
        .await
        .unwrap();
    for (key, token) in [
        ("maintenance-test", workflow.lease_token.unwrap()),
        ("explicit-test", explicit.lease_token.unwrap()),
    ] {
        rt.store
            .save_model_workflow(
                subject,
                "memory",
                key,
                token,
                None,
                Some(&serde_json::json!({"status":"no_change"})),
                None,
            )
            .await
            .unwrap();
    }
    rt.cognition
        .acknowledge_maintenance(&claimed, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    clock.advance_by(subject, Duration::days(100)).unwrap();
    rt.cognition
        .acknowledge_maintenance(&claimed, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT operation_key FROM model_workflow_operations WHERE subject_id=$1",
    )
    .bind(subject.0)
    .fetch_all(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(keys, vec!["explicit-test"]);
    // Execution time, not accelerated cognition time, defines the replay window.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM maintenance_needs WHERE need_id=$1")
        .bind(need_id)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    sqlx::query("UPDATE maintenance_needs SET terminal_at=clock_timestamp()-interval '2 days' WHERE need_id=$1").bind(need_id).execute(rt.store.pool()).await.unwrap();
    let pending = rt
        .cognition
        .enqueue_maintenance(MaintenanceRequest {
            scope_ref: "pending".into(),
            ..request
        })
        .await
        .unwrap();
    let claimed = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(claimed.need_id, pending);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM maintenance_needs WHERE need_id=$1")
        .bind(need_id)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    rt.cognition
        .acknowledge_maintenance(
            &claimed,
            MaintenanceDisposition::Retry {
                delay_seconds: 30,
                problem_code: "provider_unavailable".into(),
            },
        )
        .await
        .unwrap();
    clock.advance_by(subject, Duration::days(100)).unwrap();
    rt.cognition
        .acknowledge_maintenance(
            &claimed,
            MaintenanceDisposition::Retry {
                delay_seconds: 30,
                problem_code: "provider_unavailable".into(),
            },
        )
        .await
        .unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, None)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query("UPDATE maintenance_needs SET retry_not_before=clock_timestamp()-interval '1 second' WHERE need_id=$1").bind(pending).execute(rt.store.pool()).await.unwrap();
    let retry = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(retry.retry_count, 1);
    assert_eq!(retry.attempt_count, 2);
    let abandoned = rt.store.reserve_model_workflow(subject, "memory", "superseded-test", "input", &serde_json::json!({"maintenance_claim":{"need_id":pending.to_string(),"lease_token":retry.lease_token.unwrap().to_string(),"trigger":retry.trigger_authority_seq,"trigger_revision":retry.trigger_revision}}), 360).await.unwrap();
    rt.store
        .release_model_workflow(
            subject,
            "memory",
            "superseded-test",
            abandoned.lease_token.unwrap(),
        )
        .await
        .unwrap();
    rt.cognition
        .enqueue_maintenance(MaintenanceRequest {
            subject,
            kind: "journal_review".into(),
            scope_kind: "track".into(),
            scope_ref: "pending".into(),
            trigger_authority_seq: 1,
            due_at: clock.now(subject),
            priority: 30,
        })
        .await
        .unwrap();
    rt.cognition
        .acknowledge_maintenance(&retry, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    let survives: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM model_workflow_operations WHERE operation_key='superseded-test')").fetch_one(rt.store.pool()).await.unwrap();
    assert!(!survives);
    assert_eq!(
        rt.cognition.maintenance_needs(subject).await.unwrap()[0].state,
        "pending"
    );
    let original_digest = "a".repeat(64);
    let replacement_digest = "b".repeat(64);
    let blocked = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, Some(&original_digest))
        .await
        .unwrap()
        .remove(0);
    rt.cognition
        .acknowledge_maintenance(
            &blocked,
            MaintenanceDisposition::Blocked {
                problem_code: "maintenance_retry_exhausted".into(),
            },
        )
        .await
        .unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, Some(&original_digest))
            .await
            .unwrap()
            .is_empty()
    );
    let reactivated = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, Some(&replacement_digest))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        reactivated.trigger_authority_seq,
        blocked.trigger_authority_seq
    );
    assert!(reactivated.trigger_revision > blocked.trigger_revision);
    assert_eq!(reactivated.retry_count, 0);
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one regression covers blocked source scopes and their event-driven recovery"
)]
async fn oversized_repair_and_journal_scopes_block_until_real_triggers() {
    use nous_core::{BasisRole, CognitionDependency, CognitiveRef, OperationId, RevisionBasis};
    use nous_memory::{JournalInput, JournalPoint, JournalPointRole};
    let (root, url, _postgres) = database().await;
    let at = Utc::now().trunc_subsecs(6);
    let clock = Arc::new(ManualCognitiveClock::new(at));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let mut episodes = Vec::new();
    for offset in [-20, 0] {
        let mut input = observation(subject, Some(session.session_id));
        input.occurrence.observed_at = Some(at + Duration::minutes(offset));
        rt.material.record_observation(input).await.unwrap();
        episodes.extend(
            rt.organize_experience(subject, 256, true)
                .await
                .unwrap()
                .episodes,
        );
    }
    let memory = rt.require_memory().unwrap();
    let journal = memory
        .commit_journal(JournalInput {
            operation_id: OperationId::new(),
            subject,
            expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
            target: None,
            sources: episodes
                .iter()
                .map(|episode| EpisodePartitionSource {
                    revision: episode.revision.episode_revision_id,
                    expected_epoch: episode.object.object_epoch,
                })
                .collect(),
            title: None,
            narrative: "Complete source scope.".into(),
            producer: None,
            points: vec![JournalPoint {
                role: JournalPointRole::Summary,
                text: "Both experiences.".into(),
                basis: episodes
                    .iter()
                    .map(|episode| {
                        RevisionBasis::CognitionDependency(CognitionDependency {
                            epistemic_relation: None,
                            target_revision: CognitiveRef::EpisodeRevision(
                                episode.revision.episode_revision_id,
                            ),
                            basis_role: BasisRole::Direct,
                        })
                    })
                    .collect(),
            }],
        })
        .await
        .unwrap();
    let episode = &episodes[1];
    memory
        .revise_episode(nous_memory::ReviseEpisodeInput {
            operation_id: OperationId::new(),
            subject,
            episode_id: episode.object.episode_id,
            expected_object_epoch: episode.object.object_epoch,
            intent: "reinterpret".into(),
            title: Some("Updated source".into()),
            parent_episode_revision_id: None,
            experience_time: episode.revision.experience_time.clone(),
            boundary_explanation: episode.revision.boundary_explanation.clone(),
            producer_signature_id: None,
            members: episode
                .members
                .iter()
                .map(|member| nous_memory::EpisodeMemberInput {
                    reference: member.reference.clone(),
                    role: member.role.clone(),
                })
                .collect(),
            basis: episode.basis.clone(),
        })
        .await
        .unwrap();
    clock.advance_by(subject, Duration::seconds(300)).unwrap();
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            "journal.max_episode_count",
            serde_json::json!(1),
        )
        .await
        .unwrap();
    let scope = journal.object.journal_id.0.to_string();
    let plan = memory
        .plan_journal_review(subject, "journal_revalidate", &scope)
        .await
        .unwrap();
    assert_eq!(plan.status, "blocked");
    assert_eq!(
        plan.problem.as_deref(),
        Some("journal_scope_bound_exceeded")
    );
    assert!(plan.next_due.is_none());
    let kinds = vec!["journal_revalidate".into()];
    let claim = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    rt.cognition
        .acknowledge_maintenance(
            &claim,
            MaintenanceDisposition::Blocked {
                problem_code: plan.problem.unwrap(),
            },
        )
        .await
        .unwrap();
    clock.advance_by(subject, Duration::days(30)).unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        memory
            .journal(subject, journal.object.journal_id, None)
            .await
            .unwrap()
            .object
            .integrity_state,
        nous_core::IntegrityState::RevalidationRequired
    );
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            "journal.max_episode_count",
            serde_json::json!(2),
        )
        .await
        .unwrap();
    let claim = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        memory
            .plan_journal_review(subject, "journal_revalidate", &scope)
            .await
            .unwrap()
            .episodes
            .len(),
        2
    );
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            "journal.max_span_seconds",
            serde_json::json!(60),
        )
        .await
        .unwrap();
    let plan = memory
        .plan_journal_review(subject, "journal_revalidate", &scope)
        .await
        .unwrap();
    assert_eq!(plan.status, "blocked");
    assert_eq!(plan.problem.as_deref(), Some("journal_scope_span_exceeded"));
    rt.cognition
        .acknowledge_maintenance(
            &claim,
            MaintenanceDisposition::Blocked {
                problem_code: plan.problem.unwrap(),
            },
        )
        .await
        .unwrap();
    clock.advance_by(subject, Duration::days(30)).unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, None)
            .await
            .unwrap()
            .is_empty()
    );
    // A relevant source lifecycle event wakes the blocked revalidation obligation.
    memory
        .suppress_episode(
            subject,
            episodes[0].object.episode_id,
            OperationId::new(),
            episodes[0].object.object_epoch,
        )
        .await
        .unwrap();
    let claim = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    let ready = memory
        .plan_journal_review(subject, "journal_revalidate", &scope)
        .await
        .unwrap();
    assert_eq!(ready.status, "ready");
    assert_eq!(ready.episodes.len(), 1);
    rt.cognition
        .acknowledge_maintenance(&claim, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    // Historical repair scope bounds apply to experience extent, not age.
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            "episode.max_neighbor_span_seconds",
            serde_json::json!(60),
        )
        .await
        .unwrap();
    let mut input = observation(subject, Some(session.session_id));
    input.occurrence.observed_at = Some(at - Duration::minutes(10));
    rt.material.record_observation(input).await.unwrap();
    rt.organize_experience(subject, 256, false).await.unwrap();
    let kinds = vec!["episode_resegment".into()];
    let claims = rt
        .cognition
        .lease_maintenance(subject, &kinds, 8, 60, None)
        .await
        .unwrap();
    let claim = claims
        .iter()
        .find(|need| need.scope_kind == "track")
        .unwrap();
    let plan = memory
        .plan_episode_review(subject, "track", "interaction", claim.trigger_authority_seq)
        .await
        .unwrap();
    assert_eq!(plan.status, "blocked");
    assert_eq!(
        plan.problem.as_deref(),
        Some("historical_repair_scope_exceeded")
    );
    rt.cognition
        .acknowledge_maintenance(
            claim,
            MaintenanceDisposition::Blocked {
                problem_code: plan.problem.unwrap(),
            },
        )
        .await
        .unwrap();
    for other in claims.iter().filter(|need| need.need_id != claim.need_id) {
        rt.cognition
            .acknowledge_maintenance(other, MaintenanceDisposition::Satisfied)
            .await
            .unwrap();
    }
    clock.advance_by(subject, Duration::days(30)).unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60, None)
            .await
            .unwrap()
            .is_empty()
    );
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            "episode.max_neighbor_span_seconds",
            serde_json::json!(86400),
        )
        .await
        .unwrap();
    let resumed = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60, None)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        memory
            .plan_episode_review(
                subject,
                "track",
                "interaction",
                resumed.trigger_authority_seq
            )
            .await
            .unwrap()
            .status,
        "ready"
    );
}

async fn assert_schema_clock(
    rt: &NousRuntime,
    clock: &ManualCognitiveClock,
    subject: nous_core::SubjectId,
    occurrence: nous_core::OccurrenceId,
) {
    let schema_input = nous_memory::CreateSchemaInput {
        producer: None,
        operation_id: nous_core::OperationId::new(),
        subject,
        title: None,
        structural_claim: "Owner-assigned cognitive formation time".into(),
        applicability_scope: nous_memory::SchemaScope {
            description: "Timestamp owner contract".into(),
            aboutness: vec![],
            tags: vec![],
            valid_time: TemporalExtent::Unknown,
        },
        boundary_definition: "Evidence-backed imported schema".into(),
        formation_kind: nous_memory::SchemaFormationKind::ExplicitImport,
        evidence_links: vec![nous_memory::SchemaEvidenceLinkInput {
            role: nous_memory::SchemaEvidenceRole::Support,
            basis: nous_core::RevisionBasis::Evidence(nous_core::EvidenceRef {
                epistemic_relation: None,
                occurrence_id: occurrence,
                locator: nous_core::EvidenceLocator::WholeOccurrence,
                basis_role: nous_core::BasisRole::Direct,
            }),
        }],
    };
    let schema = rt
        .require_memory()
        .unwrap()
        .create_schema(schema_input.clone())
        .await
        .unwrap();
    assert_eq!(schema.revision.formed_at, clock.now(subject));
    assert_eq!(schema.revision.recorded_at, clock.now(subject));
    let at = clock.now(subject);
    clock.advance_by(subject, Duration::days(1)).unwrap();
    let replay_schema = rt
        .require_memory()
        .unwrap()
        .create_schema(schema_input.clone())
        .await
        .unwrap();
    assert_eq!(replay_schema.revision.formed_at, at);
    let revised_schema = rt
        .require_memory()
        .unwrap()
        .revise_schema(nous_memory::ReviseSchemaInput {
            formation_kind: schema.revision.formation_kind,
            producer: None,
            evidence_links: Vec::new(),
            operation_id: nous_core::OperationId::new(),
            subject,
            schema_id: schema.schema.schema_id,
            expected_object_epoch: schema.schema.object_epoch,
            intent: nous_memory::RevisionIntent::Correct,
            title: Some("Revised at cognition time".into()),
            structural_claim: schema_input.structural_claim,
            applicability_scope: schema_input.applicability_scope,
            boundary_definition: schema_input.boundary_definition,
            copy_link_ids: schema
                .evidence_links
                .iter()
                .map(|link| link.link_id)
                .collect(),
        })
        .await
        .unwrap();
    assert_eq!(revised_schema.revision.formed_at, clock.now(subject));
    assert_eq!(revised_schema.revision.recorded_at, clock.now(subject));
}

async fn assert_subject_pagination(rt: &NousRuntime) {
    use nous_protocol::public as p;
    let mut subjects = vec![];
    for _ in 0..51 {
        subjects.push(create_subject(rt).await);
    }
    let service = nous_kernel::transport::KernelService(rt.clone());
    let first = service
        .list_subjects(tonic::Request::new(p::ListRequest {
            status: "active".into(),
            ..Default::default()
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(first.items.len(), 50);
    assert!(!first.next_page_token.is_empty());
    let second = service
        .list_subjects(tonic::Request::new(p::ListRequest {
            status: "active".into(),
            page: Some(p::Page {
                page_token: first.next_page_token,
                page_size: 0,
            }),
            ..Default::default()
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(second.items.len(), 2);
    assert!(second.next_page_token.is_empty());
    let seen: std::collections::BTreeSet<_> = first
        .items
        .into_iter()
        .chain(second.items)
        .map(|subject| subject.subject_id)
        .collect();
    assert_eq!(seen.len(), 52);
    for subject in subjects {
        assert!(seen.contains(&subject.0.to_string()));
    }
}

#[tokio::test]
async fn historical_schema_episode_journal_use_past_heads_and_history_documents() {
    use nous_core::{BasisRole, CognitionDependency, CognitiveRef, OperationId, RevisionBasis};
    let (root, url, _pg) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock_serving(&url, &root, clock.clone(), true).await;
    let subject = create_subject(&rt).await;
    let episode = journal_source_episode(&rt, &clock, subject).await;
    let episode = historical_episode_text(&rt, subject, &episode, "archival past episode").await;
    let owner = rt.require_memory().unwrap();
    let mut memory_input = consolidation_memory(&episode);
    memory_input.representation_text = "archival past memory".into();
    let memory = owner.form_memory(memory_input).await.unwrap();
    let mut schema_input = consolidation_schema(&episode);
    schema_input.structural_claim = "archival past schema".into();
    let schema = owner.create_schema(schema_input.clone()).await.unwrap();
    let mut journal_input = nous_memory::JournalInput {
        operation_id: OperationId::new(),
        subject,
        expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
        target: None,
        sources: vec![EpisodePartitionSource {
            revision: episode.revision.episode_revision_id,
            expected_epoch: episode.object.object_epoch,
        }],
        title: None,
        narrative: "archival past journal".into(),
        points: vec![nous_memory::JournalPoint {
            role: nous_memory::JournalPointRole::Summary,
            text: "archival past summary".into(),
            basis: vec![RevisionBasis::CognitionDependency(CognitionDependency {
                epistemic_relation: None,
                target_revision: CognitiveRef::EpisodeRevision(
                    episode.revision.episode_revision_id,
                ),
                basis_role: BasisRole::Direct,
            })],
        }],
        producer: None,
    };
    let journal = owner.commit_journal(journal_input.clone()).await.unwrap();
    let cut = rt.cognition.now(subject);
    let old = vec![
        CognitiveRef::MemoryRevision(memory.revision.memory_revision_id),
        CognitiveRef::CognitiveSchemaRevision(schema.revision.schema_revision_id),
        CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
        CognitiveRef::JournalRevision(journal.revision.journal_revision_id),
    ];
    clock.advance_by(subject, Duration::seconds(10)).unwrap();
    let new_schema = owner
        .revise_schema(nous_memory::ReviseSchemaInput {
            operation_id: OperationId::new(),
            subject,
            schema_id: schema.schema.schema_id,
            expected_object_epoch: schema.schema.object_epoch,
            intent: nous_memory::RevisionIntent::Rephrase,
            title: None,
            structural_claim: "archival future schema".into(),
            applicability_scope: schema.revision.applicability_scope.clone(),
            boundary_definition: schema.revision.boundary_definition.clone(),
            formation_kind: schema.revision.formation_kind,
            producer: Some(consolidation_producer()),
            evidence_links: schema_input.evidence_links,
            copy_link_ids: vec![],
        })
        .await
        .unwrap();
    let new_episode =
        historical_episode_text(&rt, subject, &episode, "archival future episode").await;
    let current_journal = owner
        .journal(subject, journal.object.journal_id, None)
        .await
        .unwrap();
    journal_input.operation_id = OperationId::new();
    journal_input.expected_authority_seq = rt.store.authority_seq(subject).await.unwrap();
    journal_input.target = Some(nous_memory::JournalTarget {
        journal_id: journal.object.journal_id,
        expected_revision: journal.revision.journal_revision_id,
        expected_epoch: current_journal.object.object_epoch,
        intent: "revalidate".into(),
    });
    journal_input.sources = vec![EpisodePartitionSource {
        revision: new_episode.revision.episode_revision_id,
        expected_epoch: new_episode.object.object_epoch,
    }];
    journal_input.narrative = "archival future journal".into();
    journal_input.points[0].basis = vec![RevisionBasis::CognitionDependency(CognitionDependency {
        epistemic_relation: None,
        target_revision: CognitiveRef::EpisodeRevision(new_episode.revision.episode_revision_id),
        basis_role: BasisRole::Direct,
    })];
    let new_journal = owner.commit_journal(journal_input).await.unwrap();
    let new = vec![
        CognitiveRef::CognitiveSchemaRevision(new_schema.revision.schema_revision_id),
        CognitiveRef::EpisodeRevision(new_episode.revision.episode_revision_id),
        CognitiveRef::JournalRevision(new_journal.revision.journal_revision_id),
    ];
    assert_historical_domains(&rt, subject, cut, &old, &new).await;
}

async fn assert_historical_domains(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    cut: chrono::DateTime<Utc>,
    old: &[nous_core::CognitiveRef],
    new: &[nous_core::CognitiveRef],
) {
    use nous_core::*;
    let mut query = media_episode_query(subject, "archival");
    query.projection = ResultProjection::default();
    query.result_need.limit = 16;
    query.temporal_frame.authority_view = AuthorityView::AsOf(cut);
    let view = rt
        .historical_authority_view(subject, cut, RevisionView::Current)
        .await
        .unwrap();
    for reference in old {
        let state = view.cognition_for(reference).unwrap();
        let mut exact = query.clone();
        exact.expression.cues = vec![Cue::Text(nous_core::TextCue {
            text: "read archival cognition".into(),
        })];
        exact.expression.targets = vec![QueryTarget::Exact {
            reference: state.object.clone(),
        }];
        let result = rt.execute_query(exact, None).await.unwrap();
        assert!(
            result
                .result
                .results
                .iter()
                .any(|hit| &hit.reference == reference),
            "missing historical owner hit {reference}: {:?}",
            result.result
        );
        assert!(
            result
                .result
                .results
                .iter()
                .any(|hit| hit.authority_epoch == state.state["object_epoch"].as_i64())
        );
    }
    query.temporal_frame.revision_view = RevisionView::History;
    let past = rt.execute_query(query.clone(), None).await.unwrap();
    assert!(past.result.generation.lexical.is_some());
    assert!(
        past.result
            .results
            .iter()
            .all(|hit| !new.contains(&hit.reference))
    );
    query.temporal_frame.authority_view = AuthorityView::Current;
    let history = rt.execute_query(query.clone(), None).await.unwrap();
    for reference in old.iter().chain(new) {
        assert!(
            history
                .result
                .results
                .iter()
                .any(|hit| &hit.reference == reference),
            "history missed {reference}: {:?}",
            history.result
        );
    }
    query.temporal_frame.revision_view = RevisionView::Current;
    let current = rt.execute_query(query.clone(), None).await.unwrap();
    for reference in new {
        assert!(
            current
                .result
                .results
                .iter()
                .any(|hit| &hit.reference == reference)
        );
    }
    for reference in &old[1..] {
        assert!(
            !current
                .result
                .results
                .iter()
                .any(|hit| &hit.reference == reference)
        );
    }
    query.temporal_frame.revision_view = RevisionView::History;
    query.expression.constraints.recorded = Some(TimeInterval {
        start: None,
        end: Some(cut + Duration::seconds(1)),
    });
    let filtered = rt.execute_query(query, None).await.unwrap();
    assert!(
        filtered
            .result
            .results
            .iter()
            .all(|hit| !new.contains(&hit.reference))
    );
    assert!(!filtered.result.results.is_empty());
}

async fn historical_episode_text(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    episode: &EpisodeView,
    text: &str,
) -> EpisodeView {
    rt.require_memory()
        .unwrap()
        .revise_episode(nous_memory::ReviseEpisodeInput {
            operation_id: nous_core::OperationId::new(),
            subject,
            episode_id: episode.object.episode_id,
            expected_object_epoch: episode.object.object_epoch,
            intent: "reinterpret".into(),
            title: None,
            parent_episode_revision_id: None,
            experience_time: episode.revision.experience_time.clone(),
            boundary_explanation: text.into(),
            producer_signature_id: None,
            members: episode
                .members
                .iter()
                .map(|member| nous_memory::EpisodeMemberInput {
                    reference: member.reference.clone(),
                    role: member.role.clone(),
                })
                .collect(),
            basis: episode.basis.clone(),
        })
        .await
        .unwrap()
}

async fn assert_historical_synopsis(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    cut: chrono::DateTime<Utc>,
    old: nous_core::DerivedRepresentationId,
    new: nous_core::DerivedRepresentationId,
) {
    use nous_core::*;
    let view = rt
        .historical_authority_view(subject, cut, RevisionView::Current)
        .await
        .unwrap();
    assert!(
        view.material_documents
            .contains(&CognitiveRef::DerivedRepresentation(old))
    );
    assert!(
        !view
            .material_visibility
            .contains(&CognitiveRef::DerivedRepresentation(new))
    );
    let current = rt
        .material
        .project_as_of(subject, rt.cognition.now(subject))
        .await
        .unwrap();
    assert!(
        !current
            .document_references
            .contains(&CognitiveRef::DerivedRepresentation(old))
    );
    assert!(
        current
            .document_references
            .contains(&CognitiveRef::DerivedRepresentation(new))
    );
    let mut query = media_episode_query(subject, "cobalt");
    query.temporal_frame.authority_view = AuthorityView::AsOf(cut);
    let result = rt.query(query).await.unwrap();
    assert!(result.results.iter().any(|hit| {
        hit.representation
            .as_ref()
            .is_some_and(|text| text.contains("cobalt harbor"))
    }));
    assert!(!result.results.iter().any(|hit| {
        hit.representation
            .as_ref()
            .is_some_and(|text| text.contains("amber inlet"))
    }));
}
