use super::{SelfService, temporal_from_columns};
use chrono::{DateTime, Utc};
use nous_authority_store::database_error as db;
use nous_core::{CognitiveRef, Result, SubjectId, TemporalExtent};
use sqlx::Row;

#[expect(
    clippy::too_many_arguments,
    reason = "RRF hit construction keeps owner, freshness, and lane evidence together"
)]
pub(super) fn hit(
    reference: CognitiveRef,
    role: String,
    text: String,
    rank: usize,
    valid: TemporalExtent,
    formed: DateTime<Utc>,
    recorded: DateTime<Utc>,
    family: nous_core::EvidenceFamily,
    variant: &str,
    enabled_lanes: &[nous_core::EvidenceFamily],
) -> nous_core::CognitiveHit {
    let score = 1.0 / rank.max(1) as f64;
    nous_core::CognitiveHit {
        revision: Some(reference.clone()),
        reference,
        semantic_role: Some(role.clone()),
        cognitive_role: role.strip_prefix("self:").map(str::to_owned),
        formation_mode: None,
        representation: Some(text),
        authority: nous_core::AuthorityClass::SubjectCognition,
        freshness: nous_core::FreshnessDescriptor {
            observed_at: None,
            valid_time: valid,
            formed_at: Some(formed),
            recorded_at: Some(recorded),
        },
        entity_refs: Vec::new(),
        evidence: Vec::new(),
        match_evidence: nous_core::MatchEvidence {
            families: vec![family],
            base_rank_score: score,
            best_lane_rank: rank as u32,
            enabled_lane_count: enabled_lanes.len() as u32,
            final_score: score,
            variants: vec![variant.into()],
            explanation: None,
        },
        materialization: Vec::new(),
    }
}

impl SelfService {
    pub async fn context_text(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<(String, TemporalExtent, DateTime<Utc>, DateTime<Utc>)> {
        let resolved = match reference {
            CognitiveRef::SelfFacet(_) | CognitiveRef::NarrativeIdentity(_) => {
                self.store.bind_exact_reference(subject, reference).await?.0
            }
            CognitiveRef::SelfFacetRevision(_) | CognitiveRef::NarrativeIdentityRevision(_) => {
                reference.clone()
            }
            _ => {
                return Err(nous_core::Error::Invalid(
                    "Self context requires a Self exact reference".into(),
                ));
            }
        };
        match resolved {
            CognitiveRef::SelfFacetRevision(revision) => {
                let row = sqlx::query("SELECT f.acceptance_state,f.integrity_state,f.suppression_state,f.purge_state,r.statement,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM self_facets f JOIN self_facet_revisions r ON r.self_facet_id=f.self_facet_id WHERE f.subject_id=$1 AND r.self_facet_revision_id=$2")
                    .bind(subject.0)
                    .bind(revision.0)
                    .fetch_optional(self.store.pool())
                    .await
                    .map_err(db)?
                    .ok_or_else(|| nous_core::Error::NotFound("Self facet revision not found".into()))?;
                let acceptance: String = row.try_get("acceptance_state").map_err(db)?;
                let integrity: String = row.try_get("integrity_state").map_err(db)?;
                let suppression: String = row.try_get("suppression_state").map_err(db)?;
                let purge: String = row.try_get("purge_state").map_err(db)?;
                if acceptance != "accepted"
                    || integrity != "valid"
                    || suppression != "normal"
                    || purge != "normal"
                {
                    return Err(nous_core::Error::Unavailable(
                        "Self facet is not available".into(),
                    ));
                }
                Ok((
                    row.try_get("statement").map_err(db)?,
                    temporal_from_columns(
                        row.try_get("valid_time_kind").map_err(db)?,
                        row.try_get("valid_time_start").map_err(db)?,
                        row.try_get("valid_time_end").map_err(db)?,
                    )?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                ))
            }
            CognitiveRef::NarrativeIdentityRevision(revision) => {
                let row = sqlx::query("SELECT n.acceptance_state,n.integrity_state,n.suppression_state,n.purge_state,r.text,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM narrative_identities n JOIN narrative_identity_revisions r ON r.narrative_identity_id=n.narrative_identity_id WHERE n.subject_id=$1 AND r.narrative_identity_revision_id=$2")
                    .bind(subject.0)
                    .bind(revision.0)
                    .fetch_optional(self.store.pool())
                    .await
                    .map_err(db)?
                    .ok_or_else(|| nous_core::Error::NotFound("Narrative revision not found".into()))?;
                let acceptance: String = row.try_get("acceptance_state").map_err(db)?;
                let integrity: String = row.try_get("integrity_state").map_err(db)?;
                let suppression: String = row.try_get("suppression_state").map_err(db)?;
                let purge: String = row.try_get("purge_state").map_err(db)?;
                if acceptance != "accepted"
                    || integrity != "valid"
                    || suppression != "normal"
                    || purge != "normal"
                {
                    return Err(nous_core::Error::Unavailable(
                        "Narrative is not available".into(),
                    ));
                }
                Ok((
                    row.try_get("text").map_err(db)?,
                    temporal_from_columns(
                        row.try_get("valid_time_kind").map_err(db)?,
                        row.try_get("valid_time_start").map_err(db)?,
                        row.try_get("valid_time_end").map_err(db)?,
                    )?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                ))
            }
            _ => Err(nous_core::Error::Infrastructure(
                "Self exact binding returned a non-Self reference".into(),
            )),
        }
    }
}
