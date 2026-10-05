mod test_support;
use nous_core::*;
use nous_kernel::NousRuntime;
use nous_memory::*;
use nous_runtime::{CognitiveProfile, MaintenanceDisposition, MaintenanceNeed};
use nous_subject::{CognitiveSeedInput, CreateSubject};
use test_support::*;
fn producer() -> ProducerSignature {
    ProducerSignature {
        signature_hash: String::new(),
        provider_class: "deterministic-fixture".into(),
        operation: CapabilityOperation::TopologyMaintenanceText,
        implementation: "topology-fixture".into(),
        model_identity: Some("scope-aware-fixture".into()),
        model_revision: None,
        output_schema_digest: Some("a".repeat(64)),
        preprocessing_identity: "program/topology/maintenance.md".into(),
        preprocessing_revision: "fixture-v1".into(),
        config_digest: "b".repeat(64),
    }
}
fn support(plan: &TopologyPlan, reference: &CognitiveRef) -> String {
    plan.supports
        .iter()
        .find_map(|(key, value)| match value {
            AssociationSupport::Revision(RevisionSupport::CognitionDependency(d))
                if &d.target_revision == reference =>
            {
                Some(key.clone())
            }
            _ => None,
        })
        .unwrap()
}
async fn claim(rt: &NousRuntime, subject: SubjectId) -> MaintenanceNeed {
    rt.cognition
        .lease_maintenance(subject, &["topology_maintenance".into()], 1, 120, None)
        .await
        .unwrap()
        .pop()
        .unwrap()
}
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one formation-maintenance-serving scenario verifies automatic queue and atomic proposal lifecycle"
)]
async fn formation_maintains_reusable_concepts_and_serves_supported_associations() {
    let (root, url, _postgres) = database().await;
    let rt = open_runtime_with_serving(&url, &root, true, false, true).await;
    let subject = rt
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
    let owner = rt.require_memory().unwrap();
    let mut revisions = Vec::new();
    for text in [
        "Alice requires recorded reviewer approval before deployment",
        "Bob requires recorded reviewer approval before releasing the service",
    ] {
        let occurrence = observation(&rt, subject, text).await;
        let view = owner
            .form_memory(form_input(
                subject,
                occurrence.occurrence.occurrence_id,
                OperationId::new(),
                text,
            ))
            .await
            .unwrap();
        revisions.push(CognitiveRef::MemoryRevision(
            view.revision.memory_revision_id,
        ));
    }
    let first = claim(&rt, subject).await;
    let focus = parse_reference(&first.scope_kind, &first.scope_ref).unwrap();
    assert_eq!(focus, revisions[0]);
    use k::kernel_maintenance_service_server::KernelMaintenanceService;
    use nous_protocol::nous::wave::kernel::v1alpha1 as k;
    let ts = |date: chrono::DateTime<chrono::Utc>| prost_types::Timestamp {
        seconds: date.timestamp(),
        nanos: i32::try_from(date.timestamp_subsec_nanos()).unwrap(),
    };
    let wire = k::MaintenanceNeed {
        need_id: first.need_id.to_string(),
        subject_id: subject.0.to_string(),
        kind: first.kind.clone(),
        scope_kind: first.scope_kind.clone(),
        scope_ref: first.scope_ref.clone(),
        trigger_authority_seq: first.trigger_authority_seq,
        trigger_revision: first.trigger_revision,
        due_at: Some(ts(first.due_at)),
        created_at: Some(ts(first.created_at)),
        updated_at: Some(ts(first.updated_at)),
        state: first.state.clone(),
        lease_token: first.lease_token.map(|t| t.to_string()),
        lease_until: first.lease_until.map(ts),
        ..Default::default()
    };
    let service = nous_kernel::transport::KernelService(rt.clone());
    let reply = KernelMaintenanceService::plan_maintenance(
        &service,
        tonic::Request::new(k::PlanMaintenanceRequest {
            claimed: Some(wire.clone()),
        }),
    )
    .await
    .unwrap()
    .into_inner();
    assert!(reply.topology_plan_json.is_some());
    let mut wrong_scope = wire;
    wrong_scope.scope_ref = reference_parts(&revisions[1]).1;
    assert!(
        KernelMaintenanceService::plan_maintenance(
            &service,
            tonic::Request::new(k::PlanMaintenanceRequest {
                claimed: Some(wrong_scope)
            })
        )
        .await
        .is_err()
    );
    let plan = owner.plan_topology(subject, focus.clone()).await.unwrap();
    let model = plan.model_input().to_string();
    assert!(!model.contains(&subject.0.to_string()));
    assert!(!model.contains(&reference_parts(&focus).1));
    assert!(model.contains("exact_cognition"));
    let proof = support(&plan, &focus);
    let input = CommitTopologyInput {
        operation_id: OperationId::new(),
        claimed: first.clone(),
        plan: plan.clone(),
        producer: producer(),
        proposal: TopologyProposal {
            actions: vec![
                TopologyAction::CreateTag {
                    key: "new_release_approval".into(),
                    content: TagContent {
                        label: "Release approval".into(),
                        description: Some(
                            "Recorded reviewer approval required before rollout".into(),
                        ),
                        kind_hint: Some("procedure".into()),
                    },
                    support_keys: vec![proof.clone()],
                    reason: "This accepted procedure expresses a durable release control".into(),
                },
                TopologyAction::AttachTag {
                    cognition_key: "c0".into(),
                    tag_key: "new_release_approval".into(),
                    support_keys: vec![proof],
                    reason: "The focus directly expresses release approval".into(),
                },
            ],
        },
    };
    let outcome = owner.commit_topology(input.clone()).await.unwrap();
    assert_eq!(outcome.changes, 2);
    assert_eq!(
        owner.commit_topology(input).await.unwrap().results,
        outcome.results
    );
    let tag = outcome.results["new_release_approval"].clone();
    rt.cognition
        .acknowledge_maintenance(&first, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    let second = claim(&rt, subject).await;
    let focus = parse_reference(&second.scope_kind, &second.scope_ref).unwrap();
    assert_eq!(focus, revisions[1]);
    let plan = owner.plan_topology(subject, focus.clone()).await.unwrap();
    let tag_key = plan
        .tags
        .iter()
        .find(|t| CognitiveRef::Tag(t.target.tag_id) == tag)
        .unwrap()
        .key
        .clone();
    let earlier = plan
        .cognition
        .iter()
        .find(|c| c.reference == revisions[0])
        .unwrap()
        .key
        .clone();
    let proof = support(&plan, &focus);
    let earlier_proof = support(&plan, &revisions[0]);
    let mut invalid = CommitTopologyInput {
        operation_id: OperationId::new(),
        claimed: second.clone(),
        plan: plan.clone(),
        producer: producer(),
        proposal: TopologyProposal {
            actions: vec![
                TopologyAction::CreateTag {
                    key: "new_bogus".into(),
                    content: TagContent {
                        label: "Must roll back".into(),
                        description: None,
                        kind_hint: None,
                    },
                    support_keys: vec![proof.clone()],
                    reason: "Fixture checks atomic rollback before invalid reference".into(),
                },
                TopologyAction::AttachTag {
                    cognition_key: "c0".into(),
                    tag_key: "invented_uuid".into(),
                    support_keys: vec![proof.clone()],
                    reason: "Bogus key must reject the whole proposal".into(),
                },
            ],
        },
    };
    assert!(owner.commit_topology(invalid.clone()).await.is_err());
    let bogus: i64 =
        sqlx::query_scalar("SELECT count(*) FROM tag_revisions WHERE label='Must roll back'")
            .fetch_one(rt.store.pool())
            .await
            .unwrap();
    assert_eq!(bogus, 0);
    for relation in [
        TopologyRelation::CoOccurs,
        TopologyRelation::Sequence,
        TopologyRelation::Procedural,
        TopologyRelation::SharedOutcome,
    ] {
        invalid.operation_id = OperationId::new();
        invalid.proposal = TopologyProposal {
            actions: vec![TopologyAction::CreateAssociation {
                from_key: "c0".into(),
                to_key: earlier.clone(),
                relation,
                support_keys: vec![proof.clone(), earlier_proof.clone()],
                reason: "Lexical overlap and independent observations cannot prove this relation"
                    .into(),
            }],
        };
        let error = owner.commit_topology(invalid.clone()).await.unwrap_err();
        assert!(error.to_string().contains("needs"), "{error}");
    }
    invalid.operation_id = OperationId::new();
    invalid.proposal = TopologyProposal {
        actions: vec![
            TopologyAction::ReuseTag {
                tag_key: tag_key.clone(),
            },
            TopologyAction::AttachTag {
                cognition_key: "c0".into(),
                tag_key,
                support_keys: vec![proof.clone()],
                reason: "The new accepted cognition embodies the same stable procedure".into(),
            },
            TopologyAction::CreateAssociation {
                from_key: "c0".into(),
                to_key: earlier,
                relation: TopologyRelation::Related,
                support_keys: vec![proof, earlier_proof],
                reason: "Both exact inputs specify recorded release approval".into(),
            },
        ],
    };
    assert_eq!(owner.commit_topology(invalid).await.unwrap().changes, 2);
    rt.cognition
        .acknowledge_maintenance(&second, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    let tags: i64 = sqlx::query_scalar("SELECT count(*) FROM tags WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(tags, 1);
    let historical_tags: i64 =
        sqlx::query_scalar("SELECT count(*) FROM memory_revision_tags WHERE memory_revision_id=$1")
            .bind(match revisions[0] {
                CognitiveRef::MemoryRevision(id) => id.0,
                _ => unreachable!(),
            })
            .fetch_one(rt.store.pool())
            .await
            .unwrap();
    assert_eq!(historical_tags, 0);
    let before = owner
        .accretion_signals(subject, std::slice::from_ref(&tag))
        .await
        .unwrap()
        .remove(&tag)
        .unwrap();
    assert_eq!(before.attached_cognition, 2);
    assert_eq!(before.independent_roots, 2);
    assert_eq!(before.meaningful_use, 0);
    let session = rt
        .cognition
        .open_session(subject, serde_json::json!({}))
        .await
        .unwrap();
    let events = (0..3)
        .map(|_| nous_runtime::UseFeedbackEvent {
            event_id: UseEventId::new(),
            reference: revisions[0].clone(),
            use_kind: nous_runtime::UseKind::ActedOn,
            occurred_at: rt.cognition.now(subject),
            context: serde_json::json!({}),
        })
        .collect();
    rt.cognition
        .use_feedback(nous_runtime::UseFeedback {
            subject,
            session_id: Some(session.session_id),
            consumer_ref: "consumer:accretion:fixture".into(),
            events,
        })
        .await
        .unwrap();
    let after = owner
        .accretion_signals(subject, std::slice::from_ref(&tag))
        .await
        .unwrap()
        .remove(&tag)
        .unwrap();
    assert_eq!(after.meaningful_use, 3);
    assert!(after.usefulness > before.usefulness);
    assert_eq!(after.independent_roots, before.independent_roots);
    let truth: String = sqlx::query_scalar(
        "SELECT epistemic_class FROM memory_revisions WHERE memory_revision_id=$1",
    )
    .bind(match revisions[0] {
        CognitiveRef::MemoryRevision(id) => id.0,
        _ => unreachable!(),
    })
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(truth, "observed");
    let queued:i64=sqlx::query_scalar("SELECT count(*) FROM maintenance_needs WHERE subject_id=$1 AND kind='topology_maintenance' AND state='pending'").bind(subject.0).fetch_one(rt.store.pool()).await.unwrap();
    assert_eq!(queued, 1);
    let review = claim(&rt, subject).await;
    rt.cognition
        .acknowledge_maintenance(&review, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    rt.cognition
        .use_feedback(nous_runtime::UseFeedback {
            subject,
            session_id: Some(session.session_id),
            consumer_ref: "consumer:accretion:fixture".into(),
            events: [
                nous_runtime::UseKind::Presented,
                nous_runtime::UseKind::ResultRefuted,
            ]
            .into_iter()
            .map(|use_kind| nous_runtime::UseFeedbackEvent {
                event_id: UseEventId::new(),
                reference: revisions[0].clone(),
                use_kind,
                occurred_at: rt.cognition.now(subject),
                context: serde_json::json!({}),
            })
            .collect(),
        })
        .await
        .unwrap();
    let negative = owner
        .accretion_signals(subject, std::slice::from_ref(&tag))
        .await
        .unwrap()
        .remove(&tag)
        .unwrap();
    assert_eq!(negative.meaningful_use, 3);
    assert_eq!(negative.counterevidence, 1);
    assert!(negative.usefulness < after.usefulness);
    let queued:i64=sqlx::query_scalar("SELECT count(*) FROM maintenance_needs WHERE subject_id=$1 AND kind='topology_maintenance' AND state='pending'").bind(subject.0).fetch_one(rt.store.pool()).await.unwrap();
    assert_eq!(
        queued, 0,
        "exposure/refutation cannot cross a positive use interval"
    );
    let graph = rt
        .store
        .topology_projection_input(subject, true)
        .await
        .unwrap();
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.association_kind == "tag_attachment" && edge.to == tag)
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.association_kind == "assoc.related")
    );
    let tag_id = match tag {
        CognitiveRef::Tag(id) => id,
        _ => unreachable!(),
    };
    let query = CognitiveQuery {
        api_version: API_VERSION,
        subject,
        text_only_compatibility: false,
        work_context: None,
        session: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            cues: vec![Cue::Tag(TagCue { tag: tag_id })],
            ..Default::default()
        },
        exploration: ExplorationIntent::AroundTag,
        resources: Default::default(),
        result_need: Default::default(),
        effort: Default::default(),
        capabilities: CapabilityPolicy {
            text_embedding: RequirementStrength::Forbidden,
            ..Default::default()
        },
        diagnostics: Default::default(),
    };
    let bound = rt
        .cognition
        .bind_query(query)
        .await
        .unwrap()
        .for_profile(CognitiveProfile::NousNodePotential)
        .unwrap();
    let execution = rt.execute_bound_query(bound, Some(32)).await.unwrap();
    for revision in &revisions {
        assert!(
            execution
                .result
                .results
                .iter()
                .any(|hit| hit.revision.as_ref() == Some(revision) || &hit.reference == revision),
            "{:#?}",
            execution.result
        );
    }
    assert!(execution.result.results.iter().any(|hit| {
        hit.match_evidence
            .families
            .contains(&EvidenceFamily::TopologyWave)
    }));
    drop(execution);
    episode_relations(&rt, subject, &revisions).await;
}

async fn episode_relations(rt: &NousRuntime, subject: SubjectId, revisions: &[CognitiveRef]) {
    let owner = rt.require_memory().unwrap();
    let episode = owner
        .create_episode(EpisodeInput {
            operation_id: OperationId::new(),
            subject,
            track_key: "relation-witness".into(),
            title: Some("Recorded approval sequence".into()),
            parent_episode_revision_id: None,
            experience_time: TemporalExtent::Unknown,
            boundary_explanation: "Both exact members recorded in order".into(),
            producer_signature_id: None,
            members: revisions
                .iter()
                .enumerate()
                .map(|(index, reference)| EpisodeMemberInput {
                    reference: reference.clone(),
                    role: if index == 0 { "procedure" } else { "outcome" }.into(),
                })
                .collect(),
            supports: revisions
                .iter()
                .map(|reference| {
                    RevisionSupport::CognitionDependency(CognitionDependency {
                        target_revision: reference.clone(),
                        support_role: SupportRole::Direct,
                    })
                })
                .collect(),
        })
        .await
        .unwrap();
    let claimed = claim(rt, subject).await;
    let focus = parse_reference(&claimed.scope_kind, &claimed.scope_ref).unwrap();
    let plan = owner.plan_topology(subject, focus).await.unwrap();
    let key = |reference: &CognitiveRef| {
        plan.cognition
            .iter()
            .find(|c| &c.reference == reference)
            .unwrap()
            .key
            .clone()
    };
    let evidence = vec![
        support(&plan, &revisions[0]),
        support(&plan, &revisions[1]),
        support(
            &plan,
            &CognitiveRef::EpisodeRevision(episode.revision.episode_revision_id),
        ),
    ];
    let proposal = |from: usize, to: usize, relation| TopologyAction::CreateAssociation {
        from_key: key(&revisions[from]),
        to_key: key(&revisions[to]),
        relation,
        support_keys: evidence.clone(),
        reason: "Exact selected Episode members establish relation".into(),
    };
    let mut input = CommitTopologyInput {
        operation_id: OperationId::new(),
        claimed: claimed.clone(),
        plan: plan.clone(),
        producer: producer(),
        proposal: TopologyProposal {
            actions: vec![proposal(1, 0, TopologyRelation::Sequence)],
        },
    };
    let error = owner.commit_topology(input.clone()).await.unwrap_err();
    assert!(error.to_string().contains("sequence needs"), "{error}");
    input.operation_id = OperationId::new();
    input.proposal.actions = vec![
        proposal(0, 1, TopologyRelation::Sequence),
        proposal(0, 1, TopologyRelation::CoOccurs),
        proposal(0, 1, TopologyRelation::Procedural),
        proposal(0, 1, TopologyRelation::SharedOutcome),
    ];
    assert_eq!(owner.commit_topology(input).await.unwrap().changes, 4);
    rt.cognition
        .acknowledge_maintenance(&claimed, MaintenanceDisposition::Satisfied)
        .await
        .unwrap();
    // Episode review remains claimable even though it is not an Accretion center.
    owner.prioritize_topology_needs(subject).await.unwrap();
}
