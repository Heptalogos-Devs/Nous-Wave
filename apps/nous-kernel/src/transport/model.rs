use super::*;
use nous_core::{Result, SubjectId};
use nous_persistence::database_error as db;
use nous_retrieval::{QueryEmbedding, TextEmbeddingOutput};
use sqlx::Row;

impl KernelService {
    fn embedding_config(&self) -> Result<k::EmbeddingConfig> {
        let provider = self
            .0
            .serving
            .embedding
            .as_ref()
            .ok_or_else(|| Error::Unavailable("embedding space not configured".into()))?;
        let space = provider.space();
        Ok(k::EmbeddingConfig {
            space_hash: space.space_hash,
            producer_hash: provider.producer().signature_hash,
            model: space.model_identity,
            dimension: space.dimension,
        })
    }
    pub(super) async fn query_with_material(
        &self,
        input: k::KernelQueryRequest,
    ) -> Result<k::KernelQueryResponse> {
        if input.embeddings.len() > 64 {
            return Err(Error::Invalid("too many query embeddings".into()));
        }
        let mut materials = vec![];
        for material in input.embeddings {
            let provider = self
                .0
                .serving
                .embedding
                .as_ref()
                .ok_or_else(|| Error::Unavailable("embedding space not configured".into()))?;
            let space = provider.space();
            let producer = provider.producer();
            if material.text.len() > 32768
                || material.space_hash != space.space_hash
                || material.producer_hash != producer.signature_hash
                || material.vector.len() != space.dimension as usize
                || material.vector.iter().any(|v| !v.is_finite())
            {
                return Err(Error::Invalid(
                    "query embedding is incompatible with selected space/producer".into(),
                ));
            }
            materials.push(QueryEmbedding {
                text: material.text,
                output: TextEmbeddingOutput {
                    vector: material.vector,
                    space,
                    producer,
                },
            });
        }
        nous_retrieval::with_query_material(
            materials,
            self.query(
                required(input.query, "query")?,
                input.validated_candidate_limit.map(|limit| limit as usize),
            ),
        )
        .await
    }
}
#[tonic::async_trait]
impl k::model_material_service_server::ModelMaterialService for KernelService {
    async fn segment_description(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<k::DescriptionSegments>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let segments = self
                .0
                .material
                .segment_description(
                    SubjectId(id(&input.subject_id)?),
                    nous_core::DerivedRepresentationId(id(&input.id)?),
                )
                .await?;
            Ok(k::DescriptionSegments {
                segments: segments
                    .into_iter()
                    .map(|item| k::DescriptionSegment {
                        key: item.key,
                        text: item.text,
                        reference: Some(to_ref(nous_core::CognitiveRef::DerivedRegion(
                            item.region.derived_region_id,
                        ))),
                    })
                    .collect(),
            })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn find_workflow(
        &self,
        request: Request<k::FindWorkflowRequest>,
    ) -> std::result::Result<Response<k::FoundWorkflow>, Status> {
        let input = request.into_inner();
        let result:Result<_>=async {
            let row=sqlx::query("SELECT semantic_digest,snapshot,proposal,outcome FROM model_workflow_operations WHERE subject_id=$1 AND owner=$2 AND operation_key=$3").bind(id(&input.subject_id)?).bind(&input.owner).bind(&input.operation_key).fetch_optional(self.0.store.pool()).await.map_err(db)?;
            let Some(row)=row else{return Ok(k::FoundWorkflow{found:false,snapshot_json:None,proposal_json:None,outcome_json:None});};
            if row.try_get::<String,_>("semantic_digest").map_err(db)?!=input.semantic_digest{return Err(Error::Conflict("model operation identity has different semantic input".into()));}
            Ok(k::FoundWorkflow{found:true,snapshot_json:Some(row.try_get::<serde_json::Value,_>("snapshot").map_err(db)?.to_string()),proposal_json:row.try_get::<Option<serde_json::Value>,_>("proposal").map_err(db)?.map(|value|value.to_string()),outcome_json:row.try_get::<Option<serde_json::Value>,_>("outcome").map_err(db)?.map(|value|value.to_string())})
        }.await;
        result.map(Response::new).map_err(status)
    }
    async fn get_resolved_mentions(
        &self,
        request: Request<k::ResolvedMentionsRequest>,
    ) -> std::result::Result<Response<k::ResolvedMentionsResponse>, Status> {
        let input = request.into_inner();
        let result:Result<_>=async {
            let rows=sqlx::query("SELECT m.mention_id,m.surface,b.entity_ref FROM entity_mentions m JOIN LATERAL (SELECT entity_ref,binding_state FROM entity_binding_revisions WHERE mention_id=m.mention_id ORDER BY revision_no DESC LIMIT 1) b ON b.binding_state='bound' AND b.entity_ref IS NOT NULL WHERE m.subject_id=$1 AND m.occurrence_id=$2 ORDER BY m.mention_id LIMIT 129").bind(id(&input.subject_id)?).bind(id(&input.occurrence_id)?).fetch_all(self.0.store.pool()).await.map_err(db)?;
            if rows.len()>128{return Err(Error::Invalid("resolved mention candidate bound exceeded".into()));}
            let candidates=rows.into_iter().map(|row| Ok(k::ResolvedMention{key:row.try_get::<uuid::Uuid,_>("mention_id").map_err(db)?.to_string(),surface:row.try_get("surface").map_err(db)?,entity_ref:row.try_get("entity_ref").map_err(db)?})).collect::<Result<Vec<_>>>()?;
            Ok(k::ResolvedMentionsResponse{candidates})
        }.await;
        result.map(Response::new).map_err(status)
    }
    async fn reserve_workflow(
        &self,
        request: Request<k::ReserveWorkflowRequest>,
    ) -> std::result::Result<Response<k::WorkflowReservation>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let subject = SubjectId(id(&input.subject_id)?);
            if input.owner == "memory" {
                id(&input.operation_key)?;
                if self.0.memory.is_none() {
                    return Err(Error::Unavailable("Memory owner unavailable".into()));
                }
            }
            let snapshot = workflow_json(&input.snapshot_json)?;
            let reserved = self
                .0
                .store
                .reserve_model_workflow(
                    subject,
                    &input.owner,
                    &input.operation_key,
                    &input.semantic_digest,
                    &snapshot,
                )
                .await?;
            Ok(k::WorkflowReservation {
                snapshot_json: reserved.snapshot.to_string(),
                proposal_json: reserved.proposal.map(|value| value.to_string()),
                outcome_json: reserved.outcome.map(|value| value.to_string()),
                lease_token: reserved.lease_token.map(|value| value.to_string()),
                busy: reserved.busy,
            })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn save_workflow(
        &self,
        request: Request<k::SaveWorkflowRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let proposal = input
                .proposal_json
                .as_deref()
                .map(workflow_json)
                .transpose()?;
            let outcome = input
                .outcome_json
                .as_deref()
                .map(workflow_json)
                .transpose()?;
            self.0
                .store
                .save_model_workflow(
                    SubjectId(id(&input.subject_id)?),
                    &input.owner,
                    &input.operation_key,
                    id(&input.lease_token)?,
                    proposal.as_ref(),
                    outcome.as_ref(),
                )
                .await
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn release_workflow(
        &self,
        request: Request<k::ReleaseWorkflowRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let input = request.into_inner();
        self.0
            .store
            .release_model_workflow(
                SubjectId(id(&input.subject_id).map_err(status)?),
                &input.owner,
                &input.operation_key,
                id(&input.lease_token).map_err(status)?,
            )
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn commit_interpretation(
        &self,
        request: Request<k::CommitInterpretationRequest>,
    ) -> std::result::Result<Response<p::DerivedRepresentation>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            if input.text.trim().is_empty() || input.text.len() > 1_048_576 {
                return Err(Error::Invalid(
                    "invalid interpretation proposal bounds".into(),
                ));
            }
            let subject = SubjectId(id(&input.subject_id)?);
            let kind: nous_core::RepresentationKind = enum_value(&input.kind)?;
            let producer = required(input.producer, "producer")?;
            let producer = from_producer(producer)?;
            let inputs = input
                .inputs
                .into_iter()
                .map(|item| {
                    Ok(nous_material::DerivationInput {
                        ordinal: item.ordinal,
                        reference: from_ref(required(item.reference, "input reference")?)?,
                        role: item.role,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let representation = self
                .0
                .material
                .persist_derived_representation(nous_material::DerivedRepresentation {
                    derived_representation_id: nous_core::DerivedRepresentationId::new(),
                    subject_id: subject,
                    inputs,
                    strategy: input.strategy,
                    representation_kind: kind,
                    producer,
                    revision: 1,
                    payload_text: Some(input.text),
                    payload_json: input.structured_payload.map(|value| object(Some(value))),
                    payload_artifact_id: None,
                    quality: object(input.quality),
                    created_at: chrono::Utc::now(),
                    supersedes: input
                        .supersedes
                        .as_deref()
                        .map(id)
                        .transpose()?
                        .map(nous_core::DerivedRepresentationId),
                })
                .await?;
            self.get_derived_representation(p::ObjectRequest {
                subject_id: subject.0.to_string(),
                id: representation.derived_representation_id.0.to_string(),
            })
            .await
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn get_embedding_config(
        &self,
        _: Request<()>,
    ) -> std::result::Result<Response<k::EmbeddingConfig>, Status> {
        self.embedding_config().map(Response::new).map_err(status)
    }
    async fn list_embedding_needs(
        &self,
        request: Request<k::EmbeddingNeedsRequest>,
    ) -> std::result::Result<Response<k::EmbeddingNeedsResponse>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let config = self.embedding_config()?;
            let needs = self
                .0
                .serving
                .embedding_needs(SubjectId(id(&input.subject_id)?), input.limit as usize)
                .await?
                .into_iter()
                .map(|n| k::EmbeddingNeed {
                    reference: Some(to_ref(n.reference)),
                    text: n.text,
                    digest: n.digest,
                })
                .collect();
            Ok(k::EmbeddingNeedsResponse {
                config: Some(config),
                needs,
            })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn commit_embedding(
        &self,
        request: Request<k::CommitEmbeddingRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let material = required(input.material, "material")?;
            self.0
                .serving
                .commit_embedding(
                    SubjectId(id(&input.subject_id)?),
                    from_ref(required(input.reference, "reference")?)?,
                    material.text,
                    &material.space_hash,
                    &material.producer_hash,
                    material.vector,
                )
                .await
        }
        .await;
        result.map(Response::new).map_err(status)
    }
}

fn workflow_json(value: &str) -> Result<serde_json::Value> {
    if value.len() > nous_persistence::WORKFLOW_VALUE_MAX_BYTES {
        return Err(Error::Invalid("workflow value exceeds bound".into()));
    }
    serde_json::from_str(value).map_err(|_| Error::Invalid("workflow JSON is invalid".into()))
}
