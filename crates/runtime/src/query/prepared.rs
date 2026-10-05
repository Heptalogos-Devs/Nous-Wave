use super::{BoundQuery, CognitiveContributors, QueryExecution};
use crate::CognitiveRuntimeService;
use nous_core::*;
use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(crate) struct PendingQuery {
    created: Instant,
    lease: Duration,
    snapshot: QuerySnapshot,
}
enum QuerySnapshot {
    Prepared(Box<BoundQuery>),
    Executed(Box<QueryExecution>),
}
impl QuerySnapshot {
    fn bound(&self) -> &BoundQuery {
        match self {
            Self::Prepared(bound) => bound,
            Self::Executed(execution) => &execution.bound,
        }
    }
}

impl CognitiveRuntimeService {
    pub fn expire_query_leases(&self) -> Result<()> {
        let mut pending = self
            .pending_queries
            .lock()
            .map_err(|_| Error::Infrastructure("query lease lock unavailable".into()))?;
        pending.retain(|_, value| value.created.elapsed() < value.lease);
        Ok(())
    }

    pub fn retain_prepared_query(&self, bound: BoundQuery) -> Result<Uuid> {
        let mut pending = self
            .pending_queries
            .lock()
            .map_err(|_| Error::Infrastructure("query lease lock unavailable".into()))?;
        pending.retain(|_, value| value.created.elapsed() < value.lease);
        if pending.len() >= bound.config_snapshot.get(crate::QUERY_SLOTS)? {
            return Err(Error::Unavailable("prepared query slots are busy".into()));
        }
        let token = Uuid::new_v4();
        pending.insert(
            token,
            PendingQuery {
                created: Instant::now(),
                lease: Duration::from_secs(bound.config_snapshot.get(crate::QUERY_LEASE)?),
                snapshot: QuerySnapshot::Prepared(Box::new(bound)),
            },
        );
        Ok(token)
    }
    pub fn take_prepared_query(&self, subject: SubjectId, token: Uuid) -> Result<BoundQuery> {
        let mut pending = self
            .pending_queries
            .lock()
            .map_err(|_| Error::Infrastructure("query lease lock unavailable".into()))?;
        let entry = pending
            .get(&token)
            .ok_or_else(|| Error::NotFound("prepared query expired or consumed".into()))?;
        if entry.snapshot.bound().source_query.subject != subject {
            return Err(Error::Invalid(
                "prepared query belongs to another Subject".into(),
            ));
        }
        if !matches!(entry.snapshot, QuerySnapshot::Prepared(_)) {
            return Err(Error::Invalid(
                "execution ticket is not a preparation token".into(),
            ));
        }
        let value = pending
            .remove(&token)
            .ok_or_else(|| Error::NotFound("prepared query disappeared".into()))?;
        if value.created.elapsed() >= value.lease {
            return Err(Error::Unavailable("prepared query expired".into()));
        }
        match value.snapshot {
            QuerySnapshot::Prepared(bound) => Ok(*bound),
            QuerySnapshot::Executed(_) => Err(Error::Invalid("not a preparation token".into())),
        }
    }

    pub fn retain_query(
        &self,
        execution: QueryExecution,
    ) -> Result<(CognitiveQueryResult, Option<Uuid>)> {
        if execution.result.results.len() > 64 {
            return Err(Error::Invalid("rerank pool exceeds 64 candidates".into()));
        }
        if execution
            .result
            .results
            .iter()
            .map(|hit| hit.representation.as_ref().map_or(0, String::len))
            .sum::<usize>()
            > 2 * 1024 * 1024
        {
            return Ok(retention_fallback(
                execution,
                "validated pool exceeds the text retention bound",
            ));
        }
        let mut pending = self
            .pending_queries
            .lock()
            .map_err(|_| Error::Infrastructure("query lease lock unavailable".into()))?;
        pending.retain(|_, value| value.created.elapsed() < value.lease);
        if pending.len() >= execution.bound.config_snapshot.get(crate::QUERY_SLOTS)? {
            return Ok(retention_fallback(
                execution,
                "bounded query lease slots are busy",
            ));
        }
        let ticket = Uuid::new_v4();
        let result = execution.result.clone();
        pending.insert(
            ticket,
            PendingQuery {
                created: Instant::now(),
                lease: Duration::from_secs(
                    execution.bound.config_snapshot.get(crate::QUERY_LEASE)?,
                ),
                snapshot: QuerySnapshot::Executed(Box::new(execution)),
            },
        );
        Ok((result, Some(ticket)))
    }
    fn take_query(&self, subject: SubjectId, ticket: Uuid) -> Result<QueryExecution> {
        let mut pending = self
            .pending_queries
            .lock()
            .map_err(|_| Error::Infrastructure("query lease lock unavailable".into()))?;
        let Entry::Occupied(entry) = pending.entry(ticket) else {
            return Err(Error::NotFound("query lease expired or consumed".into()));
        };
        if entry.get().snapshot.bound().source_query.subject != subject {
            return Err(Error::Invalid(
                "query lease belongs to a different Subject".into(),
            ));
        }
        if !matches!(entry.get().snapshot, QuerySnapshot::Executed(_)) {
            return Err(Error::Invalid("query ticket has not executed".into()));
        }
        let value = entry.remove();
        if value.created.elapsed() >= value.lease {
            return Err(Error::Unavailable("query lease expired".into()));
        }
        match value.snapshot {
            QuerySnapshot::Executed(execution) => Ok(*execution),
            QuerySnapshot::Prepared(_) => {
                Err(Error::Invalid("query ticket has not executed".into()))
            }
        }
    }
    pub fn release_query(&self, subject: SubjectId, ticket: Uuid) -> Result<()> {
        let mut pending = self
            .pending_queries
            .lock()
            .map_err(|_| Error::Infrastructure("query lease lock unavailable".into()))?;
        if pending
            .get(&ticket)
            .is_some_and(|value| value.snapshot.bound().source_query.subject != subject)
        {
            return Err(Error::Invalid(
                "query lease belongs to a different Subject".into(),
            ));
        }
        pending.remove(&ticket);
        Ok(())
    }
    pub async fn finalize_query(
        &self,
        subject: SubjectId,
        ticket: Uuid,
        order: Vec<(CognitiveRef, f64)>,
        external_results: Vec<ExternalResourceResult>,
        contributors: CognitiveContributors<'_>,
    ) -> Result<CognitiveQueryResult> {
        let execution = self.take_query(subject, ticket)?;
        if order.len() > 64 || order.iter().any(|(_, score)| !score.is_finite()) {
            return Err(Error::Invalid(
                "rerank mapping exceeds bounds or has a nonfinite score".into(),
            ));
        }
        let pool: HashSet<_> = execution
            .result
            .results
            .iter()
            .map(|hit| hit.reference.clone())
            .collect();
        let requested: HashSet<_> = order
            .iter()
            .map(|(reference, _)| reference.clone())
            .collect();
        if requested.len() != order.len() || !requested.is_subset(&pool) {
            return Err(Error::Invalid(
                "rerank mapping contains duplicate or unknown candidates".into(),
            ));
        }
        let leaf_count = execution
            .bound
            .source_query
            .scopes()
            .iter()
            .filter(|scope| scope.operation == QueryOperation::Atom)
            .count();
        let mut outputs = vec![Vec::new(); leaf_count];
        for leaf in &execution.leaves {
            let memory_refs: Vec<_> = leaf
                .hits
                .iter()
                .filter(|hit| {
                    contributors
                        .memory
                        .is_some_and(|owner| owner.owns(&hit.reference))
                })
                .map(|hit| hit.reference.clone())
                .collect();
            let fresh: HashMap<_, _> = if let Some(owner) = contributors.memory {
                owner
                    .validate_and_materialize(subject, &memory_refs, &leaf.bound)
                    .await?
                    .0
                    .into_iter()
                    .map(|hit| (hit.reference.clone(), hit))
                    .collect()
            } else {
                HashMap::new()
            };
            for hit in &leaf.hits {
                if !pool.contains(&hit.reference) {
                    continue;
                }
                let valid = if contributors
                    .memory
                    .is_some_and(|owner| owner.owns(&hit.reference))
                {
                    same_stamp(fresh.get(&hit.reference), hit)
                } else {
                    self.generic_still_valid(subject, &leaf.bound, &hit.reference)
                        .await?
                };
                if !valid {
                    continue;
                }
                let Some(original) = execution
                    .result
                    .results
                    .iter()
                    .find(|original| original.reference == hit.reference)
                else {
                    continue;
                };
                outputs[leaf.ordinal].push(original.clone());
            }
        }
        let valid = super::tree::combine(
            &execution.bound.source_query.expression,
            &mut outputs.into_iter(),
        );
        let mut result = execution.result;
        let before = result.results.len();
        result
            .results
            .retain(|hit| valid.contains_key(&hit.reference));
        let dropped = before - result.results.len();
        if dropped > 0 {
            result.status = QueryStatus::Partial;
            result.degradation.push(Degradation {
                code: "authority_changed_during_rerank".into(),
                detail: Some(format!(
                    "discarded {dropped} candidates along the original BoundQuery"
                )),
            });
            if let Some(diagnostics) = &mut result.diagnostics {
                diagnostics
                    .candidate_counts
                    .insert("drop_authority_changed_during_rerank".into(), dropped);
            }
        }
        if !order.is_empty() {
            apply_model_order(
                &mut result.results,
                order,
                execution.bound.retrieval_policy.rrf_k,
            );
        }
        result
            .results
            .truncate(execution.bound.source_query.result_need.limit);
        for (rank, hit) in result.results.iter_mut().enumerate() {
            hit.match_evidence.final_rank = (rank + 1) as u32;
        }
        self.finalize_resources(subject, &mut result, external_results)
            .await?;
        Ok(result)
    }
    async fn generic_still_valid(
        &self,
        subject: SubjectId,
        bound: &super::BoundQuery,
        reference: &CognitiveRef,
    ) -> Result<bool> {
        if let Some(binding) = bound
            .exact_bindings
            .iter()
            .find(|binding| binding.bound_ref == *reference)
            && binding.mutable_object
        {
            match self
                .store
                .bind_exact_reference(subject, &binding.requested_ref)
                .await
            {
                Ok((current, epoch, _))
                    if current == binding.bound_ref && epoch == binding.bound_object_epoch => {}
                Ok(_) | Err(Error::NotFound(_)) | Err(Error::Invalid(_)) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        match self.store.validate_reference(subject, reference).await {
            Ok(()) => Ok(true),
            Err(Error::NotFound(_)) | Err(Error::Invalid(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

fn same_stamp(current: Option<&CognitiveHit>, old: &super::types::CandidateStamp) -> bool {
    current.is_some_and(|current| {
        current.authority_epoch == old.authority_epoch && current.revision == old.revision
    })
}

fn apply_model_order(hits: &mut Vec<CognitiveHit>, mut order: Vec<(CognitiveRef, f64)>, k: f64) {
    if order.is_empty() {
        return;
    }
    order.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut remaining: HashMap<_, _> = hits
        .drain(..)
        .map(|hit| (hit.reference.clone(), hit))
        .collect();
    for (reference, score) in order {
        if let Some(mut hit) = remaining.remove(&reference) {
            hit.match_evidence.rerank_score = Some(score);
            hit.match_evidence
                .families
                .push(EvidenceFamily::LanguageRerank);
            hits.push(hit);
        }
    }
    let mut tail: Vec<_> = remaining.into_values().collect();
    tail.sort_by_key(|hit| hit.match_evidence.final_rank);
    hits.extend(tail);
    for (rank, hit) in hits.iter_mut().enumerate() {
        hit.match_evidence.final_score = (k + 1.0) / (k + (rank + 1) as f64);
    }
}

fn retention_fallback(
    mut execution: QueryExecution,
    reason: &str,
) -> (CognitiveQueryResult, Option<Uuid>) {
    execution
        .result
        .results
        .truncate(execution.bound.source_query.result_need.limit);
    execution.result.degradation.push(Degradation {
        code: "query_rerank_pool_unavailable".into(),
        detail: Some(reason.into()),
    });
    if execution.result.status == QueryStatus::Complete {
        execution.result.status = QueryStatus::Degraded;
    }
    (execution.result, None)
}
