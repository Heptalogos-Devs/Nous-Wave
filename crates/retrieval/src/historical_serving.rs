//! Historical generations share builders, durability and leases with current Serving.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
use crate::{lifecycle::OpenArtifact, query::ServingQuery, *};
use nous_runtime::{BoundQuery, QueryPlan};
impl ServingService {
    pub(crate) async fn prepare_historical_query(
        &self,
        bound: &BoundQuery,
        plan: &QueryPlan,
        view: &HistoricalAuthoritySnapshot,
    ) -> Result<(ProjectionStatus, Arc<ServingQuery>)> {
        let _reader = self.read_gate.clone().read_owned().await;
        self.store.require_subject(view.subject).await?;
        let records = self
            .store
            .historical_serving_reusable(view.subject, &view.snapshot_digest)
            .await?;
        let mut need = plan.serving_need(&bound.source_query);
        if bound.source_query.capabilities.text_embedding == RequirementStrength::Forbidden
            && plan.cognitive_profile.requirements().query_embedding
        {
            need.topology = false;
        }
        let mut snapshot = ServingSnapshot {
            view_digest: Some(view.snapshot_digest.clone()),
            generation: self.publisher.next_generation(),
            ..Default::default()
        };
        let mut status = ProjectionStatus::default();
        for (family, space) in self.requested_families(need, &records) {
            let key = if space.is_empty() {
                family.to_owned()
            } else {
                format!("{family}:{space}")
            };
            let mut reusable = None;
            for record in records
                .iter()
                .filter(|record| record.family == family && record.space == space)
            {
                if self.compatible(record, &bound.config_snapshot).await
                    && self.open_record(record).is_ok()
                {
                    reusable = Some(record.clone());
                    break;
                }
            }
            let outcome = match reusable {
                Some(record) => {
                    status.reopened.push(key.clone());
                    Ok(record)
                }
                None => self
                    .build_family_in_view(
                        view.subject,
                        family,
                        &space,
                        &bound.config_snapshot,
                        Some(view),
                        &snapshot,
                    )
                    .await
                    .inspect(|_| status.rebuilt.push(key.clone())),
            };
            match outcome {
                Ok(record) => {
                    let artifact = self.open_record(&record)?;
                    snapshot.install(record.generation_id, artifact);
                    status.generations.insert(key, record.generation_id);
                }
                Err(error) => status.degradation.push(Degradation {
                    code: format!("{family}_projection_unavailable"),
                    detail: Some(error.to_string()),
                }),
            }
        }
        Ok((status, self.query_reader(bound, snapshot)?))
    }
}
impl ServingSnapshot {
    pub(crate) fn install(&mut self, id: ServingGenerationId, artifact: OpenArtifact) {
        match artifact {
            OpenArtifact::Lexical(index) => self.lexical = Some(index),
            OpenArtifact::Dense(index, basis) => {
                self.dense
                    .retain(|old| old.space.space_hash != index.space.space_hash);
                self.epa
                    .retain(|old| old.basis.embedding_space != index.space.space_hash);
                self.dense.push(index);
                self.epa.extend(basis);
            }
            OpenArtifact::Topology(graph) => {
                self.topology = Some(graph);
                self.vcp = None;
            }
            OpenArtifact::Vcp(assets) => {
                self.vcp = Some(assets);
                self.topology = None;
            }
            OpenArtifact::Concept(generation) => {
                self.concept.retain(|old| {
                    old.space.as_ref().map(|s| &s.space_hash)
                        != generation.space.as_ref().map(|s| &s.space_hash)
                });
                self.concept.push(generation);
            }
            OpenArtifact::Exact(postings) => {
                self.postings = postings;
                self.postings_generation = Some(id);
            }
        }
    }
}
