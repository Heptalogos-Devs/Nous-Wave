// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use crate::{AuthorityStore, database_error as db};
use nous_core::{Error, Result, SubjectId};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

/// Semantic owner identity; storage does not enumerate cognition domains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowOwner(String);
impl WorkflowOwner {
    pub fn new(value: &str) -> Result<Self> {
        if !(1..=64).contains(&value.len())
            || !value.as_bytes()[0].is_ascii_lowercase()
            || !value
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        {
            return Err(Error::Invalid("invalid workflow owner identifier".into()));
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A 1 MiB Material string can occupy 6 MiB after JSON escaping, plus metadata.
pub const ABSOLUTE_WORKFLOW_LEASE_SECONDS: u64 = 3600;
pub const WORKFLOW_VALUE_MAX_BYTES: usize = 8 * 1024 * 1024;

pub struct WorkflowReservation {
    pub snapshot: Value,
    pub proposal: Option<Value>,
    pub outcome: Option<Value>,
    pub lease_token: Option<Uuid>,
    pub busy: bool,
}

impl AuthorityStore {
    pub async fn reserve_model_workflow(
        &self,
        subject: SubjectId,
        owner: &str,
        key: &str,
        digest: &str,
        snapshot: &Value,
        lease_seconds: u64,
    ) -> Result<WorkflowReservation> {
        let owner_id = WorkflowOwner::new(owner)?;
        let owner = owner_id.as_str();
        if !(1..=ABSOLUTE_WORKFLOW_LEASE_SECONDS).contains(&lease_seconds)
            || key.is_empty()
            || key.len() > 256
            || digest.is_empty()
            || digest.len() > 128
            || serde_json::to_vec(snapshot)
                .map_err(|error| Error::Invalid(error.to_string()))?
                .len()
                > 262144
        {
            return Err(Error::Invalid("invalid model workflow reservation".into()));
        }
        let mut tx = self.begin().await?;
        let maintenance = snapshot.get("maintenance_claim");
        let maintenance_trigger = maintenance
            .and_then(|claim| claim.get("trigger_revision"))
            .and_then(Value::as_i64);
        let maintenance_need_id = if let Some(claim) = maintenance {
            let need: Uuid = claim
                .get("need_id")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("maintenance need required".into()))?
                .parse()
                .map_err(|_| Error::Invalid("invalid maintenance need".into()))?;
            let token: Uuid = claim
                .get("lease_token")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("maintenance lease required".into()))?
                .parse()
                .map_err(|_| Error::Invalid("invalid maintenance lease".into()))?;
            let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM maintenance_needs WHERE subject_id=$1 AND need_id=$2 AND state='leased' AND lease_token=$3 AND lease_until>clock_timestamp() AND trigger_revision>=$4)")
                .bind(subject.0).bind(need).bind(token).bind(maintenance_trigger.ok_or_else(|| Error::Invalid("maintenance trigger required".into()))?).fetch_one(&mut *tx).await.map_err(db)?;
            if !valid {
                return Err(Error::Conflict(
                    "maintenance lease expired or changed".into(),
                ));
            }
            Some(need)
        } else {
            None
        };

        sqlx::query("INSERT INTO model_workflow_operations(subject_id,owner,operation_key,semantic_digest,snapshot,maintenance_need_id,maintenance_trigger_revision) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING").bind(subject.0).bind(owner).bind(key).bind(digest).bind(snapshot).bind(maintenance_need_id).bind(maintenance_trigger).execute(&mut *tx).await.map_err(db)?;
        let row=sqlx::query("SELECT snapshot,proposal,outcome,semantic_digest,lease_until>now() AS live FROM model_workflow_operations WHERE subject_id=$1 AND owner=$2 AND operation_key=$3 FOR UPDATE").bind(subject.0).bind(owner).bind(key).fetch_one(&mut *tx).await.map_err(db)?;
        if row.try_get::<String, _>("semantic_digest").map_err(db)? != digest {
            return Err(Error::Conflict(
                "model operation identity has different semantic input".into(),
            ));
        }
        let snapshot = row.try_get("snapshot").map_err(db)?;
        let proposal = row.try_get("proposal").map_err(db)?;
        let outcome: Option<Value> = row.try_get("outcome").map_err(db)?;
        let busy = outcome.is_none()
            && row
                .try_get::<Option<bool>, _>("live")
                .map_err(db)?
                .unwrap_or(false);
        let token = if outcome.is_none() && !busy {
            Some(Uuid::now_v7())
        } else {
            None
        };
        if let Some(token) = token {
            sqlx::query("UPDATE model_workflow_operations SET lease_token=$4,lease_until=clock_timestamp()+($5::double precision * interval '1 second'),updated_at=now() WHERE subject_id=$1 AND owner=$2 AND operation_key=$3").bind(subject.0).bind(owner).bind(key).bind(token).bind(lease_seconds as f64).execute(&mut *tx).await.map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(WorkflowReservation {
            snapshot,
            proposal,
            outcome,
            lease_token: token,
            busy,
        })
    }

    pub async fn save_model_workflow(
        &self,
        subject: SubjectId,
        owner: &str,
        key: &str,
        token: Uuid,
        proposal: Option<&Value>,
        outcome: Option<&Value>,
    ) -> Result<()> {
        WorkflowOwner::new(owner)?;
        for value in [proposal, outcome].into_iter().flatten() {
            if serde_json::to_vec(value)
                .map_err(|error| Error::Invalid(error.to_string()))?
                .len()
                > WORKFLOW_VALUE_MAX_BYTES
            {
                return Err(Error::Invalid("model workflow value exceeds bound".into()));
            }
        }
        let updated=sqlx::query("UPDATE model_workflow_operations SET proposal=CASE WHEN $6::jsonb IS NULL THEN COALESCE($5,proposal) ELSE NULL END,outcome=COALESCE($6,outcome),snapshot=CASE WHEN $6::jsonb IS NULL THEN snapshot ELSE '{}'::jsonb END,lease_token=CASE WHEN $6::jsonb IS NULL THEN lease_token ELSE NULL END,lease_until=CASE WHEN $6::jsonb IS NULL THEN lease_until ELSE NULL END,updated_at=now() WHERE subject_id=$1 AND owner=$2 AND operation_key=$3 AND lease_token=$4 AND lease_until>now() AND outcome IS NULL").bind(subject.0).bind(owner).bind(key).bind(token).bind(proposal).bind(outcome).execute(self.pool()).await.map_err(db)?;
        if updated.rows_affected() != 1 {
            return Err(Error::Conflict(
                "model workflow lease expired or changed".into(),
            ));
        }
        Ok(())
    }

    pub async fn release_model_workflow(
        &self,
        subject: SubjectId,
        owner: &str,
        key: &str,
        token: Uuid,
    ) -> Result<()> {
        WorkflowOwner::new(owner)?;
        sqlx::query("UPDATE model_workflow_operations SET lease_token=NULL,lease_until=NULL,updated_at=now() WHERE subject_id=$1 AND owner=$2 AND operation_key=$3 AND lease_token=$4 AND outcome IS NULL").bind(subject.0).bind(owner).bind(key).bind(token).execute(self.pool()).await.map_err(db)?;
        Ok(())
    }
}
