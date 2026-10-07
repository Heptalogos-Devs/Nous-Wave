// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use crate::*;
use chrono::{DateTime, Utc};
use nous_core::{CognitiveRef, OperationId, SubjectId};
use nous_persistence::{
    MutationReceipt, check_receipt, commit_receipt, database_error as db, lock_operation,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkContextState {
    Open,
    Paused,
    Ended,
}

impl WorkContextState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Paused => "paused",
            Self::Ended => "ended",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkContextView {
    pub work_context_id: Uuid,
    pub subject_id: SubjectId,
    pub state: WorkContextState,
    pub purpose: String,
    pub unresolved_questions: Vec<String>,
    pub constraints: serde_json::Value,
    pub resume_conditions: Vec<String>,
    pub budget_summary: serde_json::Value,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub cognition_anchors: Vec<CognitiveRef>,
    pub context_text: String,
    pub entity_anchors: Vec<EntityRef>,
    pub tag_anchors: Vec<TagId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkContextInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub purpose: String,
    #[serde(default)]
    pub unresolved_questions: Vec<String>,
    #[serde(default)]
    pub constraints: serde_json::Value,
    #[serde(default)]
    pub resume_conditions: Vec<String>,
    #[serde(default)]
    pub budget_summary: serde_json::Value,
    #[serde(default)]
    pub cognition_anchors: Vec<CognitiveRef>,
    pub context_text: String,
    pub entity_anchors: Vec<EntityRef>,
    pub tag_anchors: Vec<TagId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkContextInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub work_context_id: Uuid,
    pub expected_revision: i64,
    pub purpose: String,
    pub unresolved_questions: Vec<String>,
    pub constraints: serde_json::Value,
    pub resume_conditions: Vec<String>,
    pub budget_summary: serde_json::Value,
    pub cognition_anchors: Vec<CognitiveRef>,
    pub context_text: String,
    pub entity_anchors: Vec<EntityRef>,
    pub tag_anchors: Vec<TagId>,
}

#[derive(Debug, Clone, Copy)]
enum WorkContextTransition {
    Pause,
    Resume,
    End,
}

impl CognitiveRuntimeService {
    pub async fn create_work_context(
        &self,
        input: CreateWorkContextInput,
    ) -> Result<WorkContextView> {
        validate_payload(
            &input.purpose,
            &input.unresolved_questions,
            &input.constraints,
            &input.resume_conditions,
            &input.budget_summary,
            &input.cognition_anchors,
        )?;
        validate_anchors(
            &input.context_text,
            &input.entity_anchors,
            &input.tag_anchors,
        )?;
        self.store.require_subject(input.subject).await?;
        let digest = request_digest(
            "create_work_context",
            input.subject,
            &serde_json::json!({
                "purpose": input.purpose,
                "unresolved_questions": input.unresolved_questions,
                "constraints": input.constraints,
                "resume_conditions": input.resume_conditions,
                "budget_summary": input.budget_summary,
                "cognition_anchors": input.cognition_anchors,
                "context_text": input.context_text,
                "entity_anchors": input.entity_anchors,
                "tag_anchors": input.tag_anchors,
            }),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(existing) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "create_work_context",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            return self
                .work_context(input.subject, replay_context(existing)?)
                .await;
        }
        let work_context_id = Uuid::now_v7();
        validate_refs_in_tx(
            &self.store,
            &mut tx,
            input.subject,
            &input.cognition_anchors,
        )
        .await?;
        let typed = typed_anchors(&input.entity_anchors, &input.tag_anchors);
        validate_refs_in_tx(&self.store, &mut tx, input.subject, &typed).await?;
        let now = self.now(input.subject);
        sqlx::query(
            "INSERT INTO work_contexts(work_context_id,subject_id,state,purpose,unresolved_questions,constraints,resume_conditions,budget_summary,context_text,revision,created_at,updated_at) VALUES($1,$2,'open',$3,$4,$5,$6,$7,$9,1,$8,$8)",
        )
        .bind(work_context_id)
        .bind(input.subject.0)
        .bind(&input.purpose)
        .bind(&input.unresolved_questions)
        .bind(&input.constraints)
        .bind(&input.resume_conditions)
        .bind(&input.budget_summary)
        .bind(now)
        .bind(&input.context_text)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        insert_refs(&mut tx, work_context_id, &input.cognition_anchors).await?;
        insert_typed_anchors(&mut tx, work_context_id, &typed).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "work_context",
            Some(&work_context_id.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.work_context(input.subject, work_context_id).await
    }

    pub async fn work_context(
        &self,
        subject: SubjectId,
        work_context_id: Uuid,
    ) -> Result<WorkContextView> {
        let mut tx = self.store.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let context = self
            .work_context_in(subject, work_context_id, &mut tx)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(context)
    }

    pub(crate) async fn work_context_in(
        &self,
        subject: SubjectId,
        work_context_id: Uuid,
        connection: &mut sqlx::PgConnection,
    ) -> Result<WorkContextView> {
        let row =
            sqlx::query("SELECT * FROM work_contexts WHERE subject_id=$1 AND work_context_id=$2")
                .bind(subject.0)
                .bind(work_context_id)
                .fetch_optional(&mut *connection)
                .await
                .map_err(db)?
                .ok_or_else(|| Error::NotFound("WorkContext not found".into()))?;
        self.work_context_from_row(row, connection).await
    }

    pub async fn list_work_contexts(&self, subject: SubjectId) -> Result<Vec<WorkContextView>> {
        self.store.require_subject(subject).await?;
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT work_context_id FROM work_contexts WHERE subject_id=$1 ORDER BY created_at,work_context_id",
        )
        .bind(subject.0)
        .fetch_all(self.store.pool())
        .await
        .map_err(db)?;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            result.push(self.work_context(subject, id).await?);
        }
        Ok(result)
    }

    pub async fn update_work_context(
        &self,
        input: UpdateWorkContextInput,
    ) -> Result<WorkContextView> {
        validate_payload(
            &input.purpose,
            &input.unresolved_questions,
            &input.constraints,
            &input.resume_conditions,
            &input.budget_summary,
            &input.cognition_anchors,
        )?;
        validate_anchors(
            &input.context_text,
            &input.entity_anchors,
            &input.tag_anchors,
        )?;
        self.store.require_subject(input.subject).await?;
        let digest = request_digest(
            "update_work_context",
            input.subject,
            &serde_json::json!({
                "work_context_id": input.work_context_id,
                "expected_revision": input.expected_revision,
                "purpose": input.purpose,
                "unresolved_questions": input.unresolved_questions,
                "constraints": input.constraints,
                "resume_conditions": input.resume_conditions,
                "budget_summary": input.budget_summary,
                "cognition_anchors": input.cognition_anchors,
                "context_text": input.context_text,
                "entity_anchors": input.entity_anchors,
                "tag_anchors": input.tag_anchors,
            }),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(existing) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "update_work_context",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            return self
                .work_context(input.subject, replay_context(existing)?)
                .await;
        }
        let row = sqlx::query(
            "SELECT state,revision FROM work_contexts WHERE subject_id=$1 AND work_context_id=$2 FOR UPDATE",
        )
        .bind(input.subject.0)
        .bind(input.work_context_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::NotFound("WorkContext not found".into()))?;
        let state: String = row.try_get("state").map_err(db)?;
        let revision: i64 = row.try_get("revision").map_err(db)?;
        if state == WorkContextState::Ended.as_str() {
            return Err(Error::FailedPrecondition("WorkContext is ended".into()));
        }
        if revision != input.expected_revision {
            return Err(Error::Conflict("WorkContext revision is stale".into()));
        }
        validate_refs_in_tx(
            &self.store,
            &mut tx,
            input.subject,
            &input.cognition_anchors,
        )
        .await?;
        let typed = typed_anchors(&input.entity_anchors, &input.tag_anchors);
        validate_refs_in_tx(&self.store, &mut tx, input.subject, &typed).await?;
        let now = self.now(input.subject);
        sqlx::query(
            "UPDATE work_contexts SET purpose=$3,unresolved_questions=$4,constraints=$5,resume_conditions=$6,budget_summary=$7,context_text=$9,revision=revision+1,updated_at=$8 WHERE subject_id=$1 AND work_context_id=$2",
        )
        .bind(input.subject.0)
        .bind(input.work_context_id)
        .bind(&input.purpose)
        .bind(&input.unresolved_questions)
        .bind(&input.constraints)
        .bind(&input.resume_conditions)
        .bind(&input.budget_summary)
        .bind(now)
        .bind(&input.context_text)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("DELETE FROM work_context_refs WHERE work_context_id=$1")
            .bind(input.work_context_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        insert_refs(&mut tx, input.work_context_id, &input.cognition_anchors).await?;
        sqlx::query("DELETE FROM work_context_anchors WHERE work_context_id=$1")
            .bind(input.work_context_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        insert_typed_anchors(&mut tx, input.work_context_id, &typed).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "work_context",
            Some(&input.work_context_id.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.work_context(input.subject, input.work_context_id)
            .await
    }

    pub async fn pause_work_context(
        &self,
        subject: SubjectId,
        work_context_id: Uuid,
        expected_revision: i64,
        operation_id: OperationId,
    ) -> Result<WorkContextView> {
        self.transition_work_context(
            subject,
            work_context_id,
            expected_revision,
            operation_id,
            WorkContextTransition::Pause,
        )
        .await
    }

    pub async fn resume_work_context(
        &self,
        subject: SubjectId,
        work_context_id: Uuid,
        expected_revision: i64,
        operation_id: OperationId,
    ) -> Result<WorkContextView> {
        self.transition_work_context(
            subject,
            work_context_id,
            expected_revision,
            operation_id,
            WorkContextTransition::Resume,
        )
        .await
    }

    pub async fn end_work_context(
        &self,
        subject: SubjectId,
        work_context_id: Uuid,
        expected_revision: i64,
        operation_id: OperationId,
    ) -> Result<WorkContextView> {
        self.transition_work_context(
            subject,
            work_context_id,
            expected_revision,
            operation_id,
            WorkContextTransition::End,
        )
        .await
    }

    async fn transition_work_context(
        &self,
        subject: SubjectId,
        work_context_id: Uuid,
        expected_revision: i64,
        operation_id: OperationId,
        transition: WorkContextTransition,
    ) -> Result<WorkContextView> {
        let kind = match transition {
            WorkContextTransition::Pause => "pause_work_context",
            WorkContextTransition::Resume => "resume_work_context",
            WorkContextTransition::End => "end_work_context",
        };
        let digest = request_digest(
            kind,
            subject,
            &serde_json::json!({
                "work_context_id": work_context_id,
                "expected_revision": expected_revision,
            }),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(existing) = check_receipt(&mut tx, subject, operation_id, kind, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            return self.work_context(subject, replay_context(existing)?).await;
        }
        let row = sqlx::query(
            "SELECT state,revision FROM work_contexts WHERE subject_id=$1 AND work_context_id=$2 FOR UPDATE",
        )
        .bind(subject.0)
        .bind(work_context_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::NotFound("WorkContext not found".into()))?;
        let state: String = row.try_get("state").map_err(db)?;
        let revision: i64 = row.try_get("revision").map_err(db)?;
        if revision != expected_revision {
            return Err(Error::Conflict("WorkContext revision is stale".into()));
        }
        let next = match transition {
            WorkContextTransition::Pause if state == "open" => "paused",
            WorkContextTransition::Resume if state == "paused" => "open",
            WorkContextTransition::End if matches!(state.as_str(), "open" | "paused") => "ended",
            WorkContextTransition::Pause if state == "paused" => {
                return Err(Error::FailedPrecondition(
                    "WorkContext is already paused".into(),
                ));
            }
            WorkContextTransition::Resume if state == "ended" => {
                return Err(Error::FailedPrecondition(
                    "ended WorkContext cannot resume".into(),
                ));
            }
            WorkContextTransition::End if state == "ended" => {
                return Err(Error::FailedPrecondition(
                    "WorkContext is already ended".into(),
                ));
            }
            _ => {
                return Err(Error::FailedPrecondition(
                    "invalid WorkContext transition".into(),
                ));
            }
        };
        if matches!(
            transition,
            WorkContextTransition::Pause | WorkContextTransition::End
        ) {
            let sessions = sqlx::query_scalar::<_, Uuid>(
                "SELECT session_id FROM cognitive_sessions WHERE subject_id=$1 AND active_work_context_id=$2 ORDER BY session_id FOR UPDATE",
            )
            .bind(subject.0)
            .bind(work_context_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for session in sessions {
                sqlx::query(
                    "UPDATE cognitive_sessions SET active_work_context_id=NULL,runtime_revision=runtime_revision+1,last_activity_at=$3 WHERE subject_id=$1 AND session_id=$2",
                )
                .bind(subject.0)
                .bind(session)
                .bind(self.now(subject))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let ended_at = (next == "ended").then(|| self.now(subject));
        sqlx::query(
            "UPDATE work_contexts SET state=$3,revision=revision+1,updated_at=$4,ended_at=$5 WHERE subject_id=$1 AND work_context_id=$2",
        )
        .bind(subject.0)
        .bind(work_context_id)
        .bind(next)
        .bind(self.now(subject))
        .bind(ended_at)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if next == "ended" {
            self.wake_ended_context_in(&mut tx, subject, work_context_id)
                .await?;
        }
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "work_context",
            Some(&work_context_id.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.work_context(subject, work_context_id).await
    }

    pub async fn set_active_work_context(
        &self,
        subject: SubjectId,
        session: SessionId,
        work_context_id: Option<Uuid>,
        expected_runtime_revision: i64,
        operation_id: OperationId,
    ) -> Result<SessionView> {
        let digest = request_digest(
            "set_active_work_context",
            subject,
            &serde_json::json!({
                "session_id": session,
                "work_context_id": work_context_id,
                "expected_runtime_revision": expected_runtime_revision,
            }),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(existing) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "set_active_work_context",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if existing.state != "committed" {
                return Err(Error::Unavailable(
                    "foreground WorkContext operation is already in progress".into(),
                ));
            }
            return self.session(subject, session).await;
        }
        let row = sqlx::query(
            "SELECT subject_id,closed_at,runtime_revision FROM cognitive_sessions WHERE session_id=$1 FOR UPDATE",
        )
        .bind(session.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::FailedPrecondition("session is closed or not found".into()))?;
        let owner: Uuid = row.try_get("subject_id").map_err(db)?;
        let closed: Option<DateTime<Utc>> = row.try_get("closed_at").map_err(db)?;
        let current: i64 = row.try_get("runtime_revision").map_err(db)?;
        if owner != subject.0 || closed.is_some() {
            return Err(Error::FailedPrecondition(
                "session is closed or not owned by Subject".into(),
            ));
        }
        if current != expected_runtime_revision {
            return Err(Error::Conflict("expected runtime revision is stale".into()));
        }
        if let Some(work_context_id) = work_context_id {
            let context = sqlx::query(
                "SELECT subject_id,state FROM work_contexts WHERE subject_id=$1 AND work_context_id=$2 FOR SHARE",
            )
            .bind(subject.0)
            .bind(work_context_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("WorkContext not found".into()))?;
            if context.try_get::<String, _>("state").map_err(db)? != "open" {
                return Err(Error::FailedPrecondition(
                    "only an open WorkContext can be foreground".into(),
                ));
            }
        }
        sqlx::query(
            "UPDATE cognitive_sessions SET active_work_context_id=$3,runtime_revision=runtime_revision+1,last_activity_at=$4 WHERE subject_id=$1 AND session_id=$2",
        )
        .bind(subject.0)
        .bind(session.0)
        .bind(work_context_id)
        .bind(self.now(subject))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "work_context",
            None,
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.session(subject, session).await
    }

    pub async fn active_work_context_refs(
        &self,
        subject: SubjectId,
        session: SessionId,
    ) -> Result<Vec<CognitiveRef>> {
        let rows = sqlx::query(
            "SELECT r.ref_kind,r.ref_value FROM cognitive_sessions s JOIN work_context_refs r ON r.work_context_id=s.active_work_context_id JOIN work_contexts w ON w.work_context_id=r.work_context_id WHERE s.subject_id=$1 AND s.session_id=$2 AND s.closed_at IS NULL AND w.state='open' ORDER BY r.ordinal",
        )
        .bind(subject.0)
        .bind(session.0)
        .fetch_all(self.store.pool())
        .await
        .map_err(db)?;
        rows.into_iter()
            .map(|row| {
                parse_reference(
                    &row.try_get::<String, _>("ref_kind").map_err(db)?,
                    &row.try_get::<String, _>("ref_value").map_err(db)?,
                )
            })
            .collect()
    }

    async fn work_context_from_row(
        &self,
        row: sqlx::postgres::PgRow,
        executor: &mut sqlx::PgConnection,
    ) -> Result<WorkContextView> {
        let id: Uuid = row.try_get("work_context_id").map_err(db)?;
        let refs = sqlx::query(
            "SELECT ref_kind,ref_value FROM work_context_refs WHERE work_context_id=$1 ORDER BY ordinal",
        )
        .bind(id)
        .fetch_all(&mut *executor)
        .await
        .map_err(db)?;
        let cognition_anchors = refs
            .into_iter()
            .map(|value| {
                parse_reference(
                    &value.try_get::<String, _>("ref_kind").map_err(db)?,
                    &value.try_get::<String, _>("ref_value").map_err(db)?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let anchors = sqlx::query("SELECT ref_kind,ref_value FROM work_context_anchors WHERE work_context_id=$1 ORDER BY ordinal").bind(id).fetch_all(&mut *executor).await.map_err(db)?;
        let mut entity_anchors = Vec::new();
        let mut tag_anchors = Vec::new();
        for anchor in anchors {
            match parse_reference(
                &anchor.try_get::<String, _>("ref_kind").map_err(db)?,
                &anchor.try_get::<String, _>("ref_value").map_err(db)?,
            )? {
                CognitiveRef::Entity(entity) => entity_anchors.push(entity),
                CognitiveRef::Tag(tag) => tag_anchors.push(tag),
                _ => {
                    return Err(Error::Infrastructure(
                        "invalid typed WorkContext anchor".into(),
                    ));
                }
            }
        }
        Ok(WorkContextView {
            work_context_id: id,
            subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
            state: parse_state(row.try_get("state").map_err(db)?)?,
            purpose: row.try_get("purpose").map_err(db)?,
            unresolved_questions: row.try_get("unresolved_questions").map_err(db)?,
            constraints: row.try_get("constraints").map_err(db)?,
            resume_conditions: row.try_get("resume_conditions").map_err(db)?,
            budget_summary: row.try_get("budget_summary").map_err(db)?,
            revision: row.try_get("revision").map_err(db)?,
            created_at: row.try_get("created_at").map_err(db)?,
            updated_at: row.try_get("updated_at").map_err(db)?,
            ended_at: row.try_get("ended_at").map_err(db)?,
            cognition_anchors,
            context_text: row.try_get("context_text").map_err(db)?,
            entity_anchors,
            tag_anchors,
        })
    }

    pub async fn runtime_references(
        &self,
        subject: SubjectId,
        session: SessionId,
    ) -> Result<Vec<(CognitiveRef, &'static str)>> {
        let session_view = self.session(subject, session).await?;
        let mut refs = session_view
            .resident
            .into_iter()
            .map(|value| (value.reference, "runtime_resident"))
            .collect::<Vec<_>>();
        refs.extend(
            self.active_work_context_refs(subject, session)
                .await?
                .into_iter()
                .map(|value| (value, "runtime_work_context")),
        );
        let mut seen = BTreeSet::new();
        refs.retain(|(reference, _)| seen.insert(reference.to_string()));
        Ok(refs)
    }
}

fn parse_state(value: String) -> Result<WorkContextState> {
    match value.as_str() {
        "open" => Ok(WorkContextState::Open),
        "paused" => Ok(WorkContextState::Paused),
        "ended" => Ok(WorkContextState::Ended),
        _ => Err(Error::Infrastructure("invalid WorkContext state".into())),
    }
}

fn validate_payload(
    purpose: &str,
    questions: &[String],
    constraints: &serde_json::Value,
    resume_conditions: &[String],
    budget: &serde_json::Value,
    cognition_anchors: &[CognitiveRef],
) -> Result<()> {
    if purpose.is_empty() || purpose.len() > 8192 {
        return Err(Error::Invalid(
            "WorkContext purpose is out of bounds".into(),
        ));
    }
    validate_text_array("unresolved_questions", questions)?;
    validate_text_array("resume_conditions", resume_conditions)?;
    if serde_json::to_vec(constraints)
        .map_err(|error| Error::Invalid(error.to_string()))?
        .len()
        > 65_536
    {
        return Err(Error::Invalid(
            "WorkContext constraints are too large".into(),
        ));
    }
    if serde_json::to_vec(budget)
        .map_err(|error| Error::Invalid(error.to_string()))?
        .len()
        > 16_384
    {
        return Err(Error::Invalid(
            "WorkContext budget summary is too large".into(),
        ));
    }
    if cognition_anchors.len() > 256 {
        return Err(Error::Invalid(
            "WorkContext reference bound exceeded".into(),
        ));
    }
    let mut keys = BTreeSet::new();
    for reference in cognition_anchors {
        if !matches!(
            reference,
            CognitiveRef::MemoryRevision(_)
                | CognitiveRef::CognitiveSchemaRevision(_)
                | CognitiveRef::Occurrence(_)
                | CognitiveRef::EpisodeRevision(_)
                | CognitiveRef::JournalRevision(_)
        ) {
            return Err(Error::Invalid(
                "WorkContext cognition_anchors must be exact continuation refs".into(),
            ));
        }
        if !keys.insert(reference.to_string()) {
            return Err(Error::Invalid("duplicate WorkContext reference".into()));
        }
    }
    Ok(())
}

fn validate_text_array(name: &str, values: &[String]) -> Result<()> {
    if values.len() > 64
        || values
            .iter()
            .any(|value| value.is_empty() || value.len() > 2048)
    {
        return Err(Error::Invalid(format!("{name} is out of bounds")));
    }
    Ok(())
}

async fn validate_refs_in_tx(
    store: &nous_persistence::AuthorityStore,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    cognition_anchors: &[CognitiveRef],
) -> Result<()> {
    for reference in cognition_anchors {
        if !store
            .reference_in_subject_tx(tx, subject, reference)
            .await?
        {
            return Err(Error::FailedPrecondition(
                "WorkContext reference is not owned by Subject".into(),
            ));
        }
    }
    Ok(())
}

async fn insert_refs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    work_context_id: Uuid,
    cognition_anchors: &[CognitiveRef],
) -> Result<()> {
    for (ordinal, reference) in cognition_anchors.iter().enumerate() {
        let (kind, value) = reference_parts(reference);
        sqlx::query(
            "INSERT INTO work_context_refs(work_context_id,ordinal,ref_kind,ref_value) VALUES($1,$2,$3,$4)",
        )
        .bind(work_context_id)
        .bind(ordinal as i32)
        .bind(kind)
        .bind(value)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    Ok(())
}

fn request_digest<T: Serialize>(kind: &str, subject: SubjectId, value: &T) -> Result<String> {
    nous_core::canonical_request_digest(kind, subject, value)
}

fn replay_context(receipt: MutationReceipt) -> Result<Uuid> {
    if receipt.state != "committed" {
        return Err(Error::Unavailable(
            "WorkContext operation is already in progress".into(),
        ));
    }
    receipt
        .result_ref
        .ok_or_else(|| Error::Infrastructure("WorkContext receipt has no result".into()))?
        .parse()
        .map_err(|_| Error::Infrastructure("invalid WorkContext receipt".into()))
}

impl CognitiveRuntimeService {
    async fn wake_ended_context_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        context: Uuid,
    ) -> Result<()> {
        let sequence:Option<i64>=sqlx::query_scalar("SELECT authority_seq FROM subjects WHERE subject_id=$1 AND EXISTS(SELECT 1 FROM experience_items WHERE subject_id=$1 AND active_work_context_id=$2)")
            .bind(subject.0).bind(context).fetch_optional(&mut **tx).await.map_err(db)?;
        if let Some(sequence) = sequence {
            for kind in ["episode_segment", "journal_review"] {
                self.enqueue_maintenance_in(
                    tx,
                    &MaintenanceRequest {
                        subject,
                        kind: kind.into(),
                        scope_kind: "track".into(),
                        scope_ref: "interaction".into(),
                        trigger_authority_seq: sequence,
                        due_at: self.now(subject),
                        priority: 60,
                    },
                )
                .await?;
            }
        }
        Ok(())
    }
}

fn typed_anchors(entities: &[EntityRef], tags: &[TagId]) -> Vec<CognitiveRef> {
    entities
        .iter()
        .cloned()
        .map(CognitiveRef::Entity)
        .chain(tags.iter().copied().map(CognitiveRef::Tag))
        .collect()
}
fn validate_anchors(text: &str, entities: &[EntityRef], tags: &[TagId]) -> Result<()> {
    if text.len() > 65536 || entities.len() > 256 || tags.len() > 256 {
        return Err(Error::Invalid(
            "WorkContext typed context bounds exceeded".into(),
        ));
    }
    let anchors = typed_anchors(entities, tags);
    let unique = anchors
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    if unique.len() != anchors.len() {
        return Err(Error::Invalid("duplicate WorkContext anchor".into()));
    }
    Ok(())
}
async fn insert_typed_anchors(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    anchors: &[CognitiveRef],
) -> Result<()> {
    for (ordinal, reference) in anchors.iter().enumerate() {
        let (kind, value) = reference_parts(reference);
        sqlx::query("INSERT INTO work_context_anchors(work_context_id,ordinal,ref_kind,ref_value) VALUES($1,$2,$3,$4)")
            .bind(id).bind(ordinal as i32).bind(kind).bind(value).execute(&mut **tx).await.map_err(db)?;
    }
    Ok(())
}
