mod test_support;

use chrono::{Duration, SubsecRound, Utc};
use nous_core::{SourceClass, TemporalExtent};
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
use test_support::database;

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
    NousRuntime::open_with_clock(RuntimeOptions {
        postgres_url: url.into(), max_connections: 8,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        serving_options: ServingOptions { root: root.path().join("serving"), lexical: false, dense: false, topology: false, memory_enabled: true },
        embedding: None, stored_embedding: None,
        deployment_settings: serde_json::json!({"settings":{"serving":{"lexical":{"enabled":false},"dense":{"enabled":false},"topology":{"enabled":false}}}}),
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
            .lease_maintenance(subject, &kinds, 1, 60)
            .await
            .unwrap()
            .is_empty()
    );
    clock.advance_by(subject, Duration::hours(1)).unwrap();
    let (a, b) = tokio::join!(
        rt.cognition.lease_maintenance(subject, &kinds, 1, 60),
        rt.cognition.lease_maintenance(subject, &kinds, 1, 60)
    );
    let leased: Vec<_> = a.unwrap().into_iter().chain(b.unwrap()).collect();
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].need_id, id);
    clock.advance_by(subject, Duration::days(30)).unwrap();
    assert!(
        rt.cognition
            .lease_maintenance(subject, &kinds, 1, 60)
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
        .lease_maintenance(subject, &kinds, 1, 60)
        .await
        .unwrap();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].trigger_authority_seq, 2);
    sqlx::query("UPDATE maintenance_needs SET lease_until=clock_timestamp()-interval '1 second' WHERE need_id=$1")
        .bind(id).execute(rt.store.pool()).await.unwrap();
    let retry = rt
        .cognition
        .lease_maintenance(subject, &kinds, 1, 60)
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
            supports: input.supports.clone(),
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
        CognitionDependency, CognitiveRef, IntegrityState, OperationId, RevisionSupport,
        SupportRole,
    };
    use nous_memory::{JournalInput, JournalPoint, JournalPointRole};
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let episode = journal_source_episode(&rt, &clock, subject).await;
    assert_maintenance_planning(&rt, &clock, subject).await;
    let memory = rt.require_memory().unwrap();
    let support = RevisionSupport::CognitionDependency(CognitionDependency {
        target_revision: CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
        support_role: SupportRole::Direct,
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
            text: "Two sources contributed.".into(),
            supports: vec![support],
        }],
        producer: None,
    };
    let journal = memory.commit_journal(input.clone()).await.unwrap();
    let support = RevisionSupport::CognitionDependency(CognitionDependency {
        target_revision: CognitiveRef::JournalRevision(journal.revision.journal_revision_id),
        support_role: SupportRole::Direct,
    });
    let provenance = memory
        .provenance_summary(subject, std::slice::from_ref(&support))
        .await
        .unwrap();
    assert_eq!(provenance.roots.len(), 2);
    assert_eq!(provenance.normalized_inputs.len(), 1);
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
    let mut invalid = input.clone();
    invalid.operation_id = OperationId::new();
    invalid.expected_authority_seq = rt.store.authority_seq(subject).await.unwrap();
    invalid.points[0].supports = vec![support];
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
    memory
        .purge_journal(
            subject,
            journal.object.journal_id,
            OperationId::new(),
            revised.object.object_epoch,
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
            text: source.into(),
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
            references: references.clone(),
        })
        .await
        .unwrap();
    assert_eq!(context.references, references);
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
    use nous_protocol::{kernel::authority_service_server::AuthorityService, public as p};
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
            .supports
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
    use nous_protocol::{kernel as k, kernel::authority_service_server::AuthorityService};
    let service = nous_kernel::transport::KernelService(rt.clone());
    clock.advance_by(subject, Duration::seconds(300)).unwrap();
    for kind in ["episode_resegment", "journal_review"] {
        let claim = service
            .claim_maintenance(tonic::Request::new(k::ClaimMaintenanceRequest {
                subject_id: subject.0.to_string(),
                allowed_kinds: vec![kind.into()],
                limit: 1,
                lease_seconds: 60,
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
        assert_eq!(plan.members[0].text, "object:source-a");
        assert!(plan.supports.len() >= 3);
        let finish = k::FinishMaintenanceRequest {
            claimed: Some(claimed),
            disposition: "satisfied".into(),
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
            references: vec![],
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
        .lease_maintenance(subject, &["episode_segment".into()], 1, 60)
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
        .lease_maintenance(subject, &["episode_segment".into()], 1, 60)
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
fn consolidation_memory(episode: &EpisodeView) -> nous_memory::ConsolidationMemoryContent {
    let nous_core::CognitiveRef::Occurrence(occurrence) = episode.members[0].reference else {
        panic!("expected occurrence")
    };
    nous_memory::ConsolidationMemoryContent {
        cognitive_role: nous_memory::CognitiveRole::Declarative,
        formation_mode: nous_memory::FormationMode::Grounded,
        grounding_occurrence_id: Some(occurrence),
        semantic_role: "statement".into(),
        representation_text: "A reusable observed fact.".into(),
        title: None,
        supports: vec![nous_core::RevisionSupport::Evidence(
            nous_core::EvidenceRef {
                occurrence_id: occurrence,
                locator: nous_core::EvidenceLocator::WholeOccurrence,
                support_role: nous_core::SupportRole::Direct,
            },
        )],
        aboutness: vec![],
        valid_time: TemporalExtent::Unknown,
        epistemic_class: nous_core::EpistemicClass::Derived,
    }
}
fn consolidation_schema(episode: &EpisodeView) -> nous_memory::ConsolidationSchemaContent {
    nous_memory::ConsolidationSchemaContent {
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
            .supports
            .iter()
            .cloned()
            .map(|support| nous_memory::SchemaEvidenceLinkInput {
                role: nous_memory::SchemaEvidenceRole::Support,
                support,
            })
            .collect(),
    }
}

#[tokio::test]
async fn longitudinal_consolidation_is_atomic_stale_fenced_and_replayable() {
    use nous_core::{CognitiveRef, OperationId};
    use nous_memory::{
        ConsolidationResultRef, ExpectedCognition, LongitudinalConsolidationAction as Action,
        LongitudinalConsolidationInput,
    };
    let (root, url, _postgres) = database().await;
    let clock = Arc::new(ManualCognitiveClock::new(Utc::now().trunc_subsecs(6)));
    let rt = runtime_with_clock(&url, &root, clock.clone()).await;
    let subject = create_subject(&rt).await;
    let episode = journal_source_episode(&rt, &clock, subject).await;
    let memory = rt.require_memory().unwrap();
    let input = LongitudinalConsolidationInput {
        operation_id: OperationId::new(),
        subject,
        expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
        source: ExpectedCognition {
            reference: CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
            expected_epoch: episode.object.object_epoch,
        },
        context: vec![],
        producer: consolidation_producer(),
        actions: vec![
            Action::CreateMemory {
                content: consolidation_memory(&episode),
            },
            Action::CreateMemory {
                content: consolidation_memory(&episode),
            },
            Action::CreateSchema {
                content: consolidation_schema(&episode),
            },
            Action::LinkRelation {
                from: ConsolidationResultRef::Action { index: 0 },
                to: ConsolidationResultRef::Action { index: 1 },
                relation: nous_memory::MemoryRelation::Elaborates,
            },
        ],
    };
    let committed = memory
        .commit_longitudinal_consolidation(input.clone())
        .await
        .unwrap();
    assert_eq!(committed.status, "committed");
    assert_eq!(committed.results.len(), 4);
    assert_eq!(committed.authority_seq, input.expected_authority_seq + 1);
    let Some(CognitiveRef::MemoryRevision(revision)) = committed.results[0] else {
        panic!("expected Memory result")
    };
    let object = rt
        .store
        .bind_exact_reference(subject, &CognitiveRef::MemoryRevision(revision))
        .await
        .unwrap();
    assert_eq!(object.1, Some(1));
    let Some(CognitiveRef::CognitiveSchemaRevision(schema_revision)) = committed.results[2] else {
        panic!("expected Schema result")
    };
    let schema_id: uuid::Uuid = sqlx::query_scalar(
        "SELECT schema_id FROM cognitive_schema_revisions WHERE schema_revision_id=$1",
    )
    .bind(schema_revision.0)
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    let schema = memory
        .schema(subject, nous_core::CognitiveSchemaId(schema_id))
        .await
        .unwrap();
    assert!(schema.revision.producer_signature_id.is_some());
    let producer: String = sqlx::query_scalar(
        "SELECT operation FROM producer_signatures WHERE producer_signature_id=$1",
    )
    .bind(schema.revision.producer_signature_id.unwrap())
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(producer, "memory.consolidation.text");
    clock.advance_by(subject, Duration::days(1)).unwrap();
    let replay = memory
        .commit_longitudinal_consolidation(input.clone())
        .await
        .unwrap();
    assert_eq!(replay.results, committed.results);
    assert_eq!(
        rt.store.authority_seq(subject).await.unwrap(),
        committed.authority_seq
    );
    assert_consolidation_rollback(&rt, &input, &episode).await;
    assert_consolidation_revisions(&rt, &input, &episode, &committed.results).await;
    memory
        .suppress_episode(
            subject,
            episode.object.episode_id,
            OperationId::new(),
            episode.object.object_epoch,
        )
        .await
        .unwrap();
    let mut stale = input.clone();
    stale.operation_id = OperationId::new();
    stale.expected_authority_seq = rt.store.authority_seq(subject).await.unwrap();
    assert!(
        memory
            .commit_longitudinal_consolidation(stale)
            .await
            .is_err()
    );
    assert_eq!(
        memory
            .commit_longitudinal_consolidation(input)
            .await
            .unwrap()
            .results,
        committed.results
    );
}

async fn assert_consolidation_rollback(
    rt: &NousRuntime,
    input: &nous_memory::LongitudinalConsolidationInput,
    episode: &EpisodeView,
) {
    use nous_memory::LongitudinalConsolidationAction as Action;
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_objects WHERE subject_id=$1")
        .bind(input.subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    let mut invalid = input.clone();
    invalid.operation_id = nous_core::OperationId::new();
    invalid.expected_authority_seq = rt.store.authority_seq(input.subject).await.unwrap();
    let mut bad_schema = consolidation_schema(episode);
    bad_schema.boundary_definition = String::new();
    invalid.actions = vec![
        Action::CreateMemory {
            content: consolidation_memory(episode),
        },
        Action::CreateSchema {
            content: bad_schema,
        },
    ];
    assert!(
        rt.require_memory()
            .unwrap()
            .commit_longitudinal_consolidation(invalid.clone())
            .await
            .is_err()
    );
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_objects WHERE subject_id=$1")
        .bind(input.subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        rt.store.authority_seq(input.subject).await.unwrap(),
        invalid.expected_authority_seq
    );
    let receipts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mutation_receipts WHERE subject_id=$1 AND operation_id=$2",
    )
    .bind(input.subject.0)
    .bind(invalid.operation_id.0)
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(receipts, 0);
}
async fn assert_consolidation_revisions(
    rt: &NousRuntime,
    input: &nous_memory::LongitudinalConsolidationInput,
    episode: &EpisodeView,
    results: &[Option<nous_core::CognitiveRef>],
) {
    use nous_memory::{
        ExpectedCognition, LongitudinalConsolidationAction as Action, RevisionIntent,
    };
    let mut revised = input.clone();
    revised.operation_id = nous_core::OperationId::new();
    revised.expected_authority_seq = rt.store.authority_seq(input.subject).await.unwrap();
    let memory = ExpectedCognition {
        reference: results[0].clone().unwrap(),
        expected_epoch: 1,
    };
    let schema = ExpectedCognition {
        reference: results[2].clone().unwrap(),
        expected_epoch: 1,
    };
    revised.context = vec![memory.clone(), schema.clone()];
    let mut content = consolidation_memory(episode);
    content.representation_text = "A corrected reusable fact.".into();
    revised.actions = vec![
        Action::ReviseMemory {
            target: memory,
            intent: RevisionIntent::Correct,
            content,
        },
        Action::ReviseSchema {
            target: schema,
            intent: RevisionIntent::Rephrase,
            content: consolidation_schema(episode),
        },
    ];
    let outcome = rt
        .require_memory()
        .unwrap()
        .commit_longitudinal_consolidation(revised.clone())
        .await
        .unwrap();
    assert_ne!(outcome.results[0], results[0]);
    assert_ne!(outcome.results[1], results[2]);
    for reference in outcome.results.iter().flatten() {
        assert_eq!(
            rt.store
                .bind_exact_reference(input.subject, reference)
                .await
                .unwrap()
                .1,
            Some(2)
        );
    }
    assert_eq!(
        rt.require_memory()
            .unwrap()
            .commit_longitudinal_consolidation(revised.clone())
            .await
            .unwrap()
            .results,
        outcome.results
    );
    revised.operation_id = nous_core::OperationId::new();
    revised.expected_authority_seq = outcome.authority_seq;
    assert!(
        rt.require_memory()
            .unwrap()
            .commit_longitudinal_consolidation(revised)
            .await
            .is_err()
    );
}

async fn assert_journal_consolidation(
    rt: &NousRuntime,
    subject: nous_core::SubjectId,
    episode: &EpisodeView,
    journal: &nous_memory::JournalView,
) {
    use nous_memory::{
        ExpectedCognition, LongitudinalConsolidationAction, LongitudinalConsolidationInput,
    };
    let input = LongitudinalConsolidationInput {
        operation_id: nous_core::OperationId::new(),
        subject,
        expected_authority_seq: rt.store.authority_seq(subject).await.unwrap(),
        source: ExpectedCognition {
            reference: nous_core::CognitiveRef::JournalRevision(
                journal.revision.journal_revision_id,
            ),
            expected_epoch: journal.object.object_epoch,
        },
        context: vec![],
        producer: consolidation_producer(),
        actions: vec![LongitudinalConsolidationAction::CreateSchema {
            content: consolidation_schema(episode),
        }],
    };
    let outcome = rt
        .require_memory()
        .unwrap()
        .commit_longitudinal_consolidation(input)
        .await
        .unwrap();
    let Some(nous_core::CognitiveRef::CognitiveSchemaRevision(revision)) = outcome.results[0]
    else {
        panic!("expected Schema revision")
    };
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
