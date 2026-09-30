use super::*;
use nous_core::{Result, SubjectId};
use nous_retrieval::{QueryEmbedding, TextEmbeddingOutput};

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
    ) -> Result<p::QueryResponse> {
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
        nous_retrieval::with_query_material(materials, self.query(required(input.query, "query")?))
            .await
    }
}
#[tonic::async_trait]
impl k::model_material_service_server::ModelMaterialService for KernelService {
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
            let producer = nous_core::ProducerSignature {
                signature_hash: String::new(),
                provider_class: producer.provider_class,
                operation: enum_value(&producer.operation)?,
                implementation: producer.implementation,
                model_identity: producer.model_identity,
                model_revision: producer.model_revision,
                preprocessing_identity: producer.preprocessing_identity,
                preprocessing_revision: producer.preprocessing_revision,
                config_digest: producer.config_digest,
            };
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
                    payload_artifact_id: None,
                    quality: serde_json::json!({"status":"model_interpretation"}),
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
