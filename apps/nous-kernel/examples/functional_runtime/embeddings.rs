use super::*;
use nous_retrieval::{QueryEmbedding, StoredEmbeddingConfig, TextEmbeddingOutput};
#[derive(Clone, Deserialize)]
pub struct VectorEntry {
    pub text: String,
    pub vector: Vec<f32>,
}
#[derive(Clone, Deserialize)]
pub struct VectorCache {
    pub config: StoredEmbeddingConfig,
    #[serde(default)]
    pub vectors: Vec<VectorEntry>,
}
pub fn load(path: &Path) -> Result<VectorCache> {
    let path = std::path::absolute(path).map_err(failure)?;
    if !path.starts_with(std::path::absolute("data/research").map_err(failure)?)
        || std::fs::metadata(&path).map_err(failure)?.len() > 32 * 1024 * 1024
    {
        return Err(Error::Invalid(
            "bounded research cache path required".into(),
        ));
    }
    let cache: VectorCache =
        serde_json::from_slice(&std::fs::read(path).map_err(failure)?).map_err(failure)?;
    nous_retrieval::validate_embedding_config(&cache.config)?;
    if cache.vectors.len() > 256
        || cache.vectors.iter().any(|entry| {
            entry.vector.len() != cache.config.space.dimension as usize
                || entry.vector.iter().any(|value| !value.is_finite())
        })
    {
        return Err(Error::Invalid("embedding cache shape exceeded".into()));
    }
    Ok(cache)
}
pub async fn needs(runtime: &NousRuntime, subjects: &[SubjectId]) -> Result<Value> {
    if subjects.len() > 4 {
        return Err(Error::Invalid("embedding Subject envelope exceeded".into()));
    }
    let mut sources = Vec::new();
    for subject in subjects {
        for need in runtime.serving.embedding_needs(*subject, 128).await? {
            sources.push(json!({"subject":subject,"reference":need.reference,"text":need.text}));
        }
    }
    Ok(json!({"sources":sources}))
}
pub async fn install(
    runtime: &NousRuntime,
    subjects: &[SubjectId],
    cache: &VectorCache,
) -> Result<Value> {
    if subjects.len() > 4 {
        return Err(Error::Invalid("embedding Subject envelope exceeded".into()));
    }
    let provider = runtime
        .serving
        .embedding()
        .ok_or_else(|| Error::Unavailable("stored embedding identity missing".into()))?;
    if !cache.config.space.compatible_with(&provider.space())
        || cache.config.producer.signature_hash != provider.producer().signature_hash
    {
        return Err(Error::Conflict("embedding cache identity changed".into()));
    }
    let vectors = cache
        .vectors
        .iter()
        .map(|entry| (entry.text.as_str(), &entry.vector))
        .collect::<HashMap<_, _>>();
    let mut count = 0;
    for subject in subjects {
        let needs = runtime.serving.embedding_needs(*subject, 128).await?;
        for need in needs {
            let vector = vectors
                .get(need.text.as_str())
                .ok_or_else(|| Error::Unavailable("source embedding cache incomplete".into()))?;
            runtime
                .serving
                .commit_embedding(
                    *subject,
                    need.reference,
                    need.text,
                    &cache.config.space.space_hash,
                    &cache.config.producer.signature_hash,
                    (*vector).clone(),
                )
                .await?;
            count += 1;
        }
    }
    Ok(json!({"committedReferences":count}))
}
pub fn query_material(cache: &VectorCache, text: &str) -> Result<Vec<QueryEmbedding>> {
    let entry = cache
        .vectors
        .iter()
        .find(|entry| entry.text == text)
        .ok_or_else(|| {
            Error::Unavailable("frozen query representation is absent from embedding cache".into())
        })?;
    Ok(vec![QueryEmbedding {
        text: text.into(),
        output: TextEmbeddingOutput {
            vector: entry.vector.clone(),
            space: cache.config.space.clone(),
            producer: cache.config.producer.clone(),
        },
    }])
}
