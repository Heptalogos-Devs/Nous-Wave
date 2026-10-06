//! Host-supplied model material. This adapter never invokes a model or network provider.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use crate::*;
use nous_persistence::{DenseInvalidation, ProjectionInvalidation, database_error as db};
use std::future::Future;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEmbeddingConfig {
    pub space: EmbeddingSpaceSignature,
    pub producer: ProducerSignature,
}
#[derive(Debug, Clone)]
pub struct QueryEmbedding {
    pub text: String,
    pub output: TextEmbeddingOutput,
}
tokio::task_local! { static QUERY_MATERIAL: Vec<QueryEmbedding>; }
pub async fn with_query_material<T>(
    material: Vec<QueryEmbedding>,
    operation: impl Future<Output = T>,
) -> T {
    QUERY_MATERIAL.scope(material, operation).await
}

pub(crate) fn query_material_output(
    text: &str,
    space: &EmbeddingSpaceSignature,
    producer: &ProducerSignature,
) -> Option<TextEmbeddingOutput> {
    QUERY_MATERIAL
        .try_with(|items| {
            items
                .iter()
                .find(|item| {
                    item.text == text
                        && item.output.space.compatible_with(space)
                        && item.output.producer.signature_hash == producer.signature_hash
                })
                .map(|item| item.output.clone())
        })
        .ok()
        .flatten()
}
pub struct StoredEmbeddingProvider {
    store: AuthorityStore,
    config: StoredEmbeddingConfig,
}
impl StoredEmbeddingProvider {
    pub fn new(store: AuthorityStore, config: StoredEmbeddingConfig) -> Result<Self> {
        validate_embedding_config(&config)?;
        Ok(Self { store, config })
    }
}
pub fn validate_embedding_config(config: &StoredEmbeddingConfig) -> Result<()> {
    let mut space = config.space.clone();
    space.space_hash.clear();
    let expected = blake3::hash(
        &serde_json::to_vec(&space).map_err(|error| Error::Invalid(error.to_string()))?,
    )
    .to_hex()
    .to_string();
    if config.space.space_hash != expected
        || config.producer.signature_hash
            != AuthorityStore::canonical_producer(&config.producer)?.signature_hash
    {
        return Err(Error::Invalid(
            "resolved embedding signatures are not canonical".into(),
        ));
    }
    if config.space.dimension == 0
        || config.space.dimension > 8192
        || config.space.space_hash.is_empty()
        || config.producer.signature_hash.is_empty()
        || config.producer.model_identity.as_deref() != Some(config.space.model_identity.as_str())
        || config.producer.operation != CapabilityOperation::TextEmbedding
    {
        return Err(Error::Invalid(
            "invalid embedding space/producer configuration".into(),
        ));
    }
    Ok(())
}
#[async_trait::async_trait]
impl TextEmbeddingProvider for StoredEmbeddingProvider {
    fn space(&self) -> EmbeddingSpaceSignature {
        self.config.space.clone()
    }
    fn producer(&self) -> ProducerSignature {
        self.config.producer.clone()
    }
    async fn embed(&self, request: TextEmbeddingRequest) -> Result<TextEmbeddingOutput> {
        if request.query {
            let output =
                query_material_output(&request.text, &self.config.space, &self.config.producer);
            return output.ok_or_else(|| {
                Error::Unavailable("Host did not supply compatible query embedding".into())
            });
        }
        let vector=sqlx::query_scalar::<_,Vec<f32>>("SELECT vector FROM embedding_materials WHERE subject_id=$1 AND content_digest=$2 AND space_hash=$3 AND producer_hash=$4")
            .bind(request.subject.0).bind(blake3::hash(request.text.as_bytes()).to_hex().to_string()).bind(&self.config.space.space_hash).bind(&self.config.producer.signature_hash)
            .fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(||Error::Unavailable("Host embedding material is not ready".into()))?;
        Ok(TextEmbeddingOutput {
            vector,
            space: self.space(),
            producer: self.producer(),
        })
    }
}
#[derive(Debug, Clone)]
pub struct EmbeddingNeed {
    pub reference: CognitiveRef,
    pub text: String,
    pub digest: String,
}
impl ServingService {
    pub async fn embedding_needs(
        &self,
        subject: SubjectId,
        limit: usize,
    ) -> Result<Vec<EmbeddingNeed>> {
        self.embedding_needs_in_view(subject, limit, None).await
    }
    pub async fn embedding_needs_in_view(
        &self,
        subject: SubjectId,
        limit: usize,
        view: Option<&HistoricalAuthoritySnapshot>,
    ) -> Result<Vec<EmbeddingNeed>> {
        if limit == 0 || limit > 256 {
            return Err(Error::Invalid(
                "embedding batch limit must be 1..256".into(),
            ));
        }
        self.store.require_subject(subject).await?;
        let provider = self
            .embedding()
            .ok_or_else(|| Error::Unavailable("embedding space not configured".into()))?;
        let config = provider.space();
        let producer = provider.producer();
        let capabilities = self.projection_capabilities(subject).await?;
        let budget = self
            .configuration
            .snapshot_for_subject(subject)?
            .get(crate::EPISODE_SYNOPSIS)?;
        let sources = match view {
            Some(view) => {
                if view.subject != subject {
                    return Err(Error::Invalid(
                        "historical embedding Subject mismatch".into(),
                    ));
                };
                self.store
                    .historical_projection_input(view, budget)
                    .await?
                    .sources
            }
            None => {
                self.store
                    .text_projection_input(
                        subject,
                        "dense",
                        &config.space_hash,
                        capabilities.memory,
                        budget,
                    )
                    .await?
                    .sources
            }
        };
        let mut needs = vec![];
        for doc in self.documents(sources, budget).await? {
            let digest = blake3::hash(doc.representation_text.as_bytes())
                .to_hex()
                .to_string();
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM embedding_materials WHERE subject_id=$1 AND content_digest=$2 AND space_hash=$3 AND producer_hash=$4)")
                .bind(subject.0).bind(&digest).bind(&config.space_hash).bind(&producer.signature_hash).fetch_one(self.store.pool()).await.map_err(db)?;
            if !exists {
                needs.push(EmbeddingNeed {
                    reference: doc.reference,
                    text: doc.representation_text,
                    digest,
                });
            }
            if needs.len() >= limit {
                break;
            }
        }
        Ok(needs)
    }
    pub async fn commit_embedding(
        &self,
        subject: SubjectId,
        reference: CognitiveRef,
        text: String,
        space: &str,
        producer: &str,
        vector: Vec<f32>,
    ) -> Result<()> {
        self.commit_embedding_in_view(subject, reference, text, space, producer, vector, None)
            .await
    }
    pub async fn commit_embedding_in_view(
        &self,
        subject: SubjectId,
        reference: CognitiveRef,
        text: String,
        space: &str,
        producer: &str,
        vector: Vec<f32>,
        view: Option<&HistoricalAuthoritySnapshot>,
    ) -> Result<()> {
        self.store.validate_reference(subject, &reference).await?;
        let configured = self
            .embedding()
            .ok_or_else(|| Error::Unavailable("embedding not configured".into()))?;
        if configured.space().space_hash != space
            || configured.producer().signature_hash != producer
            || vector.len() != configured.space().dimension as usize
            || vector.iter().any(|v| !v.is_finite())
        {
            return Err(Error::Invalid(
                "embedding material disagrees with configured space/producer".into(),
            ));
        }
        let input = self.projection_capabilities(subject).await?;
        let budget = self
            .configuration
            .snapshot_for_subject(subject)?
            .get(crate::EPISODE_SYNOPSIS)?;
        let sources = match view {
            Some(view) => {
                if view.subject != subject {
                    return Err(Error::Invalid(
                        "historical embedding Subject mismatch".into(),
                    ));
                };
                self.store
                    .historical_projection_input(view, budget)
                    .await?
                    .sources
            }
            None => {
                self.store
                    .text_projection_input(subject, "dense", space, input.memory, budget)
                    .await?
                    .sources
            }
        };
        let exists = self
            .documents(sources, budget)
            .await?
            .into_iter()
            .any(|d| d.reference == reference && d.representation_text == text);
        if !exists {
            return Err(Error::Conflict("embedding source changed".into()));
        }
        let mut tx = self.store.begin().await?;
        let inserted=sqlx::query("INSERT INTO embedding_materials(subject_id,content_digest,space_hash,producer_hash,vector) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
            .bind(subject.0).bind(blake3::hash(text.as_bytes()).to_hex().to_string()).bind(space).bind(producer).bind(vector).execute(&mut *tx).await.map_err(db)?;
        if inserted.rows_affected() > 0 {
            AuthorityStore::invalidate_in(
                &mut tx,
                subject,
                ProjectionInvalidation {
                    dense: DenseInvalidation::Spaces(vec![space.into()]),
                    ..Default::default()
                },
            )
            .await?;
        }
        tx.commit().await.map_err(db)
    }
}
