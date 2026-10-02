//! Subject-scoped mutation receipt mechanics. Callers hold lock_operation in the same transaction.
use crate::database_error as db;
use chrono::Utc;
use nous_core::{Error, OperationId, Result, SubjectId};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MutationReceipt {
    pub state: String,
    pub result_ref: Option<String>,
    pub result_revision: Option<Uuid>,
}

pub async fn check_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    operation: OperationId,
    kind: &str,
    digest: &str,
) -> Result<Option<MutationReceipt>> {
    let row = sqlx::query("SELECT state,result_ref,result_revision,request_digest,operation_kind FROM mutation_receipts WHERE subject_id=$1 AND operation_id=$2")
        .bind(subject.0)
        .bind(operation.0)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
    if let Some(row) = row {
        let existing: String = row.try_get("request_digest").map_err(db)?;
        if existing != digest || row.try_get::<String, _>("operation_kind").map_err(db)? != kind {
            return Err(Error::Conflict(format!(
                "{kind} operation_id was used with a different request"
            )));
        }
        return Ok(Some(MutationReceipt {
            state: row.try_get("state").map_err(db)?,
            result_ref: row.try_get("result_ref").map_err(db)?,
            result_revision: row.try_get("result_revision").map_err(db)?,
        }));
    }
    sqlx::query("INSERT INTO mutation_receipts(subject_id,operation_id,operation_kind,request_digest,state,created_at) VALUES($1,$2,$3,$4,'in_progress',$5)")
        .bind(subject.0)
        .bind(operation.0)
        .bind(kind)
        .bind(digest)
        .bind(Utc::now())
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(None)
}

pub async fn commit_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    operation: OperationId,
    result_kind: &str,
    result_ref: Option<&str>,
    result_revision: Option<Uuid>,
    result_epoch: Option<i64>,
) -> Result<()> {
    sqlx::query("UPDATE mutation_receipts SET state='committed',result_kind=$3,result_ref=$4,result_revision=$5,result_epoch=$6,committed_at=$7 WHERE subject_id=$1 AND operation_id=$2")
        .bind(subject.0)
        .bind(operation.0)
        .bind(result_kind)
        .bind(result_ref)
        .bind(result_revision)
        .bind(result_epoch)
        .bind(Utc::now())
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}
