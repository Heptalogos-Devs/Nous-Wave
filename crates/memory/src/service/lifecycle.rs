use super::*;
use nous_persistence::database_error as db;

impl MemoryService {
    pub async fn suppress(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
    ) -> Result<MemoryView> {
        self.mutate_lifecycle(
            subject,
            memory,
            operation_id,
            expected_object_epoch,
            "suppression_state",
            "normal",
            "suppressed",
        )
        .await
    }

    pub async fn restore(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
    ) -> Result<MemoryView> {
        self.mutate_lifecycle(
            subject,
            memory,
            operation_id,
            expected_object_epoch,
            "suppression_state",
            "suppressed",
            "normal",
        )
        .await
    }

    pub async fn withdraw(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
    ) -> Result<MemoryView> {
        self.mutate_lifecycle(
            subject,
            memory,
            operation_id,
            expected_object_epoch,
            "acceptance_state",
            "accepted",
            "withdrawn",
        )
        .await
    }

    pub async fn reaccept(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
    ) -> Result<MemoryView> {
        self.mutate_lifecycle(
            subject,
            memory,
            operation_id,
            expected_object_epoch,
            "acceptance_state",
            "withdrawn",
            "accepted",
        )
        .await
    }

    async fn mutate_lifecycle(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
        field: &str,
        from: &str,
        to: &str,
    ) -> Result<MemoryView> {
        let digest = operation_digest(
            field,
            subject,
            &serde_json::json!({"memory_id":memory,"expected_object_epoch":expected_object_epoch,"from":from,"to":to}),
        )?;
        let mut tx = self.begin_mutation(subject).await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(&mut tx, subject, operation_id, field, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self.memory(subject, memory, None).await;
            }
            return Err(Error::Unavailable(
                "lifecycle operation is already in progress".into(),
            ));
        }
        let row = match field {
            "suppression_state" => sqlx::query("SELECT object_epoch,suppression_state FROM memory_objects WHERE subject_id=$1 AND memory_id=$2 FOR UPDATE"),
            "acceptance_state" => sqlx::query("SELECT object_epoch,acceptance_state FROM memory_objects WHERE subject_id=$1 AND memory_id=$2 FOR UPDATE"),
            _ => return Err(Error::Internal("unsupported lifecycle state".into())),
        }.bind(subject.0).bind(memory.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||Error::NotFound("memory not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        let current: String = row.try_get(field).map_err(db)?;
        if epoch != expected_object_epoch {
            return Err(Error::Conflict("expected object epoch is stale".into()));
        }
        if current == to {
            commit_receipt(
                &mut tx,
                subject,
                operation_id,
                "memory",
                Some(&memory.0.to_string()),
                None,
                Some(epoch),
            )
            .await?;
            tx.commit().await.map_err(db)?;
            return self.memory(subject, memory, None).await;
        }
        if current != from {
            return Err(Error::FailedPrecondition(format!(
                "memory is not in {from} state"
            )));
        }
        match field {
            "suppression_state" => sqlx::query("UPDATE memory_objects SET suppression_state=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND memory_id=$2"),
            "acceptance_state" => sqlx::query("UPDATE memory_objects SET acceptance_state=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND memory_id=$2"),
            _ => return Err(Error::Internal("unsupported lifecycle state".into())),
        }.bind(subject.0).bind(memory.0).bind(to).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM resident_refs WHERE ref_kind='memory_revision' AND ref_value IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$1)").bind(memory.0).execute(&mut *tx).await.map_err(db)?;
        let sequence =
            AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
        self.invalidate_object_dependents_in(
            &mut tx,
            subject,
            "memory",
            &[memory.0],
            sequence,
            "source_lifecycle_changed",
        )
        .await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "memory",
            Some(&memory.0.to_string()),
            None,
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.memory(subject, memory, None).await
    }

    pub async fn purge_memory(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
        expected_object_epoch: i64,
    ) -> Result<()> {
        let digest = operation_digest(
            "purge_memory",
            subject,
            &serde_json::json!({"memory_id":memory,"expected_object_epoch":expected_object_epoch}),
        )?;
        let mut phase1 = self.begin_mutation(subject).await?;
        lock_operation(&mut phase1, subject, operation_id).await?;
        if let Some(receipt) =
            check_receipt(&mut phase1, subject, operation_id, "purge_memory", &digest).await?
        {
            phase1.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Ok(());
            }
        } else {
            let row=sqlx::query("SELECT object_epoch,purge_state FROM memory_objects WHERE subject_id=$1 AND memory_id=$2 FOR UPDATE").bind(subject.0).bind(memory.0).fetch_optional(&mut *phase1).await.map_err(db)?.ok_or_else(||Error::NotFound("memory not found".into()))?;
            let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
            let purge_state: String = row.try_get("purge_state").map_err(db)?;
            if epoch != expected_object_epoch {
                return Err(Error::Conflict("expected object epoch is stale".into()));
            }
            if purge_state != "purging" {
                sqlx::query("UPDATE memory_objects SET purge_state='purging',object_epoch=object_epoch+1 WHERE subject_id=$1 AND memory_id=$2").bind(subject.0).bind(memory.0).execute(&mut *phase1).await.map_err(db)?;
                let sequence = AuthorityStore::invalidate_in(
                    &mut phase1,
                    subject,
                    ProjectionInvalidation::all(),
                )
                .await?;
                self.invalidate_object_dependents_in(
                    &mut phase1,
                    subject,
                    "memory",
                    &[memory.0],
                    sequence,
                    "support_purged",
                )
                .await?;
            }
            sqlx::query("DELETE FROM resident_refs WHERE ref_kind='memory_revision' AND ref_value IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$1)").bind(memory.0).execute(&mut *phase1).await.map_err(db)?;
            phase1.commit().await.map_err(db)?;
        }
        self.complete_purge(subject, memory, operation_id).await
    }

    async fn complete_purge(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        operation_id: OperationId,
    ) -> Result<()> {
        let mut tx = self.begin_mutation(subject).await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        let receipt=sqlx::query("SELECT state,request_digest FROM mutation_receipts WHERE subject_id=$1 AND operation_id=$2 FOR UPDATE").bind(subject.0).bind(operation_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||Error::NotFound("purge receipt not found".into()))?;
        if receipt.try_get::<String, _>("state").map_err(db)? == "committed" {
            tx.commit().await.map_err(db)?;
            return Ok(());
        }
        let revisions = sqlx::query_scalar::<_, Uuid>(
            "SELECT memory_revision_id FROM memory_revisions WHERE subject_id=$1 AND memory_id=$2",
        )
        .bind(subject.0)
        .bind(memory.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for revision in &revisions {
            sqlx::query("UPDATE cognitive_schema_evidence_links SET revoked_at=COALESCE(revoked_at,$3) WHERE support_kind='memory_revision' AND support_ref=$1 AND revoked_at IS NULL AND subject_id=$2")
                .bind(revision.to_string()).bind(subject.0).bind(self.cognition.now(subject)).execute(&mut *tx).await.map_err(db)?;
            let use_rows=sqlx::query("SELECT subject_id,consumer_ref,event_id,request_digest FROM cognitive_use_events WHERE ref_kind='memory_revision' AND ref_value=$1").bind(revision.to_string()).fetch_all(&mut *tx).await.map_err(db)?;
            for row in use_rows {
                sqlx::query("INSERT INTO purged_use_receipts(subject_id,consumer_ref,event_id,request_digest,purged_at) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING").bind(row.try_get::<Uuid,_>("subject_id").map_err(db)?).bind(row.try_get::<String,_>("consumer_ref").map_err(db)?).bind(row.try_get::<Uuid,_>("event_id").map_err(db)?).bind(row.try_get::<String,_>("request_digest").map_err(db)?).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
            }
            sqlx::query("DELETE FROM cognitive_use_events WHERE ref_kind='memory_revision' AND ref_value=$1").bind(revision.to_string()).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("DELETE FROM association_evidence WHERE subject_id=$1 AND association_evidence_id IN (SELECT association_evidence_id FROM association_evidence_supports WHERE support_kind='memory_revision' AND support_ref=$2)")
                .bind(subject.0).bind(revision.to_string()).execute(&mut *tx).await.map_err(db)?;
        }
        sqlx::query("DELETE FROM association_evidence WHERE subject_id=$1 AND ((from_ref_kind='memory_revision' AND from_ref IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$2)) OR (to_ref_kind='memory_revision' AND to_ref IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$2)))").bind(subject.0).bind(memory.0).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE model_workflow_operations SET snapshot='{}'::jsonb,proposal=NULL,outcome='{\"purged\":true}'::jsonb,lease_token=NULL,lease_until=NULL,updated_at=now() WHERE subject_id=$1 AND owner='memory' AND lower(operation_key) IN (SELECT operation_id::text FROM mutation_receipts WHERE subject_id=$1 AND result_kind='memory' AND result_ref=$2)").bind(subject.0).bind(memory.0.to_string()).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM memory_objects WHERE subject_id=$1 AND memory_id=$2")
            .bind(subject.0)
            .bind(memory.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "purged",
            Some(&memory.0.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
}

impl MemoryService {
    pub(crate) async fn purge_workflow_content_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        refs: &[String],
    ) -> Result<()> {
        sqlx::query("UPDATE model_workflow_operations w SET snapshot='{}'::jsonb,proposal=NULL,outcome='{\"purged\":true}'::jsonb,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE w.subject_id=$1 AND w.owner='memory' AND (jsonb_path_query_array(w.snapshot,'$.**') ?| $2::text[] OR jsonb_path_query_array(w.proposal,'$.**') ?| $2::text[] OR lower(w.operation_key) IN (SELECT r.operation_id::text FROM mutation_receipts r WHERE r.subject_id=$1 AND (r.result_revision::text=ANY($2::text[]) OR r.result_ref=ANY($2::text[]) OR CASE WHEN r.result_kind IN ('episode_partition','longitudinal_consolidation') THEN jsonb_path_query_array(r.result_ref::jsonb,'$.**') ?| $2::text[] ELSE false END)))")
            .bind(subject.0).bind(refs).execute(&mut **tx).await.map_err(db)?;
        Ok(())
    }
}
