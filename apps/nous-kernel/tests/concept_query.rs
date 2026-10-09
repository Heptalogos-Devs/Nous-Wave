// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
mod test_support;
use nous_core::*;
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_memory::{CreateTagRequest, ReviseTagInput, TagContent, TagExpectation};
use nous_retrieval::{
    ServingOptions, TextEmbeddingOutput, TextEmbeddingProvider, TextEmbeddingRequest,
};
use nous_runtime::{COGNITIVE_PROFILE, CONCEPT_ENRICHMENT};
use nous_subject::{CognitiveSeedInput, CreateSubject};
use std::sync::{Arc, Mutex};
use test_support::*;
#[derive(Default)]
struct Probe {
    calls: Mutex<Vec<(bool, String)>>,
}
#[async_trait::async_trait]
impl TextEmbeddingProvider for Probe {
    fn space(&self) -> EmbeddingSpaceSignature {
        EmbeddingSpaceSignature {
            space_hash: "concept-test".into(),
            model_identity: "probe".into(),
            weights_revision: "1".into(),
            task: "retrieval".into(),
            input_representation: "text".into(),
            preprocessing_identity: "utf8".into(),
            preprocessing_revision: "1".into(),
            dimension: 2,
            normalization: "none".into(),
            output_semantics: "dense_vector".into(),
        }
    }
    fn producer(&self) -> ProducerSignature {
        ProducerSignature {
            model_role: None,
            model_profile: None,
            execution_profile: None,
            inference_controls_digest: None,
            role_policy_digest: None,
            prompt_id: None,
            prompt_digest: None,

            signature_hash: "concept-probe".into(),
            provider_class: "deterministic_test".into(),
            operation: CapabilityOperation::TextEmbedding,
            implementation: "concept-test".into(),
            model_identity: None,
            model_revision: None,
            output_schema_digest: None,
            preprocessing_identity: "utf8".into(),
            preprocessing_revision: "1".into(),
            config_digest: "test".into(),
        }
    }
    async fn embed(&self, request: TextEmbeddingRequest) -> Result<TextEmbeddingOutput> {
        let vector = if request.text.contains("orthogonal policy probe") {
            vec![0.0, 1.0]
        } else {
            vec![1.0, 0.0]
        };
        self.calls
            .lock()
            .unwrap()
            .push((request.query, request.text));
        Ok(TextEmbeddingOutput {
            vector,
            space: self.space(),
            producer: self.producer(),
        })
    }
}
fn query(subject: SubjectId, mut cues: Vec<Cue>) -> CognitiveQuery {
    if !cues.iter().any(|cue| matches!(cue, Cue::Text(_))) {
        cues.insert(
            0,
            Cue::Text(TextCue {
                text: "Recall the selected concepts".into(),
            }),
        );
    }
    CognitiveQuery {
        api_version: API_VERSION,
        subject,
        session: None,
        projection: Default::default(),
        temporal_frame: Default::default(),

        work_context: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            cues,
            ..Default::default()
        },
        exploration: Default::default(),
        resources: Default::default(),
        result_need: Default::default(),
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: DiagnosticsRequest::Full,
    }
}
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one integration scenario distinguishes bare/direct/explore, shared vectors and ephemeral enrichment"
)]
async fn direct_concept_recall_is_independent_and_vectors_are_shared_across_profiles() {
    let (root, url, _pg) = database().await;
    let probe = Arc::new(Probe::default());
    let rt=NousRuntime::open(RuntimeOptions {postgres_url:url,max_connections:4,acquire_timeout_ms:15000,object_root:root.path().join("objects").to_string_lossy().into_owned(),serving_options:ServingOptions {root:root.path().join("serving"),lexical:true,dense:true,topology:true,memory_enabled:true},embedding:Some(probe.clone()),stored_embedding:None,core_descriptors:vec![],deployment_document:serde_json::json!({"capabilities":{"process":{"memory":true},"subject_defaults":{"memory":true}},"serving":{"lexical":{"enabled":true},"dense":{"enabled":true},"topology":{"enabled":true}}})}).await.unwrap();
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
    let tag = owner
        .create_tag(
            subject,
            CreateTagRequest {
                operation_id: OperationId::new(),
                label: "reader reclamation".into(),
                description: Some(
                    "Wait for all active readers before freeing the old generation".into(),
                ),
                kind_hint: Some("procedure".into()),
                origin: "host_explicit".into(),
                producer: None,
            },
        )
        .await
        .unwrap();
    let observed = observation(
        &rt,
        subject,
        "Reader lease retirement safely frees generation assets",
    )
    .await;
    let mut input = form_input(
        subject,
        observed.occurrence.occurrence_id,
        OperationId::new(),
        "Reader lease retirement safely frees generation assets",
    );
    input.tags = vec![tag.tag_id];
    let memory = owner.form_memory(input).await.unwrap();
    let revision = CognitiveRef::MemoryRevision(memory.revision.memory_revision_id);
    let text = Cue::Text(TextCue {
        text: "reader lease retirement".into(),
    });
    let bare = rt
        .execute_query(query(subject, vec![text.clone()]), None)
        .await
        .unwrap();
    assert!(
        bare.result
            .results
            .iter()
            .any(|hit| hit.reference == revision)
    );
    assert!(bare.bound.activation.inferred_tags.is_empty());
    assert_eq!(bare.bound.activation.model_calls, 0);
    assert!(
        !bare
            .bound
            .enabled_lanes
            .contains(&EvidenceFamily::TagDirect)
    );
    assert!(
        !bare
            .bound
            .enabled_lanes
            .contains(&EvidenceFamily::TopologyWave)
    );
    assert!(
        rt.serving
            .publisher
            .snapshot_for(subject)
            .concept
            .is_empty()
    );
    let mut direct = query(subject, vec![Cue::Tag(TagCue { tag: tag.tag_id })]);
    direct.capabilities.text_embedding = RequirementStrength::Forbidden;
    let direct = rt.execute_query(direct, None).await.unwrap();
    assert_eq!(direct.result.results.len(), 1);
    assert_eq!(direct.result.results[0].reference, revision);
    assert!(
        direct.result.results[0]
            .match_evidence
            .families
            .contains(&EvidenceFamily::TagDirect)
    );
    assert!(
        !direct
            .bound
            .enabled_lanes
            .contains(&EvidenceFamily::TopologyWave)
    );
    assert!(
        probe
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|(_, text)| !text.starts_with("Concept:"))
    );
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            CONCEPT_ENRICHMENT.path(),
            serde_json::json!("existing"),
            None,
        )
        .await
        .unwrap();
    let before = rt.store.authority_seq(subject).await.unwrap();
    let count = probe
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(query, _)| *query)
        .count();
    let enriched = rt
        .execute_query(query(subject, vec![text.clone()]), None)
        .await
        .unwrap();
    assert!(
        enriched
            .bound
            .activation
            .inferred_tags
            .iter()
            .any(|activation| activation.tag == tag.tag_id)
    );
    assert_eq!(enriched.bound.activation.model_calls, 0);
    assert_eq!(rt.store.authority_seq(subject).await.unwrap(), before);
    assert_eq!(
        probe
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(query, _)| *query)
            .count(),
        count + 1
    );
    let calls_before_prepare = probe.calls.lock().unwrap().len();
    let prepared = rt
        .cognition
        .bind_query(query(subject, vec![text.clone()]))
        .await
        .unwrap();
    assert_eq!(probe.calls.lock().unwrap().len(), calls_before_prepare);
    assert!(prepared.activation.inferred_tags.is_empty());
    let mut forbidden = query(subject, vec![text.clone()]);
    forbidden.capabilities.text_embedding = RequirementStrength::Forbidden;
    let degraded = rt.execute_query(forbidden, None).await.unwrap();
    assert!(
        degraded
            .result
            .results
            .iter()
            .any(|hit| hit.reference == revision)
    );
    assert!(degraded.bound.activation.inferred_tags.is_empty());
    assert!(
        degraded
            .bound
            .activation
            .degradation
            .iter()
            .any(|d| d.code == "concept_enrichment_unavailable")
    );
    assert_eq!(probe.calls.lock().unwrap().len(), calls_before_prepare);
    let concept_id = enriched.bound.activation.concept_generation.unwrap();
    for profile in [
        "vcp-dtsc-v9.2.1-adapter-v1",
        "vcp-rivermemo-v3.1-adapter-v1",
        "nous-node-potential-v1",
    ] {
        rt.configuration
            .set_subject_override(
                OperationId::new(),
                subject,
                COGNITIVE_PROFILE.path(),
                serde_json::json!(profile),
                None,
            )
            .await
            .unwrap();
        let mut explored = query(
            subject,
            vec![text.clone(), Cue::Tag(TagCue { tag: tag.tag_id })],
        );
        explored.exploration = ExplorationIntent::BoundedAssociative;
        let result = rt.execute_query(explored, None).await.unwrap();
        assert!(
            result
                .bound
                .enabled_lanes
                .contains(&EvidenceFamily::TopologyWave)
        );
        let snapshot = rt.serving.publisher.snapshot_for(subject);
        let concept = snapshot
            .concept
            .iter()
            .find(|generation| generation.space.is_some())
            .unwrap();
        assert_eq!(concept.generation_id, concept_id);
        if let Some(vcp) = &snapshot.vcp {
            let id = vcp.identities.id(&CognitiveRef::Tag(tag.tag_id)).unwrap();
            assert_eq!(
                vcp.vectors.iter().find(|(key, _)| *key == id).unwrap().1,
                concept.record(tag.tag_id).unwrap().vector.clone().unwrap()
            );
        }
    }
    assert_eq!(
        probe
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(query, text)| !*query && text.starts_with("Concept:"))
            .count(),
        1
    );
    let old_snapshot = rt.serving.publisher.snapshot_for(subject);
    let historical_cut = rt.cognition.now(subject);
    let revised = owner
        .revise_tag(
            subject,
            ReviseTagInput {
                operation_id: OperationId::new(),
                target: TagExpectation {
                    tag_id: tag.tag_id,
                    expected_revision_id: tag.current_revision_id,
                },
                content: TagContent {
                    label: "reader reclamation".into(),
                    description: Some(
                        "Retain immutable historical assets until every reader releases its lease"
                            .into(),
                    ),
                    kind_hint: Some("procedure".into()),
                },
                producer: None,
            },
        )
        .await
        .unwrap();
    let updated = rt
        .execute_query(query(subject, vec![text]), None)
        .await
        .unwrap();
    assert_ne!(
        updated.bound.activation.concept_generation,
        Some(concept_id)
    );
    let snapshot = rt.serving.publisher.snapshot_for(subject);
    let concept = snapshot
        .concept
        .iter()
        .find(|g| g.generation_id == updated.bound.activation.concept_generation.unwrap())
        .unwrap();
    assert_eq!(
        concept.record(tag.tag_id).unwrap().revision,
        revised.current_revision_id
    );
    assert!(
        concept
            .record(tag.tag_id)
            .unwrap()
            .semantic
            .text
            .contains("historical assets")
    );
    assert!(
        old_snapshot
            .concept
            .iter()
            .find(|g| g.generation_id == concept_id)
            .unwrap()
            .record(tag.tag_id)
            .unwrap()
            .semantic
            .text
            .contains("active readers")
    );
    assert_eq!(
        probe
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(query, text)| !*query && text.starts_with("Concept:"))
            .count(),
        2
    );
    let semantic = rt
        .execute_query(
            query(
                subject,
                vec![Cue::Concept(ConceptCue {
                    text: "active reader reclamation".into(),
                })],
            ),
            None,
        )
        .await
        .unwrap();
    assert!(
        semantic
            .bound
            .enabled_lanes
            .contains(&EvidenceFamily::Lexical)
    );
    assert!(
        semantic
            .bound
            .enabled_lanes
            .contains(&EvidenceFamily::Dense)
    );
    assert!(
        semantic
            .bound
            .representation
            .text
            .contains("Semantic concepts:")
    );
    assert!(
        semantic
            .bound
            .activation
            .inferred_tags
            .iter()
            .any(|activation| activation.tag == tag.tag_id)
    );
    assert!(
        semantic
            .result
            .results
            .iter()
            .any(|hit| hit.reference == revision)
    );
    check_model_catalog_activity(&rt, subject, tag.tag_id, &probe).await;
    check_historical_profiles(&rt, subject, tag.tag_id, &revision, historical_cut).await;
    Box::pin(check_activation_configuration(&rt, subject)).await;
}

#[expect(
    clippy::too_many_lines,
    reason = "one model query scenario verifies Host material reuse, catalog membership and capability policy"
)]
async fn check_model_catalog_activity(
    rt: &NousRuntime,
    subject: SubjectId,
    tag: TagId,
    probe: &Probe,
) {
    use nous_kernel::transport::KernelService;
    use nous_protocol::nous::wave::kernel::v1alpha1::{
        self as k, kernel_query_service_server::KernelQueryService,
    };
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            CONCEPT_ENRICHMENT.path(),
            serde_json::json!("model"),
            None,
        )
        .await
        .unwrap();
    let service = KernelService(rt.clone());
    let before = rt.store.authority_seq(subject).await.unwrap();
    let mut input = query(
        subject,
        vec![Cue::Text(TextCue {
            text: "reader reclamation".into(),
        })],
    );
    input.capabilities.query_concept_enrichment = RequirementStrength::Required;
    let activate = |token: String| k::KernelQueryRequest {
        subject_id: subject.0.to_string(),
        preparation_token: token,
        ..Default::default()
    };
    let bound = rt.cognition.bind_query(input.clone()).await.unwrap();
    let materials = vec![k::QueryEmbedding {
        text: bound.representation.text.clone(),
        space_hash: probe.space().space_hash,
        producer_hash: probe.producer().signature_hash,
        vector: vec![1.0, 0.0],
    }];
    let queries = probe
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(query, _)| *query)
        .count();
    let token = rt
        .cognition
        .retain_prepared_query(test_support::query_reservation(bound))
        .unwrap();
    let activation = service
        .activate_query(tonic::Request::new(k::KernelQueryRequest {
            embeddings: materials,
            ..activate(token.to_string())
        }))
        .await
        .unwrap()
        .into_inner();
    let model_input: serde_json::Value = serde_json::from_str(&activation.model_input).unwrap();
    let catalog = model_input["existing_tags"].as_array().unwrap();
    assert!(
        catalog
            .iter()
            .all(|candidate| candidate.get("tag").is_none())
    );
    let key = catalog[0]["key"].as_str().unwrap();
    let response = service.query(tonic::Request::new(k::KernelQueryRequest {
        concept_model_calls:1,
        concept_output:Some(serde_json::json!({"existing_tags":[{"key":key,"strength":1.0}],"novel_concepts":[{"text":"lease-aware historical cache"}]}).to_string()),
        ..activate(activation.preparation_token)
    })).await.unwrap().into_inner().response.unwrap();
    assert_eq!(
        probe
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(query, _)| *query)
            .count(),
        queries
    );
    rt.cognition
        .use_feedback(nous_runtime::UseFeedback {
            subject,
            session_id: None,
            consumer_ref: "consumer:host:concept".into(),
            events: vec![nous_runtime::UseFeedbackEvent {
                query_id: Some(response.query_id.parse().unwrap()),
                event_id: UseEventId::new(),
                reference: response
                    .hits
                    .first()
                    .unwrap()
                    .reference
                    .as_ref()
                    .map(|reference| parse_reference(&reference.kind, &reference.value).unwrap())
                    .unwrap(),
                use_kind: nous_runtime::UseKind::ResultSupported,
                occurred_at: rt.cognition.now(subject),
                context: serde_json::json!({}),
            }],
        })
        .await
        .unwrap();
    let linked = nous_runtime::linked_query_feedback(
        &rt.store,
        subject,
        &parse_reference(
            &response.hits[0].reference.as_ref().unwrap().kind,
            &response.hits[0].reference.as_ref().unwrap().value,
        )
        .unwrap(),
        rt.cognition.now(subject),
    )
    .await
    .unwrap();
    assert_eq!(
        linked[0].signals.novel_concepts[0].text,
        "lease-aware historical cache"
    );
    let bound: serde_json::Value =
        serde_json::from_str(response.bound_query.as_deref().unwrap()).unwrap();
    assert_eq!(bound["query_activation"]["model_calls"], 1);
    assert_eq!(
        bound["query_activation"]["novel_concepts"][0]["text"],
        "lease-aware historical cache"
    );
    assert!(
        bound["query_activation"]["inferred_tags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["source"] == "model_inferred"
                && item["tag"] == serde_json::json!(tag))
    );
    assert_eq!(rt.store.authority_seq(subject).await.unwrap(), before);
    let token = rt
        .cognition
        .retain_prepared_query(test_support::query_reservation(
            rt.cognition.bind_query(input.clone()).await.unwrap(),
        ))
        .unwrap();
    let activation = service
        .activate_query(tonic::Request::new(activate(token.to_string())))
        .await
        .unwrap()
        .into_inner();
    let error = service.query(tonic::Request::new(k::KernelQueryRequest {concept_model_calls:1,concept_output:Some(serde_json::json!({"existing_tags":[{"key":"c99","strength":1.0}],"novel_concepts":[]}).to_string()),..activate(activation.preparation_token)})).await.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert!(error.message().contains("outside frozen catalog"));
    assert!(matches!(
        rt.execute_query(input.clone(), None).await,
        Err(Error::Unavailable(_))
    ));
    input.capabilities.query_concept_enrichment = RequirementStrength::Forbidden;
    let token = rt
        .cognition
        .retain_prepared_query(test_support::query_reservation(
            rt.cognition.bind_query(input.clone()).await.unwrap(),
        ))
        .unwrap();
    assert_eq!(
        service
            .activate_query(tonic::Request::new(activate(token.to_string())))
            .await
            .unwrap_err()
            .code(),
        tonic::Code::InvalidArgument
    );
    let disabled = rt.execute_query(input, None).await.unwrap();
    assert_eq!(disabled.bound.activation.model_calls, 0);
    assert_eq!(rt.store.authority_seq(subject).await.unwrap(), before);
}

async fn check_historical_profiles(
    rt: &NousRuntime,
    subject: SubjectId,
    tag: TagId,
    revision: &CognitiveRef,
    cut: chrono::DateTime<chrono::Utc>,
) {
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            CONCEPT_ENRICHMENT.path(),
            serde_json::json!("existing"),
            None,
        )
        .await
        .unwrap();
    for profile in [
        "nous-node-potential-v1",
        "vcp-dtsc-v9.2.1-adapter-v1",
        "vcp-rivermemo-v3.1-adapter-v1",
    ] {
        rt.configuration
            .set_subject_override(
                OperationId::new(),
                subject,
                COGNITIVE_PROFILE.path(),
                serde_json::json!(profile),
                None,
            )
            .await
            .unwrap();
        let mut query = query(
            subject,
            vec![
                Cue::Text(TextCue {
                    text: "reader reclamation".into(),
                }),
                Cue::Tag(TagCue { tag }),
            ],
        );
        query.temporal_frame.authority_view = AuthorityView::AsOf(cut);
        query.exploration = ExplorationIntent::BoundedAssociative;
        let result = rt.execute_query(query, None).await.unwrap();
        assert!(
            result
                .result
                .results
                .iter()
                .any(|hit| &hit.reference == revision)
        );
        let records = rt
            .store
            .historical_serving_reusable(
                subject,
                &result
                    .bound
                    .historical_authority
                    .as_ref()
                    .unwrap()
                    .snapshot_digest,
            )
            .await
            .unwrap();
        assert!(records.iter().any(|r| r.family == "dense"));
        assert!(records.iter().any(|r| r.family == "topology"));
        let concepts = records
            .iter()
            .find(|r| r.family == "concept" && !r.space.is_empty())
            .unwrap();
        let content =
            std::fs::read(std::path::Path::new(&concepts.artifact_location).join("concept.json"))
                .unwrap();
        let generation: nous_retrieval::ConceptGeneration =
            serde_json::from_slice(&content).unwrap();
        let record = generation.record(tag).unwrap();
        assert!(record.semantic.text.contains("Wait for all active readers"));
        assert!(!record.semantic.text.contains("historical assets"));
        assert!(record.vector.is_some());
        check_historical_embedding(
            rt,
            subject,
            result.bound.historical_authority.as_deref().unwrap(),
            &generation,
            tag,
        )
        .await;
        let (_, reader) = rt
            .serving
            .prepare_query(
                &result.bound,
                &nous_runtime::QueryPlan::for_bound_query(&result.bound),
            )
            .await
            .unwrap();
        let lanes = nous_runtime::QueryActivationView::provider(reader.as_ref())
            .lanes(
                &result.bound,
                &nous_runtime::QueryPlan::for_bound_query(&result.bound),
            )
            .await
            .unwrap();
        for family in [EvidenceFamily::Dense, EvidenceFamily::TopologyWave] {
            let lane = lanes.iter().find(|lane| lane.family == family).unwrap();
            assert!(
                !matches!(lane.status, nous_runtime::LaneStatus::Unavailable),
                "{profile}: {:?}",
                lane.diagnostics
            );
        }
    }
}

async fn check_historical_embedding(
    rt: &NousRuntime,
    subject: SubjectId,
    view: &HistoricalAuthoritySnapshot,
    generation: &nous_retrieval::ConceptGeneration,
    tag: TagId,
) {
    let needs = rt
        .serving
        .embedding_needs_in_view(subject, 256, Some(view))
        .await
        .unwrap();
    assert!(
        needs
            .iter()
            .all(|need| !need.text.contains("Retain immutable historical assets"))
    );
    if let Some(need) = needs
        .iter()
        .find(|need| need.reference == CognitiveRef::Tag(tag))
    {
        assert!(
            rt.serving
                .commit_embedding(
                    subject,
                    need.reference.clone(),
                    need.text.clone(),
                    &generation.space.as_ref().unwrap().space_hash,
                    &generation.producer.as_ref().unwrap().signature_hash,
                    vec![1.0, 0.0]
                )
                .await
                .is_err()
        );
        rt.serving
            .commit_embedding_in_view(
                subject,
                need.reference.clone(),
                need.text.clone(),
                &generation.space.as_ref().unwrap().space_hash,
                &generation.producer.as_ref().unwrap().signature_hash,
                vec![1.0, 0.0],
                Some(view),
            )
            .await
            .unwrap();
    }
}

async fn check_activation_configuration(rt: &NousRuntime, subject: SubjectId) {
    for label in ["matching policy probe", "orthogonal policy probe"] {
        rt.require_memory()
            .unwrap()
            .create_tag(
                subject,
                CreateTagRequest {
                    operation_id: OperationId::new(),
                    label: label.into(),
                    description: None,
                    kind_hint: None,
                    origin: "host_explicit".into(),
                    producer: None,
                },
            )
            .await
            .unwrap();
    }
    let limit = "retrieval.query.concept_activation.max_activated_tags";
    let catalog = "retrieval.query.concept_activation.model_catalog_limit";
    let threshold = "retrieval.query.concept_activation.minimum_similarity";
    for (path, value) in [
        (limit, serde_json::json!(1)),
        (catalog, serde_json::json!(1)),
        (threshold, serde_json::json!(0.0)),
    ] {
        rt.configuration
            .set_subject_override(OperationId::new(), subject, path, value, None)
            .await
            .unwrap();
    }
    let input = query(subject, vec![]);
    let frozen = rt.cognition.bind_query(input.clone()).await.unwrap();
    for path in [limit, catalog] {
        rt.configuration
            .set_subject_override(
                OperationId::new(),
                subject,
                path,
                serde_json::json!(3),
                None,
            )
            .await
            .unwrap();
    }
    let old = rt.execute_bound_query(frozen, None).await.unwrap();
    assert_eq!(old.bound.activation.inferred_tags.len(), 1);
    assert_eq!(old.bound.activation.concept_catalog.len(), 1);
    let new = rt.execute_query(input.clone(), None).await.unwrap();
    assert_eq!(new.bound.activation.inferred_tags.len(), 3);
    assert_eq!(new.bound.activation.concept_catalog.len(), 3);
    rt.configuration
        .set_subject_override(
            OperationId::new(),
            subject,
            threshold,
            serde_json::json!(0.5),
            None,
        )
        .await
        .unwrap();
    let filtered = rt.execute_query(input, None).await.unwrap();
    assert_eq!(filtered.bound.activation.inferred_tags.len(), 2);
    assert_eq!(filtered.bound.activation.concept_catalog.len(), 3);
}
