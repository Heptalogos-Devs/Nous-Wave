use super::*;
use chrono::{DateTime, Utc};
use nous_core::{CognitiveRef, EpisodeId, EpisodeRevisionId, OperationId, SubjectId};
use nous_persistence::database_error as db;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeSet;
use uuid::Uuid;
mod partition;
pub use partition::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeMemberInput {
    pub reference: CognitiveRef,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub track_key: String,
    pub title: Option<String>,
    pub parent_episode_revision_id: Option<EpisodeRevisionId>,
    pub experience_time: TemporalExtent,
    pub boundary_explanation: String,

    pub producer_signature_id: Option<Uuid>,
    pub members: Vec<EpisodeMemberInput>,
    pub supports: Vec<RevisionSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseEpisodeInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub episode_id: EpisodeId,
    pub expected_object_epoch: i64,
    pub intent: String,
    pub title: Option<String>,
    pub parent_episode_revision_id: Option<EpisodeRevisionId>,
    pub experience_time: TemporalExtent,
    pub boundary_explanation: String,

    pub producer_signature_id: Option<Uuid>,
    pub members: Vec<EpisodeMemberInput>,
    pub supports: Vec<RevisionSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeObject {
    pub episode_id: EpisodeId,
    pub subject_id: SubjectId,
    pub track_key: String,
    pub current_revision_id: EpisodeRevisionId,
    pub object_epoch: i64,
    pub acceptance_state: AcceptanceState,
    pub integrity_state: IntegrityState,
    pub suppression_state: SuppressionState,
    pub purge_state: PurgeState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRevision {
    pub episode_revision_id: EpisodeRevisionId,
    pub episode_id: EpisodeId,
    pub subject_id: SubjectId,
    pub revision_no: i32,
    pub parent_revision_id: Option<EpisodeRevisionId>,
    pub revision_intent: Option<String>,
    pub title: Option<String>,
    pub parent_episode_revision_id: Option<EpisodeRevisionId>,
    pub experience_time: TemporalExtent,
    pub boundary_explanation: String,
    pub formed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeMember {
    pub ordinal: i32,
    pub reference: CognitiveRef,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRelation {
    pub from_revision_id: EpisodeRevisionId,
    pub to_revision_id: EpisodeRevisionId,
    pub relation: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeView {
    pub object: EpisodeObject,
    pub revision: EpisodeRevision,
    pub members: Vec<EpisodeMember>,
    pub supports: Vec<RevisionSupport>,
    pub relations: Vec<EpisodeRelation>,
}

impl MemoryService {
    pub async fn create_episode(&self, input: EpisodeInput) -> Result<EpisodeView> {
        let started_at = self.cognition.now(input.subject);
        validate_episode_input(
            &input.track_key,
            &input.title,
            &input.experience_time,
            &input.boundary_explanation,
            &input.members,
            &input.supports,
        )?;
        self.store.require_subject(input.subject).await?;
        let digest = operation_digest(
            "create_episode",
            input.subject,
            &serde_json::json!({
                "track_key": input.track_key,
                "title": input.title,
                "parent": input.parent_episode_revision_id,
                "experience_time": input.experience_time,
                "boundary_explanation": input.boundary_explanation,
                "producer_signature_id": input.producer_signature_id,
                "members": input.members,
                "supports": input.supports,
            }),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "create_episode",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                let id = receipt
                    .result_ref
                    .ok_or_else(|| Error::Infrastructure("Episode receipt has no result".into()))?
                    .parse()
                    .map_err(|_| Error::Infrastructure("invalid Episode receipt".into()))?;
                return self
                    .episode(
                        input.subject,
                        EpisodeId(id),
                        receipt.result_revision.map(EpisodeRevisionId),
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "create Episode operation is already in progress".into(),
            ));
        }
        validate_episode_refs_in_tx(&self.store, &mut tx, input.subject, &input.members).await?;
        validate_episode_supports_in_tx(&self.store, &mut tx, input.subject, &input.supports)
            .await?;
        validate_parent_and_overlap(
            &mut tx,
            input.subject,
            &input.track_key,
            input.parent_episode_revision_id,
            &input.experience_time,
            None,
        )
        .await?;
        let episode_id = EpisodeId::new();
        let revision_id = EpisodeRevisionId::new();
        let formed_at = self
            .formation_time_in(&mut tx, input.subject, input.operation_id, started_at)
            .await?;
        let now = self.cognition.now(input.subject);
        let (time_kind, time_start, time_end) = temporal_columns(&input.experience_time);
        sqlx::query("INSERT INTO episode_objects(episode_id,subject_id,track_key,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,$4,1,'accepted','valid','normal','normal',$5)")
            .bind(episode_id.0).bind(input.subject.0).bind(&input.track_key).bind(revision_id.0).bind(now)
            .execute(&mut *tx).await.map_err(db)?;
        insert_episode_revision(
            &mut tx,
            &input,
            episode_id,
            revision_id,
            None,
            None,
            time_kind,
            time_start,
            time_end,
            formed_at,
            now,
        )
        .await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "episode",
            Some(&episode_id.0.to_string()),
            Some(revision_id.0),
            Some(1),
        )
        .await?;
        let authority_seq =
            AuthorityStore::invalidate_in(&mut tx, input.subject, ProjectionInvalidation::text())
                .await?;
        self.schedule_episode_in(
            &mut tx,
            input.subject,
            revision_id,
            &input.track_key,
            authority_seq,
            true,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.episode(input.subject, episode_id, None).await
    }

    async fn schedule_episode_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        revision: EpisodeRevisionId,
        track: &str,
        authority_seq: i64,
        semantic_review: bool,
    ) -> Result<()> {
        let delay = self
            .configuration
            .snapshot_for_subject(subject)?
            .get(nous_runtime::SETTLE_DELAY_KEY)?;
        let now = self.cognition.now(subject);
        let due_at = now + chrono::Duration::seconds(delay as i64);
        let consolidation_due = now
            + chrono::Duration::seconds(
                self.configuration
                    .snapshot_for_subject(subject)?
                    .get(super::longitudinal_policy::CONSOLIDATION_DELAY)? as i64,
            );
        for (kind, scope_kind, scope_ref) in [
            (
                "episode_resegment",
                "episode_revision",
                revision.0.to_string(),
            ),
            ("journal_review", "track", track.to_owned()),
            (
                "memory_consolidate",
                "episode_revision",
                revision.0.to_string(),
            ),
        ] {
            if kind == "episode_resegment" && !semantic_review {
                continue;
            }
            self.cognition
                .enqueue_maintenance_in(
                    tx,
                    &nous_runtime::MaintenanceRequest {
                        subject,
                        kind: kind.into(),
                        scope_kind: scope_kind.into(),
                        scope_ref,
                        trigger_authority_seq: authority_seq,
                        due_at: if kind == "memory_consolidate" {
                            consolidation_due
                        } else {
                            due_at
                        },
                        priority: 30,
                    },
                )
                .await?;
        }
        Ok(())
    }

    pub async fn episode(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        revision: Option<EpisodeRevisionId>,
    ) -> Result<EpisodeView> {
        let row = sqlx::query("SELECT o.episode_id,o.subject_id,o.track_key,o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state,o.created_at,r.episode_revision_id,r.revision_no,r.parent_revision_id,r.revision_intent,r.title,r.parent_episode_revision_id,r.experience_time_kind,r.experience_time_start,r.experience_time_end,r.boundary_explanation,r.formed_at,r.recorded_at,r.producer_signature_id FROM episode_objects o JOIN episode_revisions r ON r.episode_id=o.episode_id AND r.episode_revision_id=COALESCE($3,o.current_revision_id) WHERE o.subject_id=$1 AND o.episode_id=$2")
            .bind(subject.0).bind(episode.0).bind(revision.map(|value| value.0))
            .fetch_optional(self.store.pool()).await.map_err(db)?
            .ok_or_else(|| Error::NotFound("Episode not found".into()))?;
        let revision_id: Uuid = row.try_get("episode_revision_id").map_err(db)?;
        let members = load_members(self.store.pool(), revision_id).await?;
        let supports = load_supports(self.store.pool(), revision_id).await?;
        let relations = sqlx::query("SELECT from_revision_id,to_revision_id,relation,created_at FROM episode_revision_relations WHERE from_revision_id=$1 OR to_revision_id=$1 ORDER BY created_at")
            .bind(revision_id).fetch_all(self.store.pool()).await.map_err(db)?
            .into_iter().map(|row| Ok(EpisodeRelation {
                from_revision_id: EpisodeRevisionId(row.try_get("from_revision_id").map_err(db)?),
                to_revision_id: EpisodeRevisionId(row.try_get("to_revision_id").map_err(db)?),
                relation: row.try_get("relation").map_err(db)?,
                created_at: row.try_get("created_at").map_err(db)?,
            })).collect::<Result<Vec<_>>>()?;
        Ok(EpisodeView {
            object: EpisodeObject {
                episode_id: EpisodeId(row.try_get("episode_id").map_err(db)?),
                subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
                track_key: row.try_get("track_key").map_err(db)?,
                current_revision_id: EpisodeRevisionId(
                    row.try_get("current_revision_id").map_err(db)?,
                ),
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
            revision: EpisodeRevision {
                episode_revision_id: EpisodeRevisionId(revision_id),
                episode_id: EpisodeId(row.try_get("episode_id").map_err(db)?),
                subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(EpisodeRevisionId),
                revision_intent: row.try_get("revision_intent").map_err(db)?,
                title: row.try_get("title").map_err(db)?,
                parent_episode_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_episode_revision_id")
                    .map_err(db)?
                    .map(EpisodeRevisionId),
                experience_time: temporal_from_columns(
                    row.try_get("experience_time_kind").map_err(db)?,
                    row.try_get("experience_time_start").map_err(db)?,
                    row.try_get("experience_time_end").map_err(db)?,
                )?,
                boundary_explanation: row.try_get("boundary_explanation").map_err(db)?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
            },
            members,
            supports,
            relations,
        })
    }

    pub async fn episode_revision(
        &self,
        subject: SubjectId,
        revision: EpisodeRevisionId,
    ) -> Result<EpisodeView> {
        let episode: Uuid = sqlx::query_scalar("SELECT episode_id FROM episode_revisions WHERE subject_id=$1 AND episode_revision_id=$2")
            .bind(subject.0).bind(revision.0).fetch_optional(self.store.pool()).await.map_err(db)?
            .ok_or_else(|| Error::NotFound("Episode revision not found".into()))?;
        self.episode(subject, EpisodeId(episode), Some(revision))
            .await
    }

    pub async fn list_episodes(&self, subject: SubjectId) -> Result<Vec<EpisodeView>> {
        let ids = sqlx::query_scalar::<_, Uuid>("SELECT episode_id FROM episode_objects WHERE subject_id=$1 ORDER BY created_at,episode_id")
            .bind(subject.0).fetch_all(self.store.pool()).await.map_err(db)?;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            result.push(self.episode(subject, EpisodeId(id), None).await?);
        }
        Ok(result)
    }

    pub async fn episode_history(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
    ) -> Result<Vec<EpisodeView>> {
        let ids = sqlx::query_scalar::<_, Uuid>("SELECT episode_revision_id FROM episode_revisions WHERE subject_id=$1 AND episode_id=$2 ORDER BY revision_no")
            .bind(subject.0).bind(episode.0).fetch_all(self.store.pool()).await.map_err(db)?;
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            result.push(
                self.episode(subject, episode, Some(EpisodeRevisionId(id)))
                    .await?,
            );
        }
        Ok(result)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Episode revision keeps fencing, hierarchy, members, supports, and receipt commit together"
    )]
    pub async fn revise_episode(&self, input: ReviseEpisodeInput) -> Result<EpisodeView> {
        let started_at = self.cognition.now(input.subject);
        validate_episode_input(
            "",
            &input.title,
            &input.experience_time,
            &input.boundary_explanation,
            &input.members,
            &input.supports,
        )?;
        if !matches!(input.intent.as_str(), "resegment" | "reinterpret") {
            return Err(Error::Invalid("invalid Episode revision intent".into()));
        }
        let digest = operation_digest(
            "revise_episode",
            input.subject,
            &serde_json::json!({
                "episode_id": input.episode_id,
                "expected_object_epoch": input.expected_object_epoch,
                "intent": input.intent,
                "title": input.title,
                "parent": input.parent_episode_revision_id,
                "experience_time": input.experience_time,
                "boundary_explanation": input.boundary_explanation,
                "members": input.members,
                "supports": input.supports,
            }),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "revise_episode",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self
                    .episode(
                        input.subject,
                        input.episode_id,
                        receipt.result_revision.map(EpisodeRevisionId),
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "revise Episode operation is already in progress".into(),
            ));
        }
        let row = sqlx::query("SELECT track_key,current_revision_id,object_epoch,purge_state FROM episode_objects WHERE subject_id=$1 AND episode_id=$2 FOR UPDATE")
            .bind(input.subject.0).bind(input.episode_id.0).fetch_optional(&mut *tx).await.map_err(db)?
            .ok_or_else(|| Error::NotFound("Episode not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(Error::Conflict(
                "expected Episode object epoch is stale".into(),
            ));
        }
        if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
            return Err(Error::FailedPrecondition("Episode is purging".into()));
        }
        validate_episode_refs_in_tx(&self.store, &mut tx, input.subject, &input.members).await?;
        validate_episode_supports_in_tx(&self.store, &mut tx, input.subject, &input.supports)
            .await?;
        let parent = input.parent_episode_revision_id;
        validate_parent_and_overlap(
            &mut tx,
            input.subject,
            &row.try_get::<String, _>("track_key").map_err(db)?,
            parent,
            &input.experience_time,
            Some(input.episode_id.0),
        )
        .await?;
        if let Some(parent) = parent {
            ensure_no_parent_cycle(&mut tx, input.episode_id.0, parent.0).await?;
        }
        let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
        let revision_no: i32 = sqlx::query_scalar(
            "SELECT revision_no+1 FROM episode_revisions WHERE episode_revision_id=$1",
        )
        .bind(current)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let revision_id = EpisodeRevisionId::new();
        let formed_at = self
            .formation_time_in(&mut tx, input.subject, input.operation_id, started_at)
            .await?;
        let now = self.cognition.now(input.subject);
        let (kind, start, end) = temporal_columns(&input.experience_time);
        let create = EpisodeInput {
            operation_id: input.operation_id,
            subject: input.subject,
            track_key: row.try_get("track_key").map_err(db)?,
            title: input.title,
            parent_episode_revision_id: input.parent_episode_revision_id,
            experience_time: input.experience_time,
            boundary_explanation: input.boundary_explanation,

            producer_signature_id: input.producer_signature_id,
            members: input.members,
            supports: input.supports,
        };
        insert_episode_revision_with_intent(
            &mut tx,
            &create,
            input.episode_id,
            revision_id,
            Some(EpisodeRevisionId(current)),
            Some(input.intent),
            revision_no,
            kind,
            start,
            end,
            formed_at,
            now,
        )
        .await?;
        sqlx::query("UPDATE episode_objects SET current_revision_id=$3,object_epoch=object_epoch+1,integrity_state='valid' WHERE subject_id=$1 AND episode_id=$2")
            .bind(input.subject.0).bind(input.episode_id.0).bind(revision_id.0).execute(&mut *tx).await.map_err(db)?;
        let sequence =
            AuthorityStore::invalidate_in(&mut tx, input.subject, ProjectionInvalidation::all())
                .await?;
        self.schedule_episode_in(
            &mut tx,
            input.subject,
            revision_id,
            &create.track_key,
            sequence,
            true,
        )
        .await?;
        self.invalidate_episode_journals_in(
            &mut tx,
            input.subject,
            &[input.episode_id.0],
            sequence,
            "source_revised",
        )
        .await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "episode",
            Some(&input.episode_id.0.to_string()),
            Some(revision_id.0),
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.episode(input.subject, input.episode_id, None).await
    }

    pub async fn link_episode_revisions(
        &self,
        subject: SubjectId,
        operation_id: OperationId,
        from: EpisodeRevisionId,
        to: EpisodeRevisionId,
        relation: String,
    ) -> Result<()> {
        if from == to
            || !matches!(
                relation.as_str(),
                "split_from" | "merged_from" | "temporal_successor" | "derived_from"
            )
        {
            return Err(Error::Invalid("invalid Episode relation".into()));
        }
        let digest = operation_digest(
            "link_episode_revisions",
            subject,
            &serde_json::json!({"from":from,"to":to,"relation":relation}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "link_episode_revisions",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Ok(());
            }
            return Err(Error::Unavailable(
                "Episode relation operation is already in progress".into(),
            ));
        }
        for revision in [from, to] {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM episode_revisions WHERE subject_id=$1 AND episode_revision_id=$2)").bind(subject.0).bind(revision.0).fetch_one(&mut *tx).await.map_err(db)?;
            if !exists {
                return Err(Error::NotFound("Episode revision not found".into()));
            }
        }
        sqlx::query("INSERT INTO episode_revision_relations(from_revision_id,to_revision_id,relation,created_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(from.0).bind(to.0).bind(&relation).bind(self.cognition.now(subject)).execute(&mut *tx).await.map_err(db)?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "episode_relation",
            None,
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }

    pub async fn suppress_episode(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        operation_id: OperationId,
        expected: i64,
    ) -> Result<EpisodeView> {
        self.mutate_episode_lifecycle(
            subject,
            episode,
            operation_id,
            expected,
            "suppression_state",
            "normal",
            "suppressed",
        )
        .await
    }
    pub async fn restore_episode(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        operation_id: OperationId,
        expected: i64,
    ) -> Result<EpisodeView> {
        self.mutate_episode_lifecycle(
            subject,
            episode,
            operation_id,
            expected,
            "suppression_state",
            "suppressed",
            "normal",
        )
        .await
    }
    pub async fn withdraw_episode(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        operation_id: OperationId,
        expected: i64,
    ) -> Result<EpisodeView> {
        self.mutate_episode_lifecycle(
            subject,
            episode,
            operation_id,
            expected,
            "acceptance_state",
            "accepted",
            "withdrawn",
        )
        .await
    }
    pub async fn reaccept_episode(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        operation_id: OperationId,
        expected: i64,
    ) -> Result<EpisodeView> {
        self.mutate_episode_lifecycle(
            subject,
            episode,
            operation_id,
            expected,
            "acceptance_state",
            "withdrawn",
            "accepted",
        )
        .await
    }

    async fn mutate_episode_lifecycle(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        operation_id: OperationId,
        expected: i64,
        field: &str,
        from: &str,
        to: &str,
    ) -> Result<EpisodeView> {
        let digest = operation_digest(
            "episode_lifecycle",
            subject,
            &serde_json::json!({"episode":episode,"expected":expected,"field":field,"from":from,"to":to}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) =
            check_receipt(&mut tx, subject, operation_id, "episode_lifecycle", &digest).await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self.episode(subject, episode, None).await;
            }
            return Err(Error::Unavailable(
                "Episode lifecycle operation is already in progress".into(),
            ));
        }
        let row = sqlx::query("SELECT object_epoch,suppression_state,acceptance_state,purge_state FROM episode_objects WHERE subject_id=$1 AND episode_id=$2 FOR UPDATE")
            .bind(subject.0).bind(episode.0).fetch_optional(&mut *tx).await.map_err(db)?
            .ok_or_else(|| Error::NotFound("Episode not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != expected {
            return Err(Error::Conflict(
                "expected Episode object epoch is stale".into(),
            ));
        }
        if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
            return Err(Error::FailedPrecondition("Episode is purging".into()));
        }
        let current: String = row.try_get(field).map_err(db)?;
        if current != from {
            return Err(Error::FailedPrecondition(format!(
                "Episode is not in {from} state"
            )));
        }
        if field == "acceptance_state" && to == "accepted" {
            validate_episode_reaccept_in(&mut tx, subject, episode).await?;
        }
        let query = match field {
            "suppression_state" => sqlx::query(
                "UPDATE episode_objects SET suppression_state=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND episode_id=$2",
            ),
            "acceptance_state" => sqlx::query(
                "UPDATE episode_objects SET acceptance_state=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND episode_id=$2",
            ),
            _ => {
                return Err(Error::Internal(
                    "unsupported Episode lifecycle field".into(),
                ));
            }
        };
        query
            .bind(subject.0)
            .bind(episode.0)
            .bind(to)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let sequence =
            AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
        self.invalidate_episode_journals_in(
            &mut tx,
            subject,
            &[episode.0],
            sequence,
            "source_lifecycle_changed",
        )
        .await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "episode",
            Some(&episode.0.to_string()),
            None,
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.episode(subject, episode, None).await
    }

    pub async fn purge_episode(
        &self,
        subject: SubjectId,
        episode: EpisodeId,
        operation_id: OperationId,
        expected: i64,
    ) -> Result<()> {
        let digest = operation_digest(
            "purge_episode",
            subject,
            &serde_json::json!({"episode":episode,"expected":expected}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) =
            check_receipt(&mut tx, subject, operation_id, "purge_episode", &digest).await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Ok(());
            }
            return Err(Error::Unavailable(
                "Episode purge operation is already in progress".into(),
            ));
        }
        let epoch: i64 = sqlx::query_scalar("SELECT object_epoch FROM episode_objects WHERE subject_id=$1 AND episode_id=$2 FOR UPDATE").bind(subject.0).bind(episode.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(|| Error::NotFound("Episode not found".into()))?;
        if epoch != expected {
            return Err(Error::Conflict(
                "expected Episode object epoch is stale".into(),
            ));
        }
        let sequence =
            AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
        self.invalidate_episode_journals_in(
            &mut tx,
            subject,
            &[episode.0],
            sequence,
            "source_purged",
        )
        .await?;
        self.purge_episode_runtime_refs_in(&mut tx, subject, episode)
            .await?;
        sqlx::query("DELETE FROM episode_objects WHERE subject_id=$1 AND episode_id=$2")
            .bind(subject.0)
            .bind(episode.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "purged",
            Some(&episode.0.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
}

impl MemoryService {
    async fn purge_episode_runtime_refs_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        episode: EpisodeId,
    ) -> Result<()> {
        let refs: Vec<String> = sqlx::query_scalar("SELECT episode_revision_id::text FROM episode_revisions WHERE subject_id=$1 AND episode_id=$2")
            .bind(subject.0).bind(episode.0).fetch_all(&mut **tx).await.map_err(db)?;
        let mut workflow_refs = refs.clone();
        workflow_refs.push(episode.0.to_string());
        self.purge_workflow_content_in(tx, subject, &workflow_refs)
            .await?;
        sqlx::query("INSERT INTO purged_use_receipts(subject_id,consumer_ref,event_id,request_digest,purged_at) SELECT subject_id,consumer_ref,event_id,request_digest,$3 FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind='episode_revision' AND ref_value=ANY($2::text[]) ON CONFLICT DO NOTHING")
            .bind(subject.0).bind(&refs).bind(self.cognition.now(subject)).execute(&mut **tx).await.map_err(db)?;
        sqlx::query("DELETE FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind='episode_revision' AND ref_value=ANY($2::text[])")
            .bind(subject.0).bind(&refs).execute(&mut **tx).await.map_err(db)?;
        sqlx::query("DELETE FROM resident_refs WHERE ref_kind='episode_revision' AND ref_value=ANY($1::text[])")
            .bind(&refs).execute(&mut **tx).await.map_err(db)?;
        sqlx::query("DELETE FROM work_context_refs WHERE ref_kind='episode_revision' AND ref_value=ANY($1::text[])")
            .bind(&refs).execute(&mut **tx).await.map_err(db)?;
        sqlx::query("UPDATE model_workflow_operations SET snapshot='{}'::jsonb,proposal=NULL,outcome='{\"purged\":true}'::jsonb,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE subject_id=$1 AND owner='memory' AND lower(operation_key) IN (SELECT operation_id::text FROM mutation_receipts WHERE subject_id=$1 AND result_kind='episode' AND result_ref=$2)")
            .bind(subject.0).bind(episode.0.to_string()).execute(&mut **tx).await.map_err(db)?;
        Ok(())
    }
}

fn validate_episode_input(
    track: &str,
    title: &Option<String>,
    time: &TemporalExtent,
    boundary: &str,
    members: &[EpisodeMemberInput],
    supports: &[RevisionSupport],
) -> Result<()> {
    if !track.is_empty() && (track.len() > 128 || track.chars().any(char::is_control)) {
        return Err(Error::Invalid("Episode track_key is out of bounds".into()));
    }
    if title.as_ref().is_some_and(|value| value.len() > 8192) {
        return Err(Error::Invalid("Episode title is out of bounds".into()));
    }
    time.validate()?;
    if boundary.trim().is_empty() || boundary.len() > 16384 {
        return Err(Error::Invalid(
            "Episode boundary explanation is out of bounds".into(),
        ));
    }
    if members.is_empty() || members.len() > 2048 {
        return Err(Error::Invalid("Episode members are out of bounds".into()));
    }
    if supports.is_empty() || supports.len() > 256 {
        return Err(Error::Invalid("Episode supports are out of bounds".into()));
    }
    let mut keys = BTreeSet::new();
    for member in members {
        if member.role.is_empty()
            || member.role.len() > 128
            || !keys.insert(member.reference.to_string())
        {
            return Err(Error::Invalid(
                "Episode members must be unique and bounded".into(),
            ));
        }
    }
    validate_exact_supports(supports)
}

async fn validate_episode_refs_in_tx(
    store: &AuthorityStore,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    members: &[EpisodeMemberInput],
) -> Result<()> {
    for member in members {
        if !matches!(
            member.reference,
            CognitiveRef::Occurrence(_)
                | CognitiveRef::MemoryRevision(_)
                | CognitiveRef::CognitiveSchemaRevision(_)
                | CognitiveRef::EpisodeRevision(_)
        ) {
            return Err(Error::Invalid("invalid Episode member reference".into()));
        }
        if !store
            .reference_in_subject_tx(tx, subject, &member.reference)
            .await?
        {
            return Err(Error::FailedPrecondition(
                "Episode member is outside Subject".into(),
            ));
        }
    }
    Ok(())
}

async fn validate_episode_supports_in_tx(
    store: &AuthorityStore,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    supports: &[RevisionSupport],
) -> Result<()> {
    for support in supports {
        match support {
            RevisionSupport::Evidence(value) => {
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2)").bind(subject.0).bind(value.occurrence_id.0).fetch_one(&mut **tx).await.map_err(db)?;
                if !exists {
                    return Err(Error::Invalid("Episode evidence is outside Subject".into()));
                }
            }
            RevisionSupport::CognitionDependency(value) => {
                if !matches!(
                    value.target_revision,
                    CognitiveRef::MemoryRevision(_)
                        | CognitiveRef::CognitiveSchemaRevision(_)
                        | CognitiveRef::EpisodeRevision(_)
                ) {
                    return Err(Error::Invalid(
                        "Episode support must target an exact revision".into(),
                    ));
                }
                if !store
                    .reference_in_subject_tx(tx, subject, &value.target_revision)
                    .await?
                {
                    return Err(Error::Invalid("Episode support is outside Subject".into()));
                }
            }
            RevisionSupport::Seed(_) => {
                return Err(Error::Invalid(
                    "Episode cannot use Cognitive Seed support".into(),
                ));
            }
        }
    }
    Ok(())
}

async fn validate_parent_and_overlap(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    track: &str,
    parent: Option<EpisodeRevisionId>,
    time: &TemporalExtent,
    exclude: Option<Uuid>,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!(
            "episode-segment:{}:{}:{:?}",
            subject.0,
            track,
            parent.map(|value| value.0)
        ))
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    if let Some(parent) = parent {
        let parent_time = sqlx::query("SELECT experience_time_kind,experience_time_start,experience_time_end FROM episode_revisions WHERE episode_revision_id=$1 AND subject_id=$2").bind(parent.0).bind(subject.0).fetch_optional(&mut **tx).await.map_err(db)?.ok_or_else(|| Error::Invalid("Episode parent is outside Subject".into()))?;
        let parent_extent = temporal_from_columns(
            parent_time.try_get("experience_time_kind").map_err(db)?,
            parent_time.try_get("experience_time_start").map_err(db)?,
            parent_time.try_get("experience_time_end").map_err(db)?,
        )?;
        if !contains_extent(&parent_extent, time) {
            return Err(Error::Invalid(
                "Episode child lies outside parent experience span".into(),
            ));
        }
    }
    let rows = sqlx::query("SELECT r.experience_time_kind,r.experience_time_start,r.experience_time_end FROM episode_objects o JOIN episode_revisions r ON r.episode_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.track_key=$2 AND o.acceptance_state='accepted' AND r.parent_episode_revision_id IS NOT DISTINCT FROM $3 AND ($4::uuid IS NULL OR o.episode_id<>$4)").bind(subject.0).bind(track).bind(parent.map(|value| value.0)).bind(exclude).fetch_all(&mut **tx).await.map_err(db)?;
    for row in rows {
        let sibling = temporal_from_columns(
            row.try_get("experience_time_kind").map_err(db)?,
            row.try_get("experience_time_start").map_err(db)?,
            row.try_get("experience_time_end").map_err(db)?,
        )?;
        if known_overlap(&sibling, time) {
            return Err(Error::Conflict(
                "same-track Episode sibling experience spans overlap".into(),
            ));
        }
    }
    Ok(())
}

async fn validate_episode_reaccept_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    episode: EpisodeId,
) -> Result<()> {
    let row = sqlx::query("SELECT o.track_key,r.parent_episode_revision_id,r.experience_time_kind,r.experience_time_start,r.experience_time_end FROM episode_objects o JOIN episode_revisions r ON r.episode_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.episode_id=$2")
        .bind(subject.0).bind(episode.0).fetch_one(&mut **tx).await.map_err(db)?;
    let time = temporal_from_columns(
        row.try_get("experience_time_kind").map_err(db)?,
        row.try_get("experience_time_start").map_err(db)?,
        row.try_get("experience_time_end").map_err(db)?,
    )?;
    validate_parent_and_overlap(
        tx,
        subject,
        &row.try_get::<String, _>("track_key").map_err(db)?,
        row.try_get::<Option<Uuid>, _>("parent_episode_revision_id")
            .map_err(db)?
            .map(EpisodeRevisionId),
        &time,
        Some(episode.0),
    )
    .await
}

async fn ensure_no_parent_cycle(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    episode: Uuid,
    mut parent: Uuid,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    loop {
        if parent == episode || !seen.insert(parent) {
            return Err(Error::Conflict(
                "Episode parent hierarchy contains a cycle".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT episode_id,parent_episode_revision_id FROM episode_revisions WHERE episode_revision_id=$1",
        )
        .bind(parent)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::Invalid("Episode parent revision does not exist".into()))?;
        let parent_episode: Uuid = row.try_get("episode_id").map_err(db)?;
        if parent_episode == episode {
            return Err(Error::Conflict(
                "Episode parent hierarchy contains a cycle".into(),
            ));
        }
        let next: Option<Uuid> = row.try_get("parent_episode_revision_id").map_err(db)?;
        let Some(next) = next else {
            return Ok(());
        };
        parent = next;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Episode creation binds one immutable revision and its object identity"
)]
async fn insert_episode_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &EpisodeInput,
    episode: EpisodeId,
    revision: EpisodeRevisionId,
    parent_revision: Option<EpisodeRevisionId>,
    intent: Option<String>,
    kind: &str,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    formed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
) -> Result<()> {
    insert_episode_revision_with_intent(
        tx,
        input,
        episode,
        revision,
        parent_revision,
        intent,
        1,
        kind,
        start,
        end,
        formed_at,
        recorded_at,
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "Episode revision insertion receives the complete canonical payload"
)]
async fn insert_episode_revision_with_intent(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &EpisodeInput,
    episode: EpisodeId,
    revision: EpisodeRevisionId,
    parent_revision: Option<EpisodeRevisionId>,
    intent: Option<String>,
    revision_no: i32,
    kind: &str,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    formed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query("INSERT INTO episode_revisions(episode_revision_id,episode_id,subject_id,revision_no,parent_revision_id,revision_intent,title,parent_episode_revision_id,experience_time_kind,experience_time_start,experience_time_end,boundary_explanation,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
        .bind(revision.0).bind(episode.0).bind(input.subject.0).bind(revision_no).bind(parent_revision.map(|value| value.0)).bind(intent).bind(&input.title).bind(input.parent_episode_revision_id.map(|value| value.0)).bind(kind).bind(start).bind(end).bind(&input.boundary_explanation).bind(formed_at).bind(recorded_at).bind(input.producer_signature_id).execute(&mut **tx).await.map_err(db)?;
    for (ordinal, member) in input.members.iter().enumerate() {
        let (kind, value) = reference_parts(&member.reference);
        sqlx::query("INSERT INTO episode_revision_members(episode_revision_id,ordinal,ref_kind,ref_value,role) VALUES($1,$2,$3,$4,$5)").bind(revision.0).bind(ordinal as i32).bind(kind).bind(value).bind(&member.role).execute(&mut **tx).await.map_err(db)?;
    }
    for (support_no, support) in input.supports.iter().enumerate() {
        insert_episode_support(tx, revision, support_no as i32, support).await?;
    }
    Ok(())
}

async fn insert_episode_support(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    revision: EpisodeRevisionId,
    support_no: i32,
    support: &RevisionSupport,
) -> Result<()> {
    let (kind, reference, occurrence, source_region, derived, derived_region, role) = match support
    {
        RevisionSupport::Evidence(value) => {
            let (source, derived, derived_region) = match value.locator {
                EvidenceLocator::WholeOccurrence => (None, None, None),
                EvidenceLocator::SourceRegion(id) => (Some(id.0), None, None),
                EvidenceLocator::DerivedRepresentation(id) => (None, Some(id.0), None),
                EvidenceLocator::DerivedRegion(id) => (None, None, Some(id.0)),
            };
            (
                "evidence".to_owned(),
                value.occurrence_id.0.to_string(),
                Some(value.occurrence_id.0),
                source,
                derived,
                derived_region,
                value.support_role.as_str().to_owned(),
            )
        }
        RevisionSupport::CognitionDependency(value) => {
            let (kind, reference) = reference_parts(&value.target_revision);
            (
                kind,
                reference,
                None,
                None,
                None,
                None,
                value.support_role.as_str().to_owned(),
            )
        }
        RevisionSupport::Seed(_) => {
            return Err(Error::Invalid(
                "Episode cannot use Cognitive Seed support".into(),
            ));
        }
    };
    sqlx::query("INSERT INTO episode_revision_supports(episode_revision_id,support_no,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(revision.0).bind(support_no).bind(kind).bind(reference).bind(role).bind(occurrence).bind(source_region).bind(derived).bind(derived_region).execute(&mut **tx).await.map_err(db)?;
    Ok(())
}

async fn load_members(pool: &sqlx::PgPool, revision: Uuid) -> Result<Vec<EpisodeMember>> {
    sqlx::query("SELECT ordinal,ref_kind,ref_value,role FROM episode_revision_members WHERE episode_revision_id=$1 ORDER BY ordinal").bind(revision).fetch_all(pool).await.map_err(db)?.into_iter().map(|row| Ok(EpisodeMember { ordinal: row.try_get("ordinal").map_err(db)?, reference: parse_reference(&row.try_get::<String,_>("ref_kind").map_err(db)?, &row.try_get::<String,_>("ref_value").map_err(db)?)?, role: row.try_get("role").map_err(db)? })).collect()
}

async fn load_supports(pool: &sqlx::PgPool, revision: Uuid) -> Result<Vec<RevisionSupport>> {
    let rows = sqlx::query("SELECT support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id FROM episode_revision_supports WHERE episode_revision_id=$1 ORDER BY support_no").bind(revision).fetch_all(pool).await.map_err(db)?;
    rows.into_iter()
        .map(|row| {
            let role = parse_enum(row.try_get("support_role").map_err(db)?, "support role")?;
            let kind: String = row.try_get("support_kind").map_err(db)?;
            if kind == "evidence" {
                let locator = if let Some(value) = row
                    .try_get::<Option<Uuid>, _>("source_region_id")
                    .map_err(db)?
                {
                    EvidenceLocator::SourceRegion(SourceRegionId(value))
                } else if let Some(value) = row
                    .try_get::<Option<Uuid>, _>("derived_representation_id")
                    .map_err(db)?
                {
                    EvidenceLocator::DerivedRepresentation(DerivedRepresentationId(value))
                } else if let Some(value) = row
                    .try_get::<Option<Uuid>, _>("derived_region_id")
                    .map_err(db)?
                {
                    EvidenceLocator::DerivedRegion(DerivedRegionId(value))
                } else {
                    EvidenceLocator::WholeOccurrence
                };
                return Ok(RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: OccurrenceId(
                        row.try_get::<Uuid, _>("occurrence_id").map_err(db)?,
                    ),
                    locator,
                    support_role: role,
                }));
            }
            Ok(RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: parse_reference(
                    &kind,
                    &row.try_get::<String, _>("support_ref").map_err(db)?,
                )?,
                support_role: role,
            }))
        })
        .collect()
}

fn known_overlap(left: &TemporalExtent, right: &TemporalExtent) -> bool {
    match (left, right) {
        (TemporalExtent::Instant { at: left }, TemporalExtent::Instant { at: right }) => {
            left == right
        }
        (TemporalExtent::Unknown, _) | (_, TemporalExtent::Unknown) => false,
        (TemporalExtent::Instant { at }, TemporalExtent::Interval { start, end })
        | (TemporalExtent::Interval { start, end }, TemporalExtent::Instant { at }) => {
            end.is_none_or(|end| *at < end) && start.is_none_or(|start| *at >= start)
        }
        (
            TemporalExtent::Interval {
                start: left_start,
                end: left_end,
            },
            TemporalExtent::Interval {
                start: right_start,
                end: right_end,
            },
        ) => {
            left_end.is_none_or(|end| right_start.is_none_or(|start| start < end))
                && right_end.is_none_or(|end| left_start.is_none_or(|start| start < end))
        }
    }
}

fn contains_extent(parent: &TemporalExtent, child: &TemporalExtent) -> bool {
    match (parent, child) {
        (TemporalExtent::Unknown, _) => true,
        (_, TemporalExtent::Unknown) => true,
        (TemporalExtent::Instant { at: parent }, TemporalExtent::Instant { at: child }) => {
            parent == child
        }
        (TemporalExtent::Interval { start, end }, TemporalExtent::Instant { at }) => {
            start.is_none_or(|value| *at >= value) && end.is_none_or(|value| *at < value)
        }
        (
            TemporalExtent::Interval {
                start: parent_start,
                end: parent_end,
            },
            TemporalExtent::Interval {
                start: child_start,
                end: child_end,
            },
        ) => {
            parent_start.is_none_or(|value| child_start.is_some_and(|child| child >= value))
                && parent_end.is_none_or(|value| child_end.is_some_and(|child| child <= value))
        }
        _ => false,
    }
}
