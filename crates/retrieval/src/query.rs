use crate::topology_lane::topology_lane;
use crate::*;
use nous_core::{CognitiveRef, Cue, EvidenceFamily, RequirementStrength, Result};
use nous_runtime::{
    BoundQuery, LaneCandidate, LaneOutput, LaneStatus, QueryPlan, SharedLaneProvider,
};
use std::collections::HashMap;

fn text_query(query: &nous_core::CognitiveQuery) -> String {
    query
        .expression
        .cues
        .iter()
        .filter_map(|cue| match cue {
            Cue::Text(value) => Some(value.text.as_str()),
            Cue::Example(value) => Some(value.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Signals prepared once for one bound leaf. Consumers only borrow this value;
/// consuming it for Runtime lanes happens after cognitive readout.
pub struct PreparedQuerySignals {
    query_text: String,
    embedding: Option<TextEmbeddingOutput>,
    lexical: Option<LaneOutput>,
    dense: Option<LaneOutput>,
}

impl PreparedQuerySignals {
    pub fn query_text(&self) -> &str {
        &self.query_text
    }
    pub fn embedding(&self) -> Option<&TextEmbeddingOutput> {
        self.embedding.as_ref()
    }
    pub fn lexical(&self) -> Option<&LaneOutput> {
        self.lexical.as_ref()
    }
    pub fn dense(&self) -> Option<&LaneOutput> {
        self.dense.as_ref()
    }
    fn into_lanes(self) -> Vec<LaneOutput> {
        self.lexical.into_iter().chain(self.dense).collect()
    }
}

fn prepare_lexical(
    snapshot: &ServingSnapshot,
    query: &nous_core::CognitiveQuery,
    query_text: &str,
    plan: &QueryPlan,
) -> Result<LaneOutput> {
    let lexical = {
        let mut output = LaneOutput::empty(EvidenceFamily::Lexical, LaneStatus::Unavailable);
        if query_text.trim().is_empty() {
            output.status = LaneStatus::Ready;
        } else if let Some(index) = snapshot.lexical.as_ref() {
            output.status = LaneStatus::Ready;
            output.generation_ref = Some(index.generation_id);
            output.candidates = index
                .search_with_domains(
                    query_text,
                    plan.lane_budget(EvidenceFamily::Lexical),
                    &query.expression.domain_names(),
                )?
                .into_iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    item.reference.map(|reference| LaneCandidate {
                        reference,
                        rank: (index + 1) as u32,
                        variants: vec!["lexical:generation".into()],
                        provider_metadata: serde_json::Value::Null,
                    })
                })
                .collect();
        } else {
            output
                .diagnostics
                .push("lexical serving generation is unavailable".into());
        }
        output
    };

    Ok(lexical)
}

fn prepare_dense(
    snapshot: &ServingSnapshot,
    query: &nous_core::CognitiveQuery,
    query_text: &str,
    plan: &QueryPlan,
    embedding: Option<&TextEmbeddingOutput>,
    embedding_error: Option<String>,
    provider_available: bool,
) -> Result<LaneOutput> {
    let mut output = LaneOutput::empty(EvidenceFamily::Dense, LaneStatus::Unavailable);
    if query_text.trim().is_empty()
        || query.capabilities.text_embedding == RequirementStrength::Forbidden
    {
        output.status = LaneStatus::Ready;
    } else if let Some(embedding) = embedding {
        let mut best = HashMap::<CognitiveRef, (usize, Vec<String>)>::new();
        for generation in &snapshot.dense {
            if !embedding.space.compatible_with(&generation.space) {
                continue;
            }
            output.generation_ref = Some(generation.generation_id);
            let limit = plan.lane_budget(EvidenceFamily::Dense);
            let matches =
                domain_dense_matches(generation, &embedding.vector, limit, &query.expression)?;
            for (rank, item) in matches.into_iter().enumerate() {
                let Some(record) = item.record else {
                    continue;
                };
                let entry = best
                    .entry(record.reference)
                    .or_insert((rank + 1, Vec::new()));
                entry.0 = entry.0.min(rank + 1);
                entry
                    .1
                    .push(format!("dense:{}", generation.space.space_hash));
            }
        }
        if output.generation_ref.is_some() {
            output.status = LaneStatus::Ready;
            let mut values = best.into_iter().collect::<Vec<_>>();
            values.sort_by(|left, right| {
                left.1
                    .0
                    .cmp(&right.1.0)
                    .then_with(|| left.0.to_string().cmp(&right.0.to_string()))
            });
            output.candidates = values
                .into_iter()
                .take(plan.lane_budget(EvidenceFamily::Dense))
                .map(|(reference, (rank, variants))| LaneCandidate {
                    reference,
                    rank: rank as u32,
                    variants,
                    provider_metadata: serde_json::Value::Null,
                })
                .collect();
        } else {
            output
                .diagnostics
                .push("no dense generation matches the bound embedding space".into());
        }
    } else {
        output.diagnostics.push(embedding_error.unwrap_or_else(|| {
            if !provider_available {
                "text embedding provider is unavailable".into()
            } else {
                "dense serving generation is unavailable".into()
            }
        }));
    }
    Ok(output)
}

async fn prepare_signals(
    snapshot: &ServingSnapshot,
    query: &nous_core::CognitiveQuery,
    enabled_lanes: &[EvidenceFamily],
    plan: &QueryPlan,
    provider: Option<&dyn TextEmbeddingProvider>,
) -> Result<PreparedQuerySignals> {
    let query_text = text_query(query);
    let lexical = enabled_lanes
        .contains(&EvidenceFamily::Lexical)
        .then(|| prepare_lexical(snapshot, query, &query_text, plan))
        .transpose()?;

    let dense_enabled = enabled_lanes.contains(&EvidenceFamily::Dense);
    let cognitive_embedding =
        plan.expand_topology && plan.cognitive_profile.requirements().query_embedding;

    let mut embedding = None;
    let mut embedding_error = None;
    // Generation absence preserves the existing unavailable-lane semantics.
    // FORBIDDEN is checked before reaching the sole provider invocation.
    if (dense_enabled || cognitive_embedding)
        && !query_text.trim().is_empty()
        && query.capabilities.text_embedding != RequirementStrength::Forbidden
        && (!snapshot.dense.is_empty()
            || (cognitive_embedding
                && snapshot.vcp.as_ref().is_some_and(|generation| {
                    generation.cognitive_profile == plan.cognitive_profile
                })))
    {
        if let Some(provider) = provider {
            match provider
                .embed(TextEmbeddingRequest {
                    subject: query.subject,
                    text: query_text.clone(),
                    query: true,
                })
                .await
            {
                Ok(value) => embedding = Some(value),
                Err(error) => embedding_error = Some(error.to_string()),
            }
        } else {
            embedding_error = Some("text embedding provider is unavailable".into());
        }
    }
    let dense = dense_enabled
        .then(|| {
            prepare_dense(
                snapshot,
                query,
                &query_text,
                plan,
                embedding.as_ref(),
                embedding_error,
                provider.is_some(),
            )
        })
        .transpose()?;

    Ok(PreparedQuerySignals {
        query_text,
        embedding,
        lexical,
        dense,
    })
}

#[async_trait::async_trait]
impl SharedLaneProvider for ServingService {
    async fn lanes(&self, bound: &BoundQuery, plan: &QueryPlan) -> Result<Vec<LaneOutput>> {
        let snapshot = self.publisher.snapshot_for(bound.source_query.subject);
        let signals = prepare_signals(
            &snapshot,
            &bound.source_query,
            &bound.enabled_lanes,
            plan,
            self.embedding().map(|provider| provider.as_ref()),
        )
        .await?;
        let topology = bound
            .lane_enabled(EvidenceFamily::TopologyWave)
            .then(|| topology_lane(&snapshot, bound, plan, &signals))
            .transpose()?;
        let mut outputs = signals.into_lanes();
        outputs.extend(topology);
        Ok(outputs)
    }
}

fn domain_dense_matches(
    generation: &DenseGeneration,
    vector: &[f32],
    limit: usize,
    expression: &nous_core::CognitiveQueryExpr,
) -> Result<Vec<DenseMatch>> {
    if expression.domain_names().is_empty() {
        return generation.search(vector, limit);
    }
    let allowed = generation
        .records()
        .filter(|record| expression.allows_reference(&record.reference))
        .filter_map(|record| u32::try_from(record.serving_doc_id).ok())
        .collect::<roaring::RoaringBitmap>();
    generation.search_filtered(vector, limit, &allowed)
}

#[cfg(test)]
#[path = "query_signals_tests.rs"]
mod tests;
