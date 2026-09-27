//! Self Authority persistence and fixed-owner query contribution.
mod lifecycle;
mod query;
mod support;
use query::hit;
use support::*;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use nous_authority_store::{AuthorityStore, database_error as db};
use nous_core::{
    CognitiveQueryResult, CognitiveRef, QueryStatus, Result, SubjectId, TemporalExtent,
    canonical_request_digest,
};
use nous_self_domain::{
    CreateNarrativeIdentity, CreateSelfFacet, NarrativeIdentity, NarrativeIdentityRevision,
    ReviseNarrativeIdentity, ReviseSelfFacet, SelfFacet, SelfFacetKind, SelfFacetRevision,
    validate_facet_create, validate_facet_revision, validate_narrative_create,
    validate_narrative_revision,
};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct SelfService {
    pub store: AuthorityStore,
    pub serving: Option<nous_serving::ServingService>,
}
#[derive(Debug, Clone)]
pub struct SelfFacetView {
    pub object: SelfFacet,
    pub revision: SelfFacetRevision,
}
#[derive(Debug, Clone)]
pub struct NarrativeIdentityView {
    pub object: NarrativeIdentity,
    pub revision: NarrativeIdentityRevision,
}

fn empty_result(query_id: Uuid) -> CognitiveQueryResult {
    CognitiveQueryResult {
        query_id,
        generation: Default::default(),
        status: QueryStatus::Complete,
        results: Vec::new(),
        resource_actions: Vec::new(),
        degradation: Vec::new(),
        diagnostics: None,
    }
}

impl SelfService {
    pub async fn create_narrative(
        &self,
        input: CreateNarrativeIdentity,
    ) -> Result<NarrativeIdentityView> {
        validate_narrative_create(&input)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("narrative_identity.create", input.subject, &input)?;
        let object_id = nous_core::NarrativeIdentityId::new();
        let revision_id = nous_core::NarrativeIdentityRevisionId::new();
        let now = Utc::now();
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity.create",
            &digest,
        )
        .await?
            && receipt.state == "committed"
        {
            let id = Uuid::parse_str(&receipt.result_ref.ok_or_else(|| {
                nous_core::Error::Infrastructure("Narrative receipt missing result".into())
            })?)
            .map_err(|_| nous_core::Error::Infrastructure("invalid Narrative receipt".into()))?;
            tx.commit().await.map_err(db)?;
            return self
                .narrative(input.subject, nous_core::NarrativeIdentityId(id))
                .await;
        }
        sqlx::query("SELECT subject_id FROM subjects WHERE subject_id=$1 FOR UPDATE")
            .bind(input.subject.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM narrative_identities WHERE subject_id=$1 AND key=$2)",
        )
        .bind(input.subject.0)
        .bind(&input.key)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?
        {
            return Err(nous_core::Error::Conflict(
                "Narrative Identity key already exists".into(),
            ));
        }
        validate_supports(&mut tx, input.subject, &input.supports).await?;
        validate_narrative_targets(&mut tx, input.subject, &input.references).await?;
        let (kind, start, end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO narrative_identities(narrative_identity_id,subject_id,key,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,$4,1,'accepted','valid','normal','normal',$5)").bind(object_id.0).bind(input.subject.0).bind(&input.key).bind(revision_id.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO narrative_identity_revisions(narrative_identity_revision_id,narrative_identity_id,subject_id,revision_no,parent_revision_id,revision_intent,text,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,1,NULL,NULL,$4,$5,$6,$7,$8,$9,$10)").bind(revision_id.0).bind(object_id.0).bind(input.subject.0).bind(&input.text).bind(kind).bind(start).bind(end).bind(input.formed_at).bind(now).bind(input.producer_signature_id).execute(&mut *tx).await.map_err(db)?;
        insert_supports(
            &mut tx,
            "narrative_identity_revision_supports",
            "narrative_identity_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        insert_narrative_references(&mut tx, revision_id.0, &input.references).await?;
        invalidate_projections(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity",
            &object_id.0.to_string(),
            revision_id.0,
            1,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.narrative(input.subject, object_id).await
    }

    pub async fn revise_narrative(
        &self,
        input: ReviseNarrativeIdentity,
    ) -> Result<NarrativeIdentityView> {
        validate_narrative_revision(&input)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("narrative_identity.revise", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity.revise",
            &digest,
        )
        .await?
            && receipt.state == "committed"
        {
            tx.commit().await.map_err(db)?;
            return self
                .narrative(input.subject, input.narrative_identity_id)
                .await;
        }
        let row=sqlx::query("SELECT current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state FROM narrative_identities WHERE subject_id=$1 AND narrative_identity_id=$2 FOR UPDATE").bind(input.subject.0).bind(input.narrative_identity_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||nous_core::Error::NotFound("Narrative Identity not found".into()))?;
        let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(nous_core::Error::Conflict(
                "Narrative Identity object_epoch is stale".into(),
            ));
        }
        if current != input.parent_revision_id.0 {
            return Err(nous_core::Error::Conflict(
                "Narrative Identity parent revision is not current".into(),
            ));
        }
        require_active_lifecycle(&row)?;
        validate_supports(&mut tx, input.subject, &input.supports).await?;
        validate_narrative_targets(&mut tx, input.subject, &input.references).await?;
        let revision_id = nous_core::NarrativeIdentityRevisionId::new();
        let next:i32=sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM narrative_identity_revisions WHERE narrative_identity_id=$1").bind(input.narrative_identity_id.0).fetch_one(&mut *tx).await.map_err(db)?;
        let now = Utc::now();
        let (kind, start, end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO narrative_identity_revisions(narrative_identity_revision_id,narrative_identity_id,subject_id,revision_no,parent_revision_id,revision_intent,text,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)").bind(revision_id.0).bind(input.narrative_identity_id.0).bind(input.subject.0).bind(next).bind(input.parent_revision_id.0).bind(enum_name(input.revision_intent)).bind(&input.text).bind(kind).bind(start).bind(end).bind(input.formed_at).bind(now).bind(input.producer_signature_id).execute(&mut *tx).await.map_err(db)?;
        insert_supports(
            &mut tx,
            "narrative_identity_revision_supports",
            "narrative_identity_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        insert_narrative_references(&mut tx, revision_id.0, &input.references).await?;
        sqlx::query("UPDATE narrative_identities SET current_revision_id=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND narrative_identity_id=$2").bind(input.subject.0).bind(input.narrative_identity_id.0).bind(revision_id.0).execute(&mut *tx).await.map_err(db)?;
        invalidate_projections(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity",
            &input.narrative_identity_id.0.to_string(),
            revision_id.0,
            epoch + 1,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.narrative(input.subject, input.narrative_identity_id)
            .await
    }
}

fn self_revision_reference(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::SelfFacetRevision(_) | CognitiveRef::NarrativeIdentityRevision(_)
    )
}

fn text_query(query: &nous_core::CognitiveQuery) -> String {
    query
        .cues
        .iter()
        .filter_map(|cue| match cue {
            nous_core::Cue::Text(value) => Some(value.text.as_str()),
            nous_core::Cue::Example(value) => Some(value.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

type ServingCandidateLanes = HashMap<CognitiveRef, Vec<(nous_core::EvidenceFamily, usize, String)>>;

fn lexical_candidates(
    snapshot: &nous_cognitive_retrieval::ServingSnapshot,
    query: &str,
    enabled: bool,
    limit: usize,
) -> Result<ServingCandidateLanes> {
    let mut candidates = ServingCandidateLanes::new();
    let Some(index) = snapshot.lexical.as_ref() else {
        return Ok(candidates);
    };
    if !enabled {
        return Ok(candidates);
    }
    for (rank, item) in index.search(query, limit)?.into_iter().enumerate() {
        let Some(reference) = item.reference else {
            continue;
        };
        if self_revision_reference(&reference) {
            candidates.entry(reference).or_default().push((
                nous_core::EvidenceFamily::Lexical,
                rank + 1,
                "lexical:generation".into(),
            ));
        }
    }
    Ok(candidates)
}

async fn dense_candidates(
    serving: &nous_serving::ServingService,
    snapshot: &nous_cognitive_retrieval::ServingSnapshot,
    subject: SubjectId,
    query: &str,
    enabled: bool,
    embedding_allowed: bool,
    limit: usize,
) -> Result<ServingCandidateLanes> {
    let mut candidates = ServingCandidateLanes::new();
    if !enabled || !embedding_allowed || snapshot.dense.is_empty() {
        return Ok(candidates);
    }
    let Some(provider) = serving.embedding.as_ref() else {
        return Ok(candidates);
    };
    let Ok(embedding) = provider
        .embed(nous_serving::TextEmbeddingRequest {
            subject,
            text: query.to_owned(),
            query: true,
        })
        .await
    else {
        return Ok(candidates);
    };
    for generation in &snapshot.dense {
        if !embedding.space.compatible_with(&generation.space) {
            continue;
        }
        for (rank, item) in generation
            .search(&embedding.vector, limit)?
            .into_iter()
            .enumerate()
        {
            let Some(record) = item.record else {
                continue;
            };
            if self_revision_reference(&record.reference) {
                candidates.entry(record.reference).or_default().push((
                    nous_core::EvidenceFamily::Dense,
                    rank + 1,
                    format!("dense:{}", generation.space.space_hash),
                ));
            }
        }
    }
    Ok(candidates)
}

impl SelfService {
    async fn serving_hits(
        &self,
        bound: &nous_cognitive_runtime::BoundQuery,
        plan: &nous_cognitive_runtime::QueryPlan,
    ) -> Result<Vec<nous_core::CognitiveHit>> {
        let Some(serving) = &self.serving else {
            return Ok(Vec::new());
        };
        let query_text = text_query(&bound.source_query);
        if query_text.trim().is_empty() {
            return Ok(Vec::new());
        }
        let snapshot = serving.publisher.snapshot_for(bound.source_query.subject);
        let mut candidates = lexical_candidates(
            &snapshot,
            &query_text,
            bound.lane_enabled(nous_core::EvidenceFamily::Lexical),
            plan.lane_budget(nous_core::EvidenceFamily::Lexical),
        )?;
        for (reference, lanes) in dense_candidates(
            serving,
            &snapshot,
            bound.source_query.subject,
            &query_text,
            bound.lane_enabled(nous_core::EvidenceFamily::Dense),
            bound.source_query.capabilities.text_embedding
                != nous_core::RequirementStrength::Forbidden,
            plan.lane_budget(nous_core::EvidenceFamily::Dense),
        )
        .await?
        {
            candidates.entry(reference).or_default().extend(lanes);
        }
        let mut results = Vec::new();
        for (reference, lanes) in candidates {
            let Ok((text, valid, formed, recorded)) = self
                .context_text(bound.source_query.subject, &reference)
                .await
            else {
                continue;
            };
            let mut combined: Option<nous_core::CognitiveHit> = None;
            for (family, rank, variant) in lanes {
                let candidate = hit(
                    reference.clone(),
                    if matches!(reference, CognitiveRef::NarrativeIdentityRevision(_)) {
                        "self:narrative".into()
                    } else {
                        "self".into()
                    },
                    text.clone(),
                    rank,
                    valid.clone(),
                    formed,
                    recorded,
                    family,
                    &variant,
                    &bound.enabled_lanes,
                );
                if let Some(existing) = &mut combined {
                    existing.match_evidence.final_score += candidate.match_evidence.final_score;
                    existing.match_evidence.base_rank_score +=
                        candidate.match_evidence.base_rank_score;
                    existing.match_evidence.best_lane_rank = existing
                        .match_evidence
                        .best_lane_rank
                        .min(candidate.match_evidence.best_lane_rank);
                    existing
                        .match_evidence
                        .families
                        .extend(candidate.match_evidence.families);
                    existing
                        .match_evidence
                        .variants
                        .extend(candidate.match_evidence.variants);
                } else {
                    combined = Some(candidate);
                }
            }
            if let Some(mut candidate) = combined {
                candidate.match_evidence.families.sort();
                candidate.match_evidence.families.dedup();
                candidate.match_evidence.variants.sort();
                candidate.match_evidence.variants.dedup();
                results.push(candidate);
            }
        }
        results.sort_by(|left, right| {
            right
                .match_evidence
                .final_score
                .total_cmp(&left.match_evidence.final_score)
                .then_with(|| left.reference.to_string().cmp(&right.reference.to_string()))
        });
        Ok(results)
    }
}

#[async_trait]
impl nous_cognitive_runtime::CognitiveContributor for SelfService {
    #[expect(
        clippy::too_many_lines,
        reason = "SelfDirect owns one bounded batch read and deterministic materialization"
    )]
    async fn contribute(
        &self,
        bound: &nous_cognitive_runtime::BoundQuery,
        plan: &nous_cognitive_runtime::QueryPlan,
    ) -> Result<CognitiveQueryResult> {
        let query = &bound.source_query;
        if !plan
            .enabled_lanes
            .contains(&nous_core::EvidenceFamily::SelfDirect)
        {
            return Ok(empty_result(bound.query_id));
        }
        let cue = query.cues.iter().find_map(|value| match value {
            nous_core::Cue::SelfFacet(value) => Some(value),
            _ => None,
        });
        let exact_facets = bound
            .exact_bindings
            .iter()
            .filter_map(|binding| match binding.bound_ref {
                CognitiveRef::SelfFacetRevision(value) => Some(value.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let exact_narratives = bound
            .exact_bindings
            .iter()
            .filter_map(|binding| match binding.bound_ref {
                CognitiveRef::NarrativeIdentityRevision(value) => Some(value.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let has_exact = bound.exact_bindings.iter().any(|binding| {
            matches!(
                binding.bound_ref,
                CognitiveRef::SelfFacetRevision(_) | CognitiveRef::NarrativeIdentityRevision(_)
            )
        });
        let limit = plan.lane_budget(nous_core::EvidenceFamily::SelfDirect) as i64;
        if has_exact {
            let mut exact_rows = Vec::<(
                CognitiveRef,
                String,
                String,
                TemporalExtent,
                DateTime<Utc>,
                DateTime<Utc>,
            )>::new();
            if !exact_facets.is_empty() {
                let rows = sqlx::query("SELECT f.current_revision_id,f.object_epoch,r.self_facet_revision_id,f.kind,r.statement,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_id=f.self_facet_id WHERE f.subject_id=$1 AND r.self_facet_revision_id=ANY($2::uuid[]) AND f.acceptance_state='accepted' AND f.integrity_state='valid' AND f.suppression_state='normal' AND f.purge_state='normal' ORDER BY r.self_facet_revision_id").bind(query.subject.0).bind(&exact_facets).fetch_all(self.store.pool()).await.map_err(db)?;
                for row in rows {
                    let revision: Uuid = row.try_get("self_facet_revision_id").map_err(db)?;
                    let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
                    let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
                    let mutable = bound.exact_bindings.iter().any(|binding| {
                        binding.bound_ref
                            == CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId(
                                revision,
                            ))
                            && binding.mutable_object
                    });
                    let bound_epoch = bound
                        .exact_bindings
                        .iter()
                        .find(|binding| {
                            binding.bound_ref
                                == CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId(
                                    revision,
                                ))
                        })
                        .and_then(|binding| binding.bound_object_epoch);
                    if mutable && (current != revision || bound_epoch != Some(epoch)) {
                        continue;
                    }
                    exact_rows.push((
                        CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId(revision)),
                        format!("self:{}", row.try_get::<String, _>("kind").map_err(db)?),
                        row.try_get("statement").map_err(db)?,
                        temporal_from_columns(
                            row.try_get("valid_time_kind").map_err(db)?,
                            row.try_get("valid_time_start").map_err(db)?,
                            row.try_get("valid_time_end").map_err(db)?,
                        )?,
                        row.try_get("formed_at").map_err(db)?,
                        row.try_get("recorded_at").map_err(db)?,
                    ));
                }
            }
            if !exact_narratives.is_empty() {
                let rows = sqlx::query("SELECT n.current_revision_id,n.object_epoch,r.narrative_identity_revision_id,r.text,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_id=n.narrative_identity_id WHERE n.subject_id=$1 AND r.narrative_identity_revision_id=ANY($2::uuid[]) AND n.acceptance_state='accepted' AND n.integrity_state='valid' AND n.suppression_state='normal' AND n.purge_state='normal' ORDER BY r.narrative_identity_revision_id").bind(query.subject.0).bind(&exact_narratives).fetch_all(self.store.pool()).await.map_err(db)?;
                for row in rows {
                    let revision: Uuid =
                        row.try_get("narrative_identity_revision_id").map_err(db)?;
                    let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
                    let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
                    let mutable = bound.exact_bindings.iter().any(|binding| {
                        binding.bound_ref
                            == CognitiveRef::NarrativeIdentityRevision(
                                nous_core::NarrativeIdentityRevisionId(revision),
                            )
                            && binding.mutable_object
                    });
                    let bound_epoch = bound
                        .exact_bindings
                        .iter()
                        .find(|binding| {
                            binding.bound_ref
                                == CognitiveRef::NarrativeIdentityRevision(
                                    nous_core::NarrativeIdentityRevisionId(revision),
                                )
                        })
                        .and_then(|binding| binding.bound_object_epoch);
                    if mutable && (current != revision || bound_epoch != Some(epoch)) {
                        continue;
                    }
                    exact_rows.push((
                        CognitiveRef::NarrativeIdentityRevision(
                            nous_core::NarrativeIdentityRevisionId(revision),
                        ),
                        "self:narrative".into(),
                        row.try_get("text").map_err(db)?,
                        temporal_from_columns(
                            row.try_get("valid_time_kind").map_err(db)?,
                            row.try_get("valid_time_start").map_err(db)?,
                            row.try_get("valid_time_end").map_err(db)?,
                        )?,
                        row.try_get("formed_at").map_err(db)?,
                        row.try_get("recorded_at").map_err(db)?,
                    ));
                }
            }
            exact_rows.sort_by_key(|left| left.0.to_string());
            let results = exact_rows
                .into_iter()
                .take(limit as usize)
                .enumerate()
                .map(
                    |(index, (reference, role, text, valid, formed, recorded))| {
                        hit(
                            reference,
                            role,
                            text,
                            index + 1,
                            valid,
                            formed,
                            recorded,
                            nous_core::EvidenceFamily::SelfDirect,
                            "self:authority",
                            &bound.enabled_lanes,
                        )
                    },
                )
                .collect();
            return Ok(CognitiveQueryResult {
                results,
                ..empty_result(bound.query_id)
            });
        }
        let kind = cue.and_then(|value| value.kind.clone());
        if let Some(value) = kind.as_deref() {
            SelfFacetKind::try_from(value)?;
        }
        let key = cue.and_then(|value| value.key.clone());
        let rows = if has_exact {
            sqlx::query("SELECT f.kind,r.self_facet_revision_id,r.statement,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_revision_id=f.current_revision_id WHERE f.subject_id=$1 AND r.self_facet_revision_id=ANY($2::uuid[]) AND f.acceptance_state='accepted' AND f.integrity_state='valid' AND f.suppression_state='normal' AND f.purge_state='normal' ORDER BY r.self_facet_revision_id LIMIT $3").bind(query.subject.0).bind(&exact_facets).bind(limit).fetch_all(self.store.pool()).await.map_err(db)?
        } else {
            sqlx::query("SELECT f.kind,r.self_facet_revision_id,r.statement,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_revision_id=f.current_revision_id WHERE f.subject_id=$1 AND f.acceptance_state='accepted' AND f.integrity_state='valid' AND f.suppression_state='normal' AND f.purge_state='normal' AND ($2::text IS NULL OR f.kind=$2) AND ($3::text IS NULL OR f.key=$3) ORDER BY CASE f.kind WHEN 'identity' THEN 0 WHEN 'role' THEN 1 WHEN 'limitation' THEN 2 WHEN 'capability' THEN 3 WHEN 'value' THEN 4 WHEN 'preference' THEN 5 WHEN 'tendency' THEN 6 ELSE 99 END,f.key,r.self_facet_revision_id LIMIT $4").bind(query.subject.0).bind(kind).bind(key).bind(limit).fetch_all(self.store.pool()).await.map_err(db)?
        };
        let mut results = rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let kind: String = row.try_get("kind").map_err(db)?;
                Ok(hit(
                    CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId(
                        row.try_get("self_facet_revision_id").map_err(db)?,
                    )),
                    format!("self:{kind}"),
                    row.try_get("statement").map_err(db)?,
                    index + 1,
                    temporal_from_columns(
                        row.try_get("valid_time_kind").map_err(db)?,
                        row.try_get("valid_time_start").map_err(db)?,
                        row.try_get("valid_time_end").map_err(db)?,
                    )?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                    nous_core::EvidenceFamily::SelfDirect,
                    "self:authority",
                    &bound.enabled_lanes,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        if cue.is_none()
            && !has_exact
            && query
                .targets
                .iter()
                .any(|target| matches!(target, nous_core::QueryTarget::SelfCognition))
        {
            let remaining = limit.saturating_sub(results.len() as i64);
            if remaining > 0 {
                let rows = sqlx::query("SELECT r.narrative_identity_revision_id,r.text,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_revision_id=n.current_revision_id WHERE n.subject_id=$1 AND n.acceptance_state='accepted' AND n.integrity_state='valid' AND n.suppression_state='normal' AND n.purge_state='normal' ORDER BY n.key,r.narrative_identity_revision_id LIMIT $2").bind(query.subject.0).bind(remaining).fetch_all(self.store.pool()).await.map_err(db)?;
                for (offset, row) in rows.into_iter().enumerate() {
                    results.push(hit(
                        CognitiveRef::NarrativeIdentityRevision(
                            nous_core::NarrativeIdentityRevisionId(
                                row.try_get("narrative_identity_revision_id").map_err(db)?,
                            ),
                        ),
                        "self:narrative".into(),
                        row.try_get("text").map_err(db)?,
                        results.len() + offset + 1,
                        temporal_from_columns(
                            row.try_get("valid_time_kind").map_err(db)?,
                            row.try_get("valid_time_start").map_err(db)?,
                            row.try_get("valid_time_end").map_err(db)?,
                        )?,
                        row.try_get("formed_at").map_err(db)?,
                        row.try_get("recorded_at").map_err(db)?,
                        nous_core::EvidenceFamily::SelfDirect,
                        "self:authority",
                        &bound.enabled_lanes,
                    ));
                }
            }
        }
        for candidate in self.serving_hits(bound, plan).await? {
            if let Some(existing) = results
                .iter_mut()
                .find(|value| value.reference == candidate.reference)
            {
                existing.match_evidence.final_score += candidate.match_evidence.final_score;
                existing.match_evidence.base_rank_score += candidate.match_evidence.base_rank_score;
                existing.match_evidence.best_lane_rank = existing
                    .match_evidence
                    .best_lane_rank
                    .min(candidate.match_evidence.best_lane_rank);
                existing
                    .match_evidence
                    .families
                    .extend(candidate.match_evidence.families);
                existing
                    .match_evidence
                    .variants
                    .extend(candidate.match_evidence.variants);
            } else {
                results.push(candidate);
            }
        }
        for result in &mut results {
            result.match_evidence.families.sort();
            result.match_evidence.families.dedup();
            result.match_evidence.variants.sort();
            result.match_evidence.variants.dedup();
        }
        results.sort_by(|left, right| {
            right
                .match_evidence
                .final_score
                .total_cmp(&left.match_evidence.final_score)
                .then_with(|| left.reference.to_string().cmp(&right.reference.to_string()))
        });
        results.truncate(limit as usize);
        Ok(CognitiveQueryResult {
            results,
            ..empty_result(bound.query_id)
        })
    }
}

impl SelfService {
    pub fn new(store: AuthorityStore) -> Self {
        Self {
            store,
            serving: None,
        }
    }

    pub fn with_serving(mut self, serving: nous_serving::ServingService) -> Self {
        self.serving = Some(serving);
        self
    }

    #[expect(
        clippy::collapsible_if,
        reason = "idempotent receipt handling keeps committed retry return adjacent"
    )]
    pub async fn create_facet(&self, input: CreateSelfFacet) -> Result<SelfFacetView> {
        validate_facet_create(&input)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("self_facet.create", input.subject, &input)?;
        let facet_id = nous_core::SelfFacetId::new();
        let revision_id = nous_core::SelfFacetRevisionId::new();
        let now = Utc::now();
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "self_facet.create",
            &digest,
        )
        .await?
        {
            if receipt.state == "committed" {
                let id = receipt.result_ref.ok_or_else(|| {
                    nous_core::Error::Infrastructure("Self receipt has no result ref".into())
                })?;
                let parsed = Uuid::parse_str(&id).map_err(|_| {
                    nous_core::Error::Infrastructure("invalid Self receipt ref".into())
                })?;
                tx.commit().await.map_err(db)?;
                return self
                    .facet(input.subject, nous_core::SelfFacetId(parsed))
                    .await;
            }
        }
        sqlx::query("SELECT subject_id FROM subjects WHERE subject_id=$1 FOR UPDATE")
            .bind(input.subject.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM self_facets WHERE subject_id=$1 AND kind=$2 AND key=$3)",
        )
        .bind(input.subject.0)
        .bind(input.kind.as_str())
        .bind(&input.key)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?
        {
            return Err(nous_core::Error::Conflict(
                "Self facet (kind,key) already exists".into(),
            ));
        }
        validate_supports(&mut tx, input.subject, &input.supports).await?;
        let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO self_facets(self_facet_id,subject_id,kind,key,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,$4,$5,1,'accepted','valid','normal','normal',$6)").bind(facet_id.0).bind(input.subject.0).bind(input.kind.as_str()).bind(&input.key).bind(revision_id.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO self_facet_revisions(self_facet_revision_id,self_facet_id,subject_id,revision_no,parent_revision_id,revision_intent,statement,scope,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,1,NULL,NULL,$4,$5,$6,$7,$8,$9,$10,$11,$12)").bind(revision_id.0).bind(facet_id.0).bind(input.subject.0).bind(&input.statement).bind(&input.scope).bind(enum_name(input.epistemic_class)).bind(valid_kind).bind(valid_start).bind(valid_end).bind(input.formed_at).bind(now).bind(input.producer_signature_id).execute(&mut *tx).await.map_err(db)?;
        insert_supports(
            &mut tx,
            "self_facet_revision_supports",
            "self_facet_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        invalidate_projections(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "self_facet",
            &facet_id.0.to_string(),
            revision_id.0,
            1,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.facet(input.subject, facet_id).await
    }

    pub async fn revise_facet(&self, input: ReviseSelfFacet) -> Result<SelfFacetView> {
        validate_facet_revision(&input)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("self_facet.revise", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "self_facet.revise",
            &digest,
        )
        .await?
            && receipt.state == "committed"
        {
            tx.commit().await.map_err(db)?;
            return self.facet(input.subject, input.self_facet_id).await;
        }
        let row = sqlx::query("SELECT current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state FROM self_facets WHERE subject_id=$1 AND self_facet_id=$2 FOR UPDATE").bind(input.subject.0).bind(input.self_facet_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(|| nous_core::Error::NotFound("Self facet not found".into()))?;
        let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(nous_core::Error::Conflict(
                "Self facet object_epoch is stale".into(),
            ));
        }
        if current != input.parent_revision_id.0 {
            return Err(nous_core::Error::Conflict(
                "Self facet parent revision is not current".into(),
            ));
        }
        require_active_lifecycle(&row)?;
        validate_supports(&mut tx, input.subject, &input.supports).await?;
        let revision_id = nous_core::SelfFacetRevisionId::new();
        let next: i32 = sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM self_facet_revisions WHERE self_facet_id=$1").bind(input.self_facet_id.0).fetch_one(&mut *tx).await.map_err(db)?;
        let now = Utc::now();
        let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO self_facet_revisions(self_facet_revision_id,self_facet_id,subject_id,revision_no,parent_revision_id,revision_intent,statement,scope,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)").bind(revision_id.0).bind(input.self_facet_id.0).bind(input.subject.0).bind(next).bind(input.parent_revision_id.0).bind(enum_name(input.revision_intent)).bind(&input.statement).bind(&input.scope).bind(enum_name(input.epistemic_class)).bind(valid_kind).bind(valid_start).bind(valid_end).bind(input.formed_at).bind(now).bind(input.producer_signature_id).execute(&mut *tx).await.map_err(db)?;
        insert_supports(
            &mut tx,
            "self_facet_revision_supports",
            "self_facet_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        sqlx::query("UPDATE self_facets SET current_revision_id=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND self_facet_id=$2").bind(input.subject.0).bind(input.self_facet_id.0).bind(revision_id.0).execute(&mut *tx).await.map_err(db)?;
        invalidate_projections(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "self_facet",
            &input.self_facet_id.0.to_string(),
            revision_id.0,
            epoch + 1,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.facet(input.subject, input.self_facet_id).await
    }
    pub async fn facet(
        &self,
        subject: SubjectId,
        id: nous_core::SelfFacetId,
    ) -> Result<SelfFacetView> {
        let row = sqlx::query("SELECT f.kind,f.key,f.current_revision_id,f.object_epoch,f.acceptance_state,f.integrity_state,f.suppression_state,f.purge_state,f.created_at,r.self_facet_revision_id,r.revision_no,r.parent_revision_id,r.revision_intent,r.statement,r.scope,r.epistemic_class,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_revision_id=f.current_revision_id WHERE f.subject_id=$1 AND f.self_facet_id=$2")
            .bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?
            .ok_or_else(|| nous_core::Error::NotFound("Self facet not found".into()))?;
        let revision_id =
            nous_core::SelfFacetRevisionId(row.try_get("self_facet_revision_id").map_err(db)?);
        Ok(SelfFacetView {
            object: SelfFacet {
                self_facet_id: id,
                subject_id: subject,
                kind: parse_kind(row.try_get("kind").map_err(db)?)?,
                key: row.try_get("key").map_err(db)?,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: parse_enum(
                    row.try_get("acceptance_state").map_err(db)?,
                    "acceptance state",
                )?,
                integrity_state: parse_enum(
                    row.try_get("integrity_state").map_err(db)?,
                    "integrity state",
                )?,
                suppression_state: parse_enum(
                    row.try_get("suppression_state").map_err(db)?,
                    "suppression state",
                )?,
                purge_state: parse_enum(row.try_get("purge_state").map_err(db)?, "purge state")?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision: SelfFacetRevision {
                self_facet_revision_id: revision_id,
                self_facet_id: id,
                subject_id: subject,
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(nous_core::SelfFacetRevisionId),
                revision_intent: row
                    .try_get::<Option<String>, _>("revision_intent")
                    .map_err(db)?
                    .map(|v| parse_enum(v, "Self revision intent"))
                    .transpose()?,
                statement: row.try_get("statement").map_err(db)?,
                scope: row.try_get("scope").map_err(db)?,
                epistemic_class: parse_enum(
                    row.try_get("epistemic_class").map_err(db)?,
                    "epistemic class",
                )?,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                supports: load_supports(
                    self.store.pool(),
                    "self_facet_revision_supports",
                    "self_facet_revision_id",
                    revision_id.0,
                    subject,
                )
                .await?,
            },
        })
    }

    pub async fn narrative(
        &self,
        subject: SubjectId,
        id: nous_core::NarrativeIdentityId,
    ) -> Result<NarrativeIdentityView> {
        let row=sqlx::query("SELECT n.key,n.current_revision_id,n.object_epoch,n.acceptance_state,n.integrity_state,n.suppression_state,n.purge_state,n.created_at,r.narrative_identity_revision_id,r.revision_no,r.parent_revision_id,r.revision_intent,r.text,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_revision_id=n.current_revision_id WHERE n.subject_id=$1 AND n.narrative_identity_id=$2").bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(||nous_core::Error::NotFound("Narrative Identity not found".into()))?;
        let revision_id = nous_core::NarrativeIdentityRevisionId(
            row.try_get("narrative_identity_revision_id").map_err(db)?,
        );
        Ok(NarrativeIdentityView {
            object: NarrativeIdentity {
                narrative_identity_id: id,
                subject_id: subject,
                key: row.try_get("key").map_err(db)?,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: parse_enum(
                    row.try_get("acceptance_state").map_err(db)?,
                    "acceptance state",
                )?,
                integrity_state: parse_enum(
                    row.try_get("integrity_state").map_err(db)?,
                    "integrity state",
                )?,
                suppression_state: parse_enum(
                    row.try_get("suppression_state").map_err(db)?,
                    "suppression state",
                )?,
                purge_state: parse_enum(row.try_get("purge_state").map_err(db)?, "purge state")?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision: NarrativeIdentityRevision {
                narrative_identity_revision_id: revision_id,
                narrative_identity_id: id,
                subject_id: subject,
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(nous_core::NarrativeIdentityRevisionId),
                revision_intent: row
                    .try_get::<Option<String>, _>("revision_intent")
                    .map_err(db)?
                    .map(|v| parse_enum(v, "Self revision intent"))
                    .transpose()?,
                text: row.try_get("text").map_err(db)?,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                supports: load_supports(
                    self.store.pool(),
                    "narrative_identity_revision_supports",
                    "narrative_identity_revision_id",
                    revision_id.0,
                    subject,
                )
                .await?,
                references: load_narrative_references(self.store.pool(), revision_id.0).await?,
            },
        })
    }
}
