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
    let mut tags = Vec::new();
    for label in ["Deployment approval", "Deployment rollout", "Garden"] {
        let tag = memory
            .create_tag(
                subject,
                CreateTagRequest {
                    operation_id: OperationId::new(),
                    label: label.into(),
                    description: None,
                    kind_hint: Some("topic".into()),
                    origin: "host_explicit".into(),
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
    for (from, to) in [(tags[0], tags[1]), (tags[1], tags[2])] {
        let edge = memory
            .create_association(
                CreateAssociationRequest {
                    operation_id: OperationId::new(),
                    from: CognitiveRef::Tag(from),
                    to: CognitiveRef::Tag(to),
                    relation_kind: "related".into(),
                    polarity: AssociationPolarity::Positive,
                    support_class: AssociationSupportClass::SourceEvidence,
                    supports: vec![support.clone()],
                    producer_signature_id: None,
                    valid_time: TemporalExtent::Unknown,
                },
                subject,
            )
            .await
            .unwrap();
        edge_ids.push(edge.association_evidence_id);
    }
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
}
