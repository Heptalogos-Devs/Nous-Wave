//! Durable work obligations and execution leases for host-granted maintenance.

use crate::*;
use nous_persistence::database_error as db;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};

pub const MAINTENANCE_KINDS: &[&str] = &[
    "episode_segment",
    "episode_resegment",
    "journal_review",
    "journal_revalidate",
    "memory_consolidate",
    "schema_review",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceNeed {
    pub need_id: Uuid,
    pub subject_id: SubjectId,
    pub kind: String,
    pub scope_kind: String,
    pub scope_ref: String,
    pub trigger_authority_seq: i64,
    pub due_at: DateTime<Utc>,
    pub priority: i32,
    pub state: String,
    pub lease_token: Option<Uuid>,
    pub lease_until: Option<DateTime<Utc>>,
    pub attempt_count: i32,
    pub last_problem_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MaintenanceRequest {
    pub subject: SubjectId,
    pub kind: String,
    pub scope_kind: String,
    pub scope_ref: String,
    pub trigger_authority_seq: i64,
    pub due_at: DateTime<Utc>,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub enum MaintenanceDisposition {
    Satisfied,
    Obsolete,
    Pending {
        due_at: DateTime<Utc>,
        problem_code: Option<String>,
    },
}

impl CognitiveRuntimeService {
    pub async fn enqueue_maintenance(&self, request: MaintenanceRequest) -> Result<Uuid> {
        self.require_subject(request.subject).await?;
        let mut tx = self.store.begin().await?;
        let id = self.enqueue_maintenance_in(&mut tx, &request).await?;
        tx.commit().await.map_err(db)?;
        Ok(id)
    }

    pub async fn enqueue_maintenance_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &MaintenanceRequest,
    ) -> Result<Uuid> {
        if !MAINTENANCE_KINDS.contains(&request.kind.as_str())
            || !valid_identifier(&request.scope_kind)
            || request.scope_ref.is_empty()
            || request.scope_ref.len() > 512
            || request.trigger_authority_seq < 0
            || !(0..=100).contains(&request.priority)
        {
            return Err(Error::Invalid("invalid maintenance scope or policy".into()));
        }
        sqlx::query_scalar(
            "INSERT INTO maintenance_needs(need_id,subject_id,kind,scope_kind,scope_ref,trigger_authority_seq,due_at,priority,state,created_at,updated_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'pending',$9,$9) ON CONFLICT(subject_id,kind,scope_kind,scope_ref) WHERE state IN ('pending','leased') DO UPDATE SET trigger_authority_seq=GREATEST(maintenance_needs.trigger_authority_seq,excluded.trigger_authority_seq),due_at=LEAST(maintenance_needs.due_at,excluded.due_at),priority=GREATEST(maintenance_needs.priority,excluded.priority),updated_at=excluded.updated_at RETURNING need_id",
        )
        .bind(Uuid::now_v7())
        .bind(request.subject.0)
        .bind(&request.kind)
        .bind(&request.scope_kind)
        .bind(&request.scope_ref)
        .bind(request.trigger_authority_seq)
        .bind(request.due_at)
        .bind(request.priority)
        .bind(self.now(request.subject))
        .fetch_one(&mut **tx)
        .await
        .map_err(db)
    }

    pub async fn lease_maintenance(
        &self,
        subject: SubjectId,
        allowed_kinds: &[String],
        limit: u32,
        lease_seconds: u32,
    ) -> Result<Vec<MaintenanceNeed>> {
        self.require_subject(subject).await?;
        if limit == 0
            || limit > 128
            || lease_seconds == 0
            || lease_seconds > 3600
            || allowed_kinds
                .iter()
                .any(|kind| !MAINTENANCE_KINDS.contains(&kind.as_str()))
        {
            return Err(Error::Invalid("invalid maintenance claim envelope".into()));
        }
        let mut tx = self.store.begin().await?;
        let now = self.now(subject);
        let rows = sqlx::query(
            "WITH due AS (SELECT need_id FROM maintenance_needs WHERE subject_id=$1 AND kind=ANY($2::text[]) AND due_at<=$3 AND (state='pending' OR (state='leased' AND lease_until<=clock_timestamp())) ORDER BY priority DESC,due_at,need_id LIMIT $4 FOR UPDATE SKIP LOCKED) UPDATE maintenance_needs n SET state='leased',lease_token=$5,lease_until=clock_timestamp()+make_interval(secs=>$6),attempt_count=attempt_count+1,updated_at=$3 FROM due WHERE n.need_id=due.need_id RETURNING n.*",
        )
        .bind(subject.0)
        .bind(allowed_kinds)
        .bind(now)
        .bind(i64::from(limit))
        .bind(Uuid::now_v7())
        .bind(f64::from(lease_seconds))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let needs = rows
            .into_iter()
            .map(need_from_row)
            .collect::<Result<Vec<_>>>()?;
        tx.commit().await.map_err(db)?;
        Ok(needs)
    }

    pub async fn acknowledge_maintenance(
        &self,
        claimed: &MaintenanceNeed,
        disposition: MaintenanceDisposition,
    ) -> Result<()> {
        let token = claimed
            .lease_token
            .ok_or_else(|| Error::Invalid("need is not leased".into()))?;
        let (state, due, problem) = match disposition {
            MaintenanceDisposition::Satisfied => ("satisfied", claimed.due_at, None),
            MaintenanceDisposition::Obsolete => ("obsolete", claimed.due_at, None),
            MaintenanceDisposition::Pending {
                due_at,
                problem_code,
            } => ("pending", due_at, problem_code),
        };
        // A trigger arriving during execution survives acknowledgement as pending work.
        let updated = sqlx::query(
            "UPDATE maintenance_needs SET state=CASE WHEN trigger_authority_seq>$4 THEN 'pending' ELSE $5 END,due_at=CASE WHEN trigger_authority_seq>$4 THEN due_at ELSE $6 END,lease_token=NULL,lease_until=NULL,last_problem_code=$7,updated_at=$8 WHERE subject_id=$1 AND need_id=$2 AND state='leased' AND lease_token=$3 AND lease_until>clock_timestamp()",
        )
        .bind(claimed.subject_id.0)
        .bind(claimed.need_id)
        .bind(token)
        .bind(claimed.trigger_authority_seq)
        .bind(state)
        .bind(due)
        .bind(problem)
        .bind(self.now(claimed.subject_id))
        .execute(self.store.pool())
        .await
        .map_err(db)?;
        if updated.rows_affected() == 0 {
            return Err(Error::Conflict(
                "maintenance lease expired or replaced".into(),
            ));
        }
        Ok(())
    }

    pub async fn maintenance_needs(&self, subject: SubjectId) -> Result<Vec<MaintenanceNeed>> {
        self.require_subject(subject).await?;
        sqlx::query("SELECT * FROM maintenance_needs WHERE subject_id=$1 AND state IN ('pending','leased') ORDER BY priority DESC,due_at,need_id LIMIT 128")
            .bind(subject.0)
            .fetch_all(self.store.pool())
            .await
            .map_err(db)?
            .into_iter()
            .map(need_from_row)
            .collect()
    }
}

fn valid_identifier(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn need_from_row(row: sqlx::postgres::PgRow) -> Result<MaintenanceNeed> {
    Ok(MaintenanceNeed {
        need_id: row.try_get("need_id").map_err(db)?,
        subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
        kind: row.try_get("kind").map_err(db)?,
        scope_kind: row.try_get("scope_kind").map_err(db)?,
        scope_ref: row.try_get("scope_ref").map_err(db)?,
        trigger_authority_seq: row.try_get("trigger_authority_seq").map_err(db)?,
        due_at: row.try_get("due_at").map_err(db)?,
        priority: row.try_get("priority").map_err(db)?,
        state: row.try_get("state").map_err(db)?,
        lease_token: row.try_get("lease_token").map_err(db)?,
        lease_until: row.try_get("lease_until").map_err(db)?,
        attempt_count: row.try_get("attempt_count").map_err(db)?,
        last_problem_code: row.try_get("last_problem_code").map_err(db)?,
        created_at: row.try_get("created_at").map_err(db)?,
        updated_at: row.try_get("updated_at").map_err(db)?,
    })
}
