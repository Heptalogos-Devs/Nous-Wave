mod test_support;
use nous_core::*;
use nous_kernel::transport::KernelService;
use nous_memory::{
    AssociationPolarity, AssociationSupport, AssociationSupportClass, CreateAssociationRequest,
    CreateTagRequest, MergeTagsInput, SplitTagInput, TagContent, TagExpectation,
};
use nous_protocol::nous::wave::v1alpha1 as p;
use nous_subject::{CognitiveSeedInput, CreateSubject};
use p::topology_service_server::TopologyService;
use test_support::*;
use tonic::Request;

fn content(label: &str) -> TagContent {
    TagContent {
        label: label.into(),
        description: Some("Release process concept".into()),
        kind_hint: Some("procedure".into()),
    }
}
fn expectation(tag: &nous_memory::Tag) -> TagExpectation {
    TagExpectation {
        tag_id: tag.tag_id,
        expected_revision_id: tag.current_revision_id,
    }
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one lifecycle scenario verifies history, aliases, merge interpretation and split scope"
)]
async fn concept_lineage_preserves_history_and_canonicalizes_current_query_and_serving() {
    let (root, url, _postgres) = database().await;
    let rt = open_runtime(&url, &root).await;
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
    let mut tags = Vec::new();
    for label in [
        "Deployment approval",
        "Release authorization",
        "Release process",
    ] {
        tags.push(
            owner
                .create_tag(
                    subject,
                    CreateTagRequest {
                        operation_id: OperationId::new(),
                        label: label.into(),
                        description: None,
                        kind_hint: None,
                        origin: "host_explicit".into(),
                    },
                )
                .await
                .unwrap(),
        );
    }
    let (_, old_binding) = rt
        .store
        .resolve_identity(subject, "tag", "Deployment approval", false)
        .await
        .unwrap();
    let old_lexical = old_binding[0].lexical_ref.clone();
    let service = KernelService(rt.clone());
    let revise = p::ReviseTagRequest {
        operation_id: OperationId::new().0.to_string(),
        subject_id: subject.0.to_string(),
        target: Some(p::TagRevisionTarget {
            tag_id: tags[0].tag_id.0.to_string(),
            expected_revision_id: tags[0].current_revision_id.to_string(),
        }),
        content: Some(p::TagContent {
            label: "Release approval".into(),
            description: Some("Approval before rollout".into()),
            kind_hint: Some("procedure".into()),
        }),
    };
    let revised = TopologyService::revise_tag(&service, Request::new(revise.clone()))
        .await
        .unwrap()
        .into_inner();
    assert_ne!(
        revised.current_revision_id,
        tags[0].current_revision_id.to_string()
    );
    let replay = TopologyService::revise_tag(&service, Request::new(revise.clone()))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(revised.current_revision_id, replay.current_revision_id);
    let mut stale = revise.clone();
    stale.operation_id = OperationId::new().0.to_string();
    assert_eq!(
        TopologyService::revise_tag(&service, Request::new(stale))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Aborted
    );
    tags[0].current_revision_id = revised.current_revision_id.parse().unwrap();
    for alias in ["Deployment approval", "Release approval"] {
        let (status, result) = rt
            .store
            .resolve_identity(subject, "tag", alias, false)
            .await
            .unwrap();
        assert_eq!(status, "BOUND");
        assert_eq!(result[0].canonical, CognitiveRef::Tag(tags[0].tag_id));
    }
    let mut revisions = Vec::new();
    for text in [
        "Alice approved the October release before rollout",
        "Bob approved the November release before rollout",
    ] {
        let occurrence = observation(&rt, subject, text).await;
        let mut input = form_input(
            subject,
            occurrence.occurrence.occurrence_id,
            OperationId::new(),
            text,
        );
        input.tags = vec![tags[1].tag_id];
        revisions.push(
            owner
                .form_memory(input)
                .await
                .unwrap()
                .revision
                .memory_revision_id,
        );
    }
    let supports = revisions
        .iter()
        .map(|id| {
            RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: CognitiveRef::MemoryRevision(*id),
                support_role: SupportRole::Direct,
            })
        })
        .collect::<Vec<_>>();
    let association = owner
        .create_association(
            CreateAssociationRequest {
                operation_id: OperationId::new(),
                from: CognitiveRef::MemoryRevision(revisions[0]),
                to: CognitiveRef::Tag(tags[1].tag_id),
                relation_kind: "tag_attachment".into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::HostExplicit,
                supports: vec![AssociationSupport::Revision(supports[0].clone())],
                producer_signature_id: None,
                valid_time: TemporalExtent::Unknown,
            },
            subject,
        )
        .await
        .unwrap();
    let merge = MergeTagsInput {
        operation_id: OperationId::new(),
        survivor: expectation(&tags[0]),
        retired: vec![expectation(&tags[1])],
        supports: supports.clone(),
    };
    owner.merge_tags(subject, merge.clone()).await.unwrap();
    owner.merge_tags(subject, merge).await.unwrap();
    for name in [
        "Release authorization",
        "Release approval",
        "Deployment approval",
    ] {
        let (status, binding) = rt
            .store
            .resolve_identity(subject, "tag", name, false)
            .await
            .unwrap();
        assert_eq!(status, "BOUND");
        assert_eq!(binding[0].canonical, CognitiveRef::Tag(tags[0].tag_id));
        assert_eq!(binding[0].display_name, "Release approval");
    }
    let (_, binding) = rt
        .store
        .resolve_identity(subject, "tag", &old_lexical, true)
        .await
        .unwrap();
    assert_eq!(binding[0].canonical, CognitiveRef::Tag(tags[0].tag_id));
    let catalog = owner
        .plan_topology(subject, CognitiveRef::MemoryRevision(revisions[0]))
        .await
        .unwrap();
    assert!(
        catalog
            .tags
            .iter()
            .find(|t| t.target.tag_id == tags[0].tag_id)
            .unwrap()
            .aliases
            .contains(&"Release authorization".to_string())
    );
    let unchanged: uuid::Uuid =
        sqlx::query_scalar("SELECT tag_id FROM memory_revision_tags WHERE memory_revision_id=$1")
            .bind(revisions[0].0)
            .fetch_one(rt.store.pool())
            .await
            .unwrap();
    assert_eq!(unchanged, tags[1].tag_id.0);
    let evidence_endpoint: String = sqlx::query_scalar(
        "SELECT to_ref FROM association_evidence WHERE association_evidence_id=$1",
    )
    .bind(association.association_evidence_id.0)
    .fetch_one(rt.store.pool())
    .await
    .unwrap();
    assert_eq!(evidence_endpoint, tags[1].tag_id.0.to_string());
    let projection = rt
        .store
        .topology_projection_input(subject, true)
        .await
        .unwrap();
    assert!(
        projection
            .nodes
            .contains(&CognitiveRef::Tag(tags[0].tag_id))
    );
    assert!(
        !projection
            .nodes
            .contains(&CognitiveRef::Tag(tags[1].tag_id))
    );
    assert!(projection.edges.iter().any(|edge| edge.from
        == CognitiveRef::MemoryRevision(revisions[0])
        && edge.to == CognitiveRef::Tag(tags[0].tag_id)));
    let neighborhood = owner
        .association_neighborhood(subject, CognitiveRef::Tag(tags[1].tag_id), 64, 1)
        .await
        .unwrap();
    assert!(
        neighborhood
            .nodes
            .contains(&CognitiveRef::Tag(tags[0].tag_id))
    );
    assert_eq!(
        neighborhood.associations[0].to,
        CognitiveRef::Tag(tags[0].tag_id)
    );
    let bound = rt
        .cognition
        .bind_query(CognitiveQuery {
            api_version: API_VERSION,
            subject,
            text_only_compatibility: false,
            work_context: None,
            session: None,
            situation: Default::default(),
            expression: CognitiveQueryExpr {
                cues: vec![
                    Cue::Text(TextCue {
                        text: "release approval".into(),
                    }),
                    Cue::Tag(TagCue {
                        tag: tags[1].tag_id,
                    }),
                ],
                ..Default::default()
            },
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .unwrap();
    assert!(
        matches!(&bound.source_query.expression.cues[1],Cue::Tag(tag) if tag.tag==tags[0].tag_id)
    );
    assert!(bound.representation.text.contains("Release approval"));
    // A subsequent merge flattens interpretation; old immutable facts stay intact.
    owner
        .merge_tags(
            subject,
            MergeTagsInput {
                operation_id: OperationId::new(),
                survivor: expectation(&tags[2]),
                retired: vec![expectation(&tags[0])],
                supports: supports.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        rt.store
            .canonical_tag_id(subject, tags[1].tag_id)
            .await
            .unwrap(),
        tags[2].tag_id
    );
    let split = SplitTagInput {
        operation_id: OperationId::new(),
        parent: expectation(&tags[2]),
        children: vec![content("Approval policy"), content("Rollout procedure")],
        supports: supports.clone(),
    };
    let children = owner.split_tag(subject, split.clone()).await.unwrap();
    let replay = owner.split_tag(subject, split.clone()).await.unwrap();
    assert_eq!(
        children.iter().map(|t| t.tag_id).collect::<Vec<_>>(),
        replay.iter().map(|t| t.tag_id).collect::<Vec<_>>()
    );
    assert_eq!(
        rt.store
            .canonical_tag_id(subject, tags[2].tag_id)
            .await
            .unwrap(),
        tags[2].tag_id
    );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM tags WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    let mut invalid = split;
    invalid.operation_id = OperationId::new();
    invalid.supports = vec![supports[0].clone(), supports[0].clone()];
    assert!(owner.split_tag(subject, invalid).await.is_err());
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM tags WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(before, after);
    let lineage: i64 = sqlx::query_scalar("SELECT count(*) FROM tag_lineage WHERE subject_id=$1")
        .bind(subject.0)
        .fetch_one(rt.store.pool())
        .await
        .unwrap();
    assert_eq!(lineage, 4);
}
