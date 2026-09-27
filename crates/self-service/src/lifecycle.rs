use super::{
    SelfService, apply_lifecycle, canonical_request_digest, check_receipt, commit_receipt,
    invalidate_projections, lock_operation,
};
use chrono::Utc;
use nous_authority_store::database_error as db;
use nous_core::{CognitiveRef, Result, reference_parts};
use nous_self_domain::MutateSelfLifecycle;
use sqlx::Row;
use uuid::Uuid;

#[expect(
    clippy::too_many_lines,
    reason = "Self lifecycle keeps lock, dependency invalidation, purge, and receipt ordering atomic"
)]
impl SelfService {
    pub async fn lifecycle(&self, input: MutateSelfLifecycle) -> Result<()> {
        if input.operation_id.0.is_nil() || input.expected_object_epoch <= 0 {
            return Err(nous_core::Error::Invalid(
                "operation_id and positive expected_object_epoch are required".into(),
            ));
        }
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("self.lifecycle", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "self.lifecycle",
            &digest,
        )
        .await?
            && receipt.state == "committed"
        {
            tx.commit().await.map_err(db)?;
            return Ok(());
        }
        let (table, column, id, dep_kind) = match input.reference {
            CognitiveRef::SelfFacet(value) => (
                "self_facets",
                "self_facet_id",
                value.0,
                "self_facet_revision",
            ),
            CognitiveRef::NarrativeIdentity(value) => (
                "narrative_identities",
                "narrative_identity_id",
                value.0,
                "narrative_identity_revision",
            ),
            _ => {
                return Err(nous_core::Error::Invalid(
                    "Self lifecycle requires a Self object reference".into(),
                ));
            }
        };
        let sql = format!(
            "SELECT current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state FROM {table} WHERE subject_id=$1 AND {column}=$2 FOR UPDATE"
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(input.subject.0)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| nous_core::Error::NotFound("Self object not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(nous_core::Error::Conflict(
                "Self object_epoch is stale".into(),
            ));
        }
        let (ref_kind, ref_value) = reference_parts(&input.reference);
        if matches!(
            input.operation,
            nous_self_domain::SelfLifecycleOperation::Purge
        ) {
            let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
            sqlx::query("UPDATE self_facets f SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE f.subject_id=$1 AND EXISTS (SELECT 1 FROM self_facet_revision_supports s WHERE s.self_facet_revision_id=f.current_revision_id AND s.support_kind=$2 AND s.support_ref=$3)")
                .bind(input.subject.0)
                .bind(dep_kind)
                .bind(current.to_string())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query("UPDATE narrative_identities n SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE n.subject_id=$1 AND EXISTS (SELECT 1 FROM narrative_identity_revision_supports s WHERE s.narrative_identity_revision_id=n.current_revision_id AND s.support_kind=$2 AND s.support_ref=$3)")
                .bind(input.subject.0)
                .bind(dep_kind)
                .bind(current.to_string())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query("UPDATE narrative_identities SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE subject_id=$1 AND EXISTS (SELECT 1 FROM narrative_references WHERE narrative_identity_revision_id=narrative_identities.current_revision_id AND target_ref_kind=$2 AND target_ref=$3)").bind(input.subject.0).bind(dep_kind).bind(current.to_string()).execute(&mut *tx).await.map_err(db)?;
            let resident_delete = if table == "self_facets" {
                "DELETE FROM resident_refs WHERE ref_kind='self_facet_revision' AND ref_value IN (SELECT self_facet_revision_id::text FROM self_facet_revisions WHERE self_facet_id=$1)"
            } else {
                "DELETE FROM resident_refs WHERE ref_kind='narrative_identity_revision' AND ref_value IN (SELECT narrative_identity_revision_id::text FROM narrative_identity_revisions WHERE narrative_identity_id=$1)"
            };
            sqlx::query(resident_delete)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query("INSERT INTO self_purge_receipts(subject_id,operation_id,ref_kind,ref_value,request_digest,purged_at) VALUES($1,$2,$3,$4,$5,$6)").bind(input.subject.0).bind(input.operation_id.0).bind(&ref_kind).bind(&ref_value).bind(&digest).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
            let sql = format!("DELETE FROM {table} WHERE {column}=$1");
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        } else {
            apply_lifecycle(&mut tx, table, column, id, input.operation).await?;
            if matches!(
                input.operation,
                nous_self_domain::SelfLifecycleOperation::Withdraw
                    | nous_self_domain::SelfLifecycleOperation::Suppress
                    | nous_self_domain::SelfLifecycleOperation::MarkRevalidationRequired
            ) {
                let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
                sqlx::query("UPDATE self_facets f SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE f.subject_id=$1 AND EXISTS (SELECT 1 FROM self_facet_revision_supports s WHERE s.self_facet_revision_id=f.current_revision_id AND s.support_kind=$2 AND s.support_ref=$3)")
                    .bind(input.subject.0)
                    .bind(dep_kind)
                    .bind(current.to_string())
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                sqlx::query("UPDATE narrative_identities n SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE n.subject_id=$1 AND EXISTS (SELECT 1 FROM narrative_identity_revision_supports s WHERE s.narrative_identity_revision_id=n.current_revision_id AND s.support_kind=$2 AND s.support_ref=$3)")
                    .bind(input.subject.0)
                    .bind(dep_kind)
                    .bind(current.to_string())
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                sqlx::query("UPDATE narrative_identities SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE subject_id=$1 AND EXISTS (SELECT 1 FROM narrative_references WHERE narrative_identity_revision_id=narrative_identities.current_revision_id AND target_ref_kind=$2 AND target_ref=$3)").bind(input.subject.0).bind(dep_kind).bind(current.to_string()).execute(&mut *tx).await.map_err(db)?;
            }
        }
        invalidate_projections(&mut tx, input.subject).await?;
        let reference = input.reference.to_string();
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "self.lifecycle",
            &reference,
            input.operation_id.0,
            epoch + 1,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
}
