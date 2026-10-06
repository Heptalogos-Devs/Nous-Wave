mod test_support;
use nous_core::*;
use nous_kernel::transport::KernelService;
use nous_memory::{
    AssociationPolarity, AssociationSupport, AssociationSupportClass, CreateAssociationRequest,
    CreateTagRequest,
};
use nous_protocol::nous::wave::v1alpha1 as p;
use nous_subject::{CognitiveSeedInput, CreateSubject};
use p::topology_service_server::TopologyService;
use test_support::*;
use tonic::Request;

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one Subject scenario verifies paging and supported graph lifecycle"
)]
async fn agent_topology_reads_supported_edges_and_searches_the_tag_catalog() {
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
    let memory = rt.require_memory().unwrap();
    let producer = ProducerSignature {
        signature_hash: String::new(),
        provider_class: "deterministic_test".into(),
        operation: CapabilityOperation::ConceptMaintenanceText,
        implementation: "concept-proposal-test".into(),
        model_identity: None,
        model_revision: None,
        output_schema_digest: None,
        preprocessing_identity: "concept-maintenance".into(),
        preprocessing_revision: "1".into(),
        config_digest: "test-config".into(),
    };
    let mut tags = Vec::new();
    for label in ["Deployment approval", "Deployment rollout", "Garden"] {
        let tag = memory
            .create_tag(
                subject,
                CreateTagRequest {
                    producer: Some(producer.clone()),
                    operation_id: OperationId::new(),
                    label: label.into(),
                    description: None,
                    kind_hint: Some("topic".into()),
                    origin: "concept_maintenance".into(),
                },
            )
            .await
            .unwrap();
        tags.push(tag.tag_id);
    }
    rt.store
        .bind_identity(
            subject,
            CognitiveRef::Tag(tags[2]),
            "Garden".into(),
            vec!["deployment-garden".into()],
        )
        .await
        .unwrap();
    let observed = observation(
        &rt,
        subject,
        "Deployment approval precedes rollout; rollout includes the Garden environment",
    )
    .await;
    let support = AssociationSupport::Revision(RevisionSupport::Evidence(EvidenceRef {
        occurrence_id: observed.occurrence.occurrence_id,
        locator: EvidenceLocator::WholeOccurrence,
        support_role: SupportRole::Direct,
    }));
    let mut edge_ids = Vec::new();
    let mut requests = Vec::new();
    for (from, to) in [(tags[0], tags[1]), (tags[1], tags[2])] {
        let request = CreateAssociationRequest {
            producer: Some(producer.clone()),
            operation_id: OperationId::new(),
            from: CognitiveRef::Tag(from),
            to: CognitiveRef::Tag(to),
            relation_kind: "assoc.related".into(),
            polarity: AssociationPolarity::Positive,
            support_class: AssociationSupportClass::CognitiveDerivation,
            supports: vec![support.clone()],
            producer_signature_id: None,
            valid_time: TemporalExtent::Unknown,
        };
        let edge = memory
            .create_association(request.clone(), subject)
            .await
            .unwrap();
        let replay = memory
            .create_association(request.clone(), subject)
            .await
            .unwrap();
        assert_eq!(edge.association_evidence_id, replay.association_evidence_id);
        requests.push(request.clone());
        let mut invalid = request;
        invalid.operation_id = OperationId::new();
        invalid.relation_kind = "assoc.invented".into();
        assert!(matches!(
            memory.create_association(invalid, subject).await,
            Err(Error::Invalid(_))
        ));
        edge_ids.push(edge.association_evidence_id);
    }
    let producers: i64 = sqlx::query_scalar("SELECT count(DISTINCT producer_signature_id) FROM tag_revisions WHERE tag_id=ANY($1::uuid[])")
        .bind(tags.iter().map(|tag| tag.0).collect::<Vec<_>>()).fetch_one(rt.store.pool()).await.unwrap();
    assert_eq!(producers, 1);
    let association_producers: i64 = sqlx::query_scalar("SELECT count(DISTINCT producer_signature_id) FROM association_evidence WHERE subject_id=$1")
        .bind(subject.0).fetch_one(rt.store.pool()).await.unwrap();
    assert_eq!(association_producers, 1);
    let service = KernelService(rt.clone());
    let search = |text: &str, token: &str| p::SearchTagsRequest {
        subject_id: subject.0.to_string(),
        text: text.into(),
        page: Some(p::Page {
            page_size: 1,
            page_token: token.into(),
        }),
    };
    let first = TopologyService::search_tags(&service, Request::new(search("Deployment", "")))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(first.items.len(), 1);
    assert!(!first.next_page_token.is_empty());
    let second = TopologyService::search_tags(
        &service,
        Request::new(search("Deployment", &first.next_page_token)),
    )
    .await
    .unwrap()
    .into_inner();
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].tag_id, second.items[0].tag_id);
    let third = TopologyService::search_tags(
        &service,
        Request::new(search("Deployment", &second.next_page_token)),
    )
    .await
    .unwrap()
    .into_inner();
    assert_eq!(third.items[0].tag_id, tags[2].0.to_string());
    assert!(third.next_page_token.is_empty());
    assert!(
        TopologyService::search_tags(
            &service,
            Request::new(search("Garden", &first.next_page_token))
        )
        .await
        .is_err()
    );
    let request = |nodes, depth| p::NeighborhoodRequest {
        subject_id: subject.0.to_string(),
        root: Some(p::CognitiveRef {
            kind: "tag".into(),
            value: tags[0].0.to_string(),
        }),
        max_nodes: nodes,
        max_depth: depth,
    };
    let one = TopologyService::get_neighborhood(&service, Request::new(request(64, 1)))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(one.nodes.len(), 2);
    assert_eq!(one.associations.len(), 1);
    assert_eq!(one.associations[0].supports.len(), 1);
    let two = TopologyService::get_neighborhood(&service, Request::new(request(64, 2)))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(two.nodes.len(), 3);
    assert_eq!(two.associations.len(), 2);
    let limited = TopologyService::get_neighborhood(&service, Request::new(request(1, 2)))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(limited.nodes.len(), 1);
    assert!(limited.truncated);
    memory
        .revoke_association(subject, edge_ids[0], OperationId::new())
        .await
        .unwrap();
    let revoked = TopologyService::get_neighborhood(&service, Request::new(request(64, 2)))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(revoked.nodes.len(), 1);
    assert!(revoked.associations.is_empty());
    let store = &rt.store;
    let revision = |tag: TagId| async move {
        sqlx::query_scalar::<_, uuid::Uuid>("SELECT current_revision_id FROM tags WHERE tag_id=$1")
            .bind(tag.0)
            .fetch_one(store.pool())
            .await
            .unwrap()
    };
    memory
        .merge_tags(
            subject,
            nous_memory::MergeTagsInput {
                operation_id: OperationId::new(),
                survivor: nous_memory::TagExpectation {
                    tag_id: tags[2],
                    expected_revision_id: revision(tags[2]).await,
                },
                retired: vec![nous_memory::TagExpectation {
                    tag_id: tags[1],
                    expected_revision_id: revision(tags[1]).await,
                }],
                supports: vec![match support {
                    AssociationSupport::Revision(support) => support,
                    _ => unreachable!(),
                }],
            },
        )
        .await
        .unwrap();
    let replay = memory
        .create_association(requests[1].clone(), subject)
        .await
        .unwrap();
    assert_eq!(replay.association_evidence_id, edge_ids[1]);
    let mut stale = requests[1].clone();
    stale.operation_id = OperationId::new();
    assert!(matches!(
        memory.create_association(stale, subject).await,
        Err(Error::Conflict(_))
    ));
}
