use crate::topology_lane::topology_lane;
use crate::*;
use nous_core::{CognitiveRef, Cue, EvidenceFamily, RequirementStrength, Result};
use nous_runtime::{
    BoundQuery, LaneCandidate, LaneOutput, LaneStatus, QueryPlan, SharedLaneProvider,
};
use std::collections::HashMap;

fn text_query(query: &nous_core::CognitiveQuery) -> String {
    query
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

#[async_trait::async_trait]
impl SharedLaneProvider for ServingService {
    async fn lanes(&self, bound: &BoundQuery, plan: &QueryPlan) -> Result<Vec<LaneOutput>> {
        let snapshot = self.publisher.snapshot_for(bound.source_query.subject);
        let query_text = text_query(&bound.source_query);
        let mut outputs = Vec::new();

        if bound.lane_enabled(EvidenceFamily::Lexical) {
            let mut output = LaneOutput::empty(EvidenceFamily::Lexical, LaneStatus::Unavailable);
            if query_text.trim().is_empty() {
                output.status = LaneStatus::Ready;
            } else if let Some(index) = snapshot.lexical.as_ref() {
                output.status = LaneStatus::Ready;
                output.generation_ref = Some(index.generation_id);
                output.candidates = index
                    .search(&query_text, plan.lane_budget(EvidenceFamily::Lexical))?
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
            outputs.push(output);
        }

        if bound.lane_enabled(EvidenceFamily::Dense) {
            let mut output = LaneOutput::empty(EvidenceFamily::Dense, LaneStatus::Unavailable);
            if query_text.trim().is_empty()
                || bound.source_query.capabilities.text_embedding == RequirementStrength::Forbidden
            {
                output.status = LaneStatus::Ready;
            } else if let Some(provider) = self.embedding.as_ref() {
                if snapshot.dense.is_empty() {
                    output
                        .diagnostics
                        .push("dense serving generation is unavailable".into());
                } else {
                    match provider
                        .embed(TextEmbeddingRequest {
                            subject: bound.source_query.subject,
                            text: query_text,
                            query: true,
                        })
                        .await
                    {
                        Ok(embedding) => {
                            let mut best = HashMap::<CognitiveRef, (usize, Vec<String>)>::new();
                            for generation in &snapshot.dense {
                                if !embedding.space.compatible_with(&generation.space) {
                                    continue;
                                }
                                output.generation_ref = Some(generation.generation_id);
                                for (rank, item) in generation
                                    .search(
                                        &embedding.vector,
                                        plan.lane_budget(EvidenceFamily::Dense),
                                    )?
                                    .into_iter()
                                    .enumerate()
                                {
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
                                output.diagnostics.push(
                                    "no dense generation matches the bound embedding space".into(),
                                );
                            }
                        }
                        Err(error) => output.diagnostics.push(error.to_string()),
                    }
                }
            } else {
                output
                    .diagnostics
                    .push("text embedding provider is unavailable".into());
            }
            outputs.push(output);
        }

        if bound.lane_enabled(EvidenceFamily::TopologyWave) {
            outputs.push(topology_lane(&snapshot, bound, plan));
        }

        Ok(outputs)
    }
}
