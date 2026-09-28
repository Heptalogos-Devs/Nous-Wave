use chrono::{DateTime, Utc};
use nous_authority_store::{AuthorityStore, ProjectionInvalidation, database_error as db};
use nous_core::{
    EvidenceLocator, EvidenceRef, OperationId, Result, RevisionSupport, SubjectId, TemporalExtent,
    parse_reference, reference_parts,
};
use nous_self_domain::{
    NarrativeReference, NarrativeReferenceInput, SelfFacetKind, SelfLifecycleOperation,
};
use serde::Serialize;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) fn parse_kind(value: String) -> Result<SelfFacetKind> {
    SelfFacetKind::try_from(value.as_str())
}

pub(super) fn parse_enum<T: serde::de::DeserializeOwned>(value: String, name: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| nous_core::Error::Infrastructure(format!("invalid {name}: {error}")))
}

pub(super) fn enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .map_err(|e| nous_core::Error::Internal(e.to_string()))
        .and_then(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| nous_core::Error::Internal("expected enum string".into()))
        })
        .unwrap_or_default()
}

pub(super) fn temporal_columns(
    value: &TemporalExtent,
) -> (&'static str, Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    match value {
        TemporalExtent::Unknown => ("unknown", None, None),
        TemporalExtent::Instant { at } => ("instant", Some(*at), None),
        TemporalExtent::Interval { start, end } => ("interval", *start, *end),
    }
}

pub(super) fn temporal_from_columns(
    kind: String,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<TemporalExtent> {
    match kind.as_str() {
        "unknown" => Ok(TemporalExtent::Unknown),
        "instant" => Ok(TemporalExtent::Instant {
            at: start.ok_or_else(|| {
                nous_core::Error::Infrastructure("instant has no timestamp".into())
            })?,
        }),
        "interval" => Ok(TemporalExtent::Interval { start, end }),
        _ => Err(nous_core::Error::Infrastructure(
            "invalid temporal extent kind".into(),
        )),
    }
}

pub(super) fn require_active_lifecycle(row: &sqlx::postgres::PgRow) -> Result<()> {
    let acceptance: String = row.try_get("acceptance_state").map_err(db)?;
    let integrity: String = row.try_get("integrity_state").map_err(db)?;
    let suppression: String = row.try_get("suppression_state").map_err(db)?;
    let purge: String = row.try_get("purge_state").map_err(db)?;
    if acceptance != "accepted"
        || integrity != "valid"
        || suppression != "normal"
        || purge != "normal"
    {
        return Err(nous_core::Error::FailedPrecondition(
            "Self object is not accepted, valid, normal, and not purging".into(),
        ));
    }
    Ok(())
}

pub(super) async fn invalidate_projections(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<()> {
    AuthorityStore::invalidate_in(tx, subject, ProjectionInvalidation::all())
        .await
        .map(|_| ())
}

pub(super) async fn lock_operation(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("{}:{}", subject.0, operation.0))
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}

#[derive(Debug)]
pub(super) struct Receipt {
    pub(super) state: String,
    pub(super) result_ref: Option<String>,
}

pub(super) async fn check_receipt(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
    kind: &str,
    digest: &str,
) -> Result<Option<Receipt>> {
    if let Some(row) = sqlx::query("SELECT state,result_ref,request_digest FROM mutation_receipts WHERE subject_id=$1 AND operation_id=$2").bind(subject.0).bind(operation.0).fetch_optional(&mut **tx).await.map_err(db)? {
        let existing: String = row.try_get("request_digest").map_err(db)?;
        if existing != digest { return Err(nous_core::Error::Conflict(format!("{kind} operation_id was used with a different request"))); }
        return Ok(Some(Receipt { state: row.try_get("state").map_err(db)?, result_ref: row.try_get("result_ref").map_err(db)? }));
    }
    sqlx::query("INSERT INTO mutation_receipts(subject_id,operation_id,operation_kind,request_digest,state,created_at) VALUES($1,$2,$3,$4,'in_progress',$5)").bind(subject.0).bind(operation.0).bind(kind).bind(digest).bind(Utc::now()).execute(&mut **tx).await.map_err(db)?;
    Ok(None)
}

pub(super) async fn commit_receipt(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
    kind: &str,
    reference: &str,
    revision: Uuid,
    epoch: i64,
) -> Result<()> {
    sqlx::query("UPDATE mutation_receipts SET state='committed',result_kind=$3,result_ref=$4,result_revision=$5,result_epoch=$6,committed_at=$7 WHERE subject_id=$1 AND operation_id=$2").bind(subject.0).bind(operation.0).bind(kind).bind(reference).bind(revision).bind(epoch).bind(Utc::now()).execute(&mut **tx).await.map_err(db)?;
    Ok(())
}

pub(super) async fn validate_supports(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    supports: &[RevisionSupport],
) -> Result<()> {
    for support in supports {
        match support {
            RevisionSupport::Evidence(value) => {
                validate_evidence(tx, subject, value).await?;
            }
            RevisionSupport::CognitionDependency(value) => {
                let (kind, reference) = reference_parts(&value.target_revision);
                let exists = match kind.as_str() {
                    "memory_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2)").bind(subject.0).bind(Uuid::parse_str(&reference).map_err(|_| nous_core::Error::Invalid("invalid Memory revision support".into()))?).fetch_one(&mut **tx).await.map_err(db)?,
                    "cognitive_schema_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2)").bind(subject.0).bind(Uuid::parse_str(&reference).map_err(|_| nous_core::Error::Invalid("invalid Schema revision support".into()))?).fetch_one(&mut **tx).await.map_err(db)?,
                    "self_facet_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM self_facet_revisions WHERE subject_id=$1 AND self_facet_revision_id=$2)").bind(subject.0).bind(Uuid::parse_str(&reference).map_err(|_| nous_core::Error::Invalid("invalid Self revision support".into()))?).fetch_one(&mut **tx).await.map_err(db)?,
                    "narrative_identity_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM narrative_identity_revisions WHERE subject_id=$1 AND narrative_identity_revision_id=$2)").bind(subject.0).bind(Uuid::parse_str(&reference).map_err(|_| nous_core::Error::Invalid("invalid Narrative revision support".into()))?).fetch_one(&mut **tx).await.map_err(db)?,
                    _ => false,
                };
                if !exists {
                    return Err(nous_core::Error::FailedPrecondition(
                        "Self cognition dependency is not an owned exact revision".into(),
                    ));
                }
            }
            RevisionSupport::Seed(value) => {
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cognitive_seed_versions WHERE subject_id=$1 AND seed_version_id=$2)")
                    .bind(subject.0)
                    .bind(value.seed_version_id.0)
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(db)?;
                if !exists {
                    return Err(nous_core::Error::FailedPrecondition(
                        "Cognitive Seed support is not owned by Subject".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

async fn validate_evidence(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    evidence: &EvidenceRef,
) -> Result<()> {
    let row = sqlx::query(
        "SELECT artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2",
    )
    .bind(subject.0)
    .bind(evidence.occurrence_id.0)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(|| {
        nous_core::Error::FailedPrecondition(
            "Self support occurrence is not owned by Subject".into(),
        )
    })?;
    let occurrence_artifact: Option<Uuid> = row.try_get("artifact_id").map_err(db)?;
    match evidence.locator {
        EvidenceLocator::WholeOccurrence => {}
        EvidenceLocator::SourceRegion(id) => {
            let artifact: Uuid = sqlx::query_scalar(
                "SELECT artifact_id FROM source_regions WHERE subject_id=$1 AND source_region_id=$2",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(&mut **tx)
            .await
            .map_err(db)?;
            if Some(artifact) != occurrence_artifact {
                return Err(nous_core::Error::FailedPrecondition(
                    "Self source region support does not match occurrence".into(),
                ));
            }
        }
        EvidenceLocator::DerivedRepresentation(id) => {
            let artifact: Option<Uuid> = sqlx::query_scalar(
                "SELECT sr.artifact_id FROM derived_representations d JOIN source_regions sr USING(source_region_id) WHERE d.subject_id=$1 AND d.derived_representation_id=$2",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(&mut **tx)
            .await
            .map_err(db)?;
            if artifact != occurrence_artifact {
                return Err(nous_core::Error::FailedPrecondition(
                    "Self derived representation support does not match occurrence".into(),
                ));
            }
        }
        EvidenceLocator::DerivedRegion(id) => {
            let artifact: Option<Uuid> = sqlx::query_scalar(
                "SELECT sr.artifact_id FROM derived_regions dr JOIN derived_representations d USING(derived_representation_id) JOIN source_regions sr USING(source_region_id) WHERE dr.subject_id=$1 AND dr.derived_region_id=$2",
            )
            .bind(subject.0)
            .bind(id.0)
            .fetch_one(&mut **tx)
            .await
            .map_err(db)?;
            if artifact != occurrence_artifact {
                return Err(nous_core::Error::FailedPrecondition(
                    "Self derived region support does not match occurrence".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(super) async fn insert_supports(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    column: &str,
    revision: Uuid,
    supports: &[RevisionSupport],
) -> Result<()> {
    for support in supports {
        let (
            kind,
            reference,
            role,
            occurrence,
            source_region,
            derived_representation,
            derived_region,
            seed_path,
        ) = match support {
            RevisionSupport::Evidence(value) => (
                "evidence".into(),
                value.canonical_key(),
                value.support_role.as_str().to_owned(),
                Some(value.occurrence_id.0),
                match value.locator {
                    EvidenceLocator::SourceRegion(id) => Some(id.0),
                    _ => None,
                },
                match value.locator {
                    EvidenceLocator::DerivedRepresentation(id) => Some(id.0),
                    _ => None,
                },
                match value.locator {
                    EvidenceLocator::DerivedRegion(id) => Some(id.0),
                    _ => None,
                },
                None,
            ),
            RevisionSupport::CognitionDependency(value) => {
                let (kind, reference) = reference_parts(&value.target_revision);
                (
                    kind,
                    reference,
                    value.support_role.as_str().to_owned(),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
            }
            RevisionSupport::Seed(value) => (
                "cognitive_seed_version".into(),
                value.seed_version_id.0.to_string(),
                "direct".into(),
                None,
                None,
                None,
                None,
                Some(value.semantic_path.clone()),
            ),
        };
        let sql = format!(
            "INSERT INTO {table}({column},support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,seed_path) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(revision)
            .bind(kind)
            .bind(reference)
            .bind(role)
            .bind(occurrence)
            .bind(source_region)
            .bind(derived_representation)
            .bind(derived_region)
            .bind(seed_path)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
    }
    Ok(())
}

pub(super) async fn load_supports(
    pool: &sqlx::PgPool,
    table: &str,
    column: &str,
    revision: Uuid,
    subject: SubjectId,
) -> Result<Vec<RevisionSupport>> {
    let sql = format!(
        "SELECT support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,seed_path FROM {table} WHERE {column}=$1 ORDER BY support_kind,support_ref,support_role"
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(revision)
        .fetch_all(pool)
        .await
        .map_err(db)?;
    let mut result = Vec::new();
    for row in rows {
        let kind: String = row.try_get("support_kind").map_err(db)?;
        let reference: String = row.try_get("support_ref").map_err(db)?;
        let role: String = row.try_get("support_role").map_err(db)?;
        if kind == "occurrence"
            || kind == "source_region"
            || kind == "derived_representation"
            || kind == "derived_region"
        {
            let occurrence: Uuid = row.try_get("occurrence_id").map_err(db)?;
            let locator = match kind.as_str() {
                "occurrence" => EvidenceLocator::WholeOccurrence,
                "source_region" => EvidenceLocator::SourceRegion(nous_core::SourceRegionId(
                    row.try_get::<Option<Uuid>, _>("source_region_id")
                        .map_err(db)?
                        .ok_or_else(|| {
                            nous_core::Error::Infrastructure(
                                "source region support missing locator".into(),
                            )
                        })?,
                )),
                "derived_representation" => {
                    EvidenceLocator::DerivedRepresentation(nous_core::DerivedRepresentationId(
                        row.try_get::<Option<Uuid>, _>("derived_representation_id")
                            .map_err(db)?
                            .ok_or_else(|| {
                                nous_core::Error::Infrastructure(
                                    "derived support missing locator".into(),
                                )
                            })?,
                    ))
                }
                _ => EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(
                    row.try_get::<Option<Uuid>, _>("derived_region_id")
                        .map_err(db)?
                        .ok_or_else(|| {
                            nous_core::Error::Infrastructure(
                                "derived region support missing locator".into(),
                            )
                        })?,
                )),
            };
            result.push(RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: nous_core::OccurrenceId(occurrence),
                locator,
                support_role: parse_enum(role, "support role")?,
            }));
        } else if kind == "cognitive_seed_version" {
            let target = parse_reference(&kind, &reference)?;
            let nous_core::CognitiveRef::CognitiveSeedVersion(seed) = target else {
                return Err(nous_core::Error::Infrastructure(
                    "invalid Cognitive Seed support reference".into(),
                ));
            };
            let path: String = row.try_get("seed_path").map_err(db)?;
            result.push(RevisionSupport::Seed(nous_core::SeedSupportRef::new(
                seed, path,
            )?));
        } else {
            let target = parse_reference(&kind, &reference)?;
            result.push(RevisionSupport::CognitionDependency(
                nous_core::CognitionDependency {
                    target_revision: target,
                    support_role: parse_enum(role, "support role")?,
                },
            ));
        }
    }
    let _ = subject;
    Ok(result)
}

pub(super) async fn validate_narrative_targets(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    references: &[NarrativeReferenceInput],
) -> Result<()> {
    for reference in references {
        let (kind, value) = reference_parts(&reference.target_exact_ref);
        let exists = match kind.as_str() {
            "memory_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2)").bind(subject.0).bind(Uuid::parse_str(&value).map_err(|_| nous_core::Error::Invalid("invalid narrative target".into()))?).fetch_one(&mut **tx).await.map_err(db)?,
            "self_facet_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM self_facet_revisions WHERE subject_id=$1 AND self_facet_revision_id=$2)").bind(subject.0).bind(Uuid::parse_str(&value).map_err(|_| nous_core::Error::Invalid("invalid narrative target".into()))?).fetch_one(&mut **tx).await.map_err(db)?,
            _ => false,
        };
        if !exists {
            return Err(nous_core::Error::FailedPrecondition(
                "NarrativeReference target is not an owned exact revision".into(),
            ));
        }
    }
    Ok(())
}

pub(super) async fn insert_narrative_references(
    tx: &mut Transaction<'_, Postgres>,
    revision: Uuid,
    references: &[NarrativeReferenceInput],
) -> Result<()> {
    for (position, reference) in references.iter().enumerate() {
        let (kind, value) = reference_parts(&reference.target_exact_ref);
        sqlx::query("INSERT INTO narrative_references(narrative_identity_revision_id,position,target_ref_kind,target_ref,role) VALUES($1,$2,$3,$4,$5)").bind(revision).bind(position as i32).bind(kind).bind(value).bind(&reference.role).execute(&mut **tx).await.map_err(db)?;
    }
    Ok(())
}

pub(super) async fn load_narrative_references(
    pool: &sqlx::PgPool,
    revision: Uuid,
) -> Result<Vec<NarrativeReference>> {
    let rows = sqlx::query("SELECT position,target_ref_kind,target_ref,role FROM narrative_references WHERE narrative_identity_revision_id=$1 ORDER BY position").bind(revision).fetch_all(pool).await.map_err(db)?;
    rows.into_iter()
        .map(|row| {
            Ok(NarrativeReference {
                narrative_revision_id: nous_core::NarrativeIdentityRevisionId(revision),
                position: row.try_get("position").map_err(db)?,
                target_exact_ref: parse_reference(
                    &row.try_get::<String, _>("target_ref_kind").map_err(db)?,
                    &row.try_get::<String, _>("target_ref").map_err(db)?,
                )?,
                role: row.try_get("role").map_err(db)?,
            })
        })
        .collect()
}

pub(super) async fn apply_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    column: &str,
    id: Uuid,
    operation: SelfLifecycleOperation,
) -> Result<()> {
    let (set, expected) = match operation {
        SelfLifecycleOperation::Withdraw => (
            "acceptance_state='withdrawn'",
            "acceptance_state='accepted'",
        ),
        SelfLifecycleOperation::Reaccept => (
            "acceptance_state='accepted'",
            "acceptance_state='withdrawn'",
        ),
        SelfLifecycleOperation::Suppress => (
            "suppression_state='suppressed'",
            "suppression_state='normal'",
        ),
        SelfLifecycleOperation::Restore => (
            "suppression_state='normal'",
            "suppression_state='suppressed'",
        ),
        SelfLifecycleOperation::MarkRevalidationRequired => (
            "integrity_state='revalidation_required'",
            "integrity_state='valid'",
        ),
        SelfLifecycleOperation::RestoreValid => (
            "integrity_state='valid'",
            "integrity_state='revalidation_required'",
        ),
        SelfLifecycleOperation::Purge => ("purge_state='purging'", "purge_state='normal'"),
    };
    let sql = format!(
        "UPDATE {table} SET {set},object_epoch=object_epoch+1 WHERE {column}=$1 AND {expected}"
    );
    if sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(db)?
        .rows_affected()
        != 1
    {
        return Err(nous_core::Error::FailedPrecondition(
            "Self lifecycle transition is not valid for current state".into(),
        ));
    }
    Ok(())
}
