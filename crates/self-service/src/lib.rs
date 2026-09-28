//! Self Authority persistence and fixed-owner query contribution.
mod lifecycle;
mod narrative;
mod query;
mod seed_import;
mod support;
use query::hit;
pub use seed_import::SeedImportResult;
use support::*;

use async_trait::async_trait;
use chrono::Utc;
use nous_authority_store::{AuthorityStore, database_error as db};
use nous_core::{
    CognitiveRef, EpistemicClass, OperationId, Result, RevisionSupport, SubjectId, TemporalExtent,
    canonical_request_digest,
};
use nous_self_domain::{
    CreateNarrativeIdentity, CreateSelfFacet, NarrativeIdentity, NarrativeIdentityRevision,
    ReviseNarrativeIdentity, ReviseSelfFacet, SelfFacet, SelfFacetKind, SelfFacetRevision,
    validate_facet_create, validate_facet_revision, validate_narrative_create,
    validate_narrative_revision,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct SelfService {
    pub store: AuthorityStore,
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

#[async_trait]
impl nous_cognitive_runtime::CognitiveContributor for SelfService {
    fn owns(&self, reference: &nous_core::CognitiveRef) -> bool {
        matches!(
            reference,
            nous_core::CognitiveRef::SelfFacet(_)
                | nous_core::CognitiveRef::SelfFacetRevision(_)
                | nous_core::CognitiveRef::NarrativeIdentity(_)
                | nous_core::CognitiveRef::NarrativeIdentityRevision(_)
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Self direct-lane SQL keeps exact, facet, and narrative candidate ordering together"
    )]
    async fn direct_lanes(
        &self,
        bound: &nous_cognitive_runtime::BoundQuery,
        plan: &nous_cognitive_runtime::QueryPlan,
    ) -> Result<Vec<nous_cognitive_runtime::LaneOutput>> {
        let query = &bound.source_query;
        if !plan
            .enabled_lanes
            .contains(&nous_core::EvidenceFamily::SelfDirect)
        {
            return Ok(Vec::new());
        }
        let self_target = query
            .targets
            .iter()
            .any(|target| matches!(target, nous_core::QueryTarget::SelfCognition));
        let self_cue = query.cues.iter().find_map(|cue| match cue {
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
        if !self_target
            && self_cue.is_none()
            && exact_facets.is_empty()
            && exact_narratives.is_empty()
        {
            return Ok(Vec::new());
        }

        let mut output = nous_cognitive_runtime::LaneOutput::empty(
            nous_core::EvidenceFamily::SelfDirect,
            nous_cognitive_runtime::LaneStatus::Ready,
        );
        let limit = plan.lane_budget(nous_core::EvidenceFamily::SelfDirect) as i64;
        let mut rank = 1_u32;
        if !exact_facets.is_empty() {
            let rows = sqlx::query("SELECT r.self_facet_revision_id FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_id=f.self_facet_id WHERE f.subject_id=$1 AND r.self_facet_revision_id=ANY($2::uuid[]) AND f.acceptance_state='accepted' AND f.integrity_state='valid' AND f.suppression_state='normal' AND f.purge_state='normal' ORDER BY r.self_facet_revision_id")
                .bind(query.subject.0)
                .bind(&exact_facets)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                output
                    .candidates
                    .push(nous_cognitive_runtime::LaneCandidate {
                        reference: CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId(
                            row.try_get("self_facet_revision_id").map_err(db)?,
                        )),
                        rank,
                        variants: vec!["self:exact".into()],
                        provider_metadata: serde_json::Value::Null,
                    });
                rank += 1;
            }
        }
        if !exact_narratives.is_empty() {
            let rows = sqlx::query("SELECT r.narrative_identity_revision_id FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_id=n.narrative_identity_id WHERE n.subject_id=$1 AND r.narrative_identity_revision_id=ANY($2::uuid[]) AND n.acceptance_state='accepted' AND n.integrity_state='valid' AND n.suppression_state='normal' AND n.purge_state='normal' ORDER BY r.narrative_identity_revision_id")
                .bind(query.subject.0)
                .bind(&exact_narratives)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                output
                    .candidates
                    .push(nous_cognitive_runtime::LaneCandidate {
                        reference: CognitiveRef::NarrativeIdentityRevision(
                            nous_core::NarrativeIdentityRevisionId(
                                row.try_get("narrative_identity_revision_id").map_err(db)?,
                            ),
                        ),
                        rank,
                        variants: vec!["self:exact".into()],
                        provider_metadata: serde_json::Value::Null,
                    });
                rank += 1;
            }
        }

        if exact_facets.is_empty() && exact_narratives.is_empty() {
            let kind = self_cue.and_then(|value| value.kind.clone());
            if let Some(value) = kind.as_deref() {
                SelfFacetKind::try_from(value)?;
            }
            let key = self_cue.and_then(|value| value.key.clone());
            let rows = sqlx::query("SELECT r.self_facet_revision_id FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_revision_id=f.current_revision_id WHERE f.subject_id=$1 AND f.acceptance_state='accepted' AND f.integrity_state='valid' AND f.suppression_state='normal' AND f.purge_state='normal' AND ($2::text IS NULL OR f.kind=$2) AND ($3::text IS NULL OR f.key=$3) ORDER BY CASE f.kind WHEN 'identity' THEN 0 WHEN 'role' THEN 1 WHEN 'limitation' THEN 2 WHEN 'capability' THEN 3 WHEN 'value' THEN 4 WHEN 'preference' THEN 5 WHEN 'tendency' THEN 6 ELSE 99 END,f.key,r.self_facet_revision_id LIMIT $4")
                .bind(query.subject.0)
                .bind(kind)
                .bind(key)
                .bind(limit)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                output
                    .candidates
                    .push(nous_cognitive_runtime::LaneCandidate {
                        reference: CognitiveRef::SelfFacetRevision(nous_core::SelfFacetRevisionId(
                            row.try_get("self_facet_revision_id").map_err(db)?,
                        )),
                        rank,
                        variants: vec!["self:authority".into()],
                        provider_metadata: serde_json::Value::Null,
                    });
                rank += 1;
            }
            if self_target && self_cue.is_none() {
                let remaining = limit.saturating_sub(output.candidates.len() as i64);
                if remaining > 0 {
                    let rows = sqlx::query("SELECT r.narrative_identity_revision_id FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_id=n.narrative_identity_id WHERE n.subject_id=$1 AND n.acceptance_state='accepted' AND n.integrity_state='valid' AND n.suppression_state='normal' AND n.purge_state='normal' ORDER BY n.key,r.narrative_identity_revision_id LIMIT $2")
                        .bind(query.subject.0)
                        .bind(remaining)
                        .fetch_all(self.store.pool())
                        .await
                        .map_err(db)?;
                    for row in rows {
                        output
                            .candidates
                            .push(nous_cognitive_runtime::LaneCandidate {
                                reference: CognitiveRef::NarrativeIdentityRevision(
                                    nous_core::NarrativeIdentityRevisionId(
                                        row.try_get("narrative_identity_revision_id")
                                            .map_err(db)?,
                                    ),
                                ),
                                rank,
                                variants: vec!["self:narrative".into()],
                                provider_metadata: serde_json::Value::Null,
                            });
                        rank += 1;
                    }
                }
            }
        }
        output.candidates.truncate(limit as usize);
        Ok(vec![output])
    }

    async fn validate_and_materialize(
        &self,
        _subject: SubjectId,
        references: &[nous_core::CognitiveRef],
        bound: &nous_cognitive_runtime::BoundQuery,
    ) -> Result<(
        Vec<nous_core::CognitiveHit>,
        std::collections::BTreeMap<String, usize>,
    )> {
        let subject = bound.source_query.subject;
        let facet_refs = references
            .iter()
            .filter_map(|reference| match reference {
                CognitiveRef::SelfFacetRevision(value) => Some(value.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let narrative_refs = references
            .iter()
            .filter_map(|reference| match reference {
                CognitiveRef::NarrativeIdentityRevision(value) => Some(value.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut drops = std::collections::BTreeMap::new();
        let mut hits = Vec::new();
        if !facet_refs.is_empty() {
            let rows = sqlx::query("SELECT f.current_revision_id,f.object_epoch,f.acceptance_state,f.integrity_state,f.suppression_state,f.purge_state,r.self_facet_revision_id,f.kind,r.statement,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_id=f.self_facet_id WHERE f.subject_id=$1 AND r.self_facet_revision_id=ANY($2::uuid[])")
                .bind(subject.0)
                .bind(&facet_refs)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                let revision = nous_core::SelfFacetRevisionId(
                    row.try_get("self_facet_revision_id").map_err(db)?,
                );
                let reference = CognitiveRef::SelfFacetRevision(revision);
                if !self_reference_is_available(&row, &reference, bound, &mut drops)? {
                    continue;
                }
                let hit = hit(
                    reference,
                    format!("self:{}", row.try_get::<String, _>("kind").map_err(db)?),
                    row.try_get("statement").map_err(db)?,
                    temporal_from_columns(
                        row.try_get("valid_time_kind").map_err(db)?,
                        row.try_get("valid_time_start").map_err(db)?,
                        row.try_get("valid_time_end").map_err(db)?,
                    )?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                );
                hits.push(hit);
            }
        }
        if !narrative_refs.is_empty() {
            let rows = sqlx::query("SELECT n.current_revision_id,n.object_epoch,n.acceptance_state,n.integrity_state,n.suppression_state,n.purge_state,r.narrative_identity_revision_id,r.text,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_id=n.narrative_identity_id WHERE n.subject_id=$1 AND r.narrative_identity_revision_id=ANY($2::uuid[])")
                .bind(subject.0)
                .bind(&narrative_refs)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                let revision = nous_core::NarrativeIdentityRevisionId(
                    row.try_get("narrative_identity_revision_id").map_err(db)?,
                );
                let reference = CognitiveRef::NarrativeIdentityRevision(revision);
                if !self_reference_is_available(&row, &reference, bound, &mut drops)? {
                    continue;
                }
                let hit = hit(
                    reference,
                    "self:narrative".into(),
                    row.try_get("text").map_err(db)?,
                    temporal_from_columns(
                        row.try_get("valid_time_kind").map_err(db)?,
                        row.try_get("valid_time_start").map_err(db)?,
                        row.try_get("valid_time_end").map_err(db)?,
                    )?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                );
                hits.push(hit);
            }
        }
        Ok((hits, drops))
    }
}

fn self_reference_is_available(
    row: &sqlx::postgres::PgRow,
    reference: &CognitiveRef,
    bound: &nous_cognitive_runtime::BoundQuery,
    drops: &mut std::collections::BTreeMap<String, usize>,
) -> Result<bool> {
    let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
    let revision = match reference {
        CognitiveRef::SelfFacetRevision(value) => value.0,
        CognitiveRef::NarrativeIdentityRevision(value) => value.0,
        _ => return Ok(false),
    };
    let historical = bound.revision_policy.allows_historical(reference);
    if !historical && current != revision {
        *drops.entry("not_current".into()).or_default() += 1;
        return Ok(false);
    }
    let state_ok = row.try_get::<String, _>("acceptance_state").map_err(db)? == "accepted"
        && row.try_get::<String, _>("integrity_state").map_err(db)? == "valid"
        && row.try_get::<String, _>("suppression_state").map_err(db)? == "normal"
        && row.try_get::<String, _>("purge_state").map_err(db)? == "normal";
    if !state_ok {
        *drops.entry("unavailable_lifecycle".into()).or_default() += 1;
        return Ok(false);
    }
    if let Some(binding) = bound
        .exact_bindings
        .iter()
        .find(|binding| binding.bound_ref == *reference)
        && binding.mutable_object
        && binding.bound_object_epoch != Some(row.try_get::<i64, _>("object_epoch").map_err(db)?)
    {
        *drops.entry("stale_exact_binding".into()).or_default() += 1;
        return Ok(false);
    }
    Ok(true)
}

impl SelfService {
    pub fn new(store: AuthorityStore) -> Self {
        Self { store }
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
