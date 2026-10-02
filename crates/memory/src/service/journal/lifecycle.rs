use super::*;

impl MemoryService {
    pub async fn mutate_journal_lifecycle(
        &self,
        subject: SubjectId,
        journal: JournalId,
        operation: OperationId,
        expected_epoch: i64,
        action: &str,
    ) -> Result<JournalView> {
        let (field, from, to) = match action {
            "suppress" => ("suppression_state", "normal", "suppressed"),
            "restore" => ("suppression_state", "suppressed", "normal"),
            "withdraw" => ("acceptance_state", "accepted", "withdrawn"),
            "reaccept" => ("acceptance_state", "withdrawn", "accepted"),
            _ => return Err(Error::Invalid("invalid Journal lifecycle action".into())),
        };
        let digest = operation_digest(
            "journal_lifecycle",
            subject,
            &serde_json::json!({"journal":journal,"epoch":expected_epoch,"action":action}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation).await?;
        if let Some(receipt) =
            check_receipt(&mut tx, subject, operation, "journal_lifecycle", &digest).await?
        {
            if receipt.state != "committed" {
                return Err(Error::Unavailable(
                    "Journal lifecycle is in progress".into(),
                ));
            }
            tx.commit().await.map_err(db)?;
            return self
                .journal(
                    subject,
                    journal,
                    receipt.result_revision.map(JournalRevisionId),
                )
                .await;
        }
        sqlx::query("SELECT subject_id FROM subjects WHERE subject_id=$1 FOR UPDATE")
            .bind(subject.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let row = sqlx::query(
            "SELECT * FROM journal_objects WHERE subject_id=$1 AND journal_id=$2 FOR UPDATE",
        )
        .bind(subject.0)
        .bind(journal.0)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::NotFound("Journal not found".into()))?;
        if row.get::<i64, _>("object_epoch") != expected_epoch {
            return Err(Error::Conflict("Journal epoch is stale".into()));
        }
        if row.get::<String, _>(field) != from || row.get::<String, _>("purge_state") != "normal" {
            return Err(Error::FailedPrecondition(
                "Journal lifecycle transition is invalid".into(),
            ));
        }
        let statement = if field == "suppression_state" {
            "UPDATE journal_objects SET suppression_state=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND journal_id=$2"
        } else {
            "UPDATE journal_objects SET acceptance_state=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND journal_id=$2"
        };
        sqlx::query(statement)
            .bind(subject.0)
            .bind(journal.0)
            .bind(to)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::text()).await?;
        let revision: Uuid = row.get("current_revision_id");
        commit_receipt(
            &mut tx,
            subject,
            operation,
            "journal",
            Some(&journal.0.to_string()),
            Some(revision),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.journal(subject, journal, Some(JournalRevisionId(revision)))
            .await
    }

    pub async fn purge_journal(
        &self,
        subject: SubjectId,
        journal: JournalId,
        operation: OperationId,
        expected_epoch: i64,
    ) -> Result<()> {
        let digest = operation_digest(
            "journal_purge",
            subject,
            &serde_json::json!({"journal":journal,"epoch":expected_epoch}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation).await?;
        if let Some(receipt) =
            check_receipt(&mut tx, subject, operation, "journal_purge", &digest).await?
        {
            if receipt.state != "committed" {
                return Err(Error::Unavailable("Journal purge is in progress".into()));
            }
            tx.commit().await.map_err(db)?;
            return Ok(());
        }
        let epoch:Option<i64>=sqlx::query_scalar("SELECT object_epoch FROM journal_objects WHERE subject_id=$1 AND journal_id=$2 FOR UPDATE")
            .bind(subject.0).bind(journal.0).fetch_optional(&mut *tx).await.map_err(db)?;
        if epoch != Some(expected_epoch) {
            return Err(Error::Conflict("Journal purge epoch is stale".into()));
        }
        sqlx::query("INSERT INTO purged_use_receipts(subject_id,consumer_ref,event_id,request_digest,purged_at) SELECT e.subject_id,e.consumer_ref,e.event_id,e.request_digest,$3 FROM cognitive_use_events e JOIN journal_revisions r ON r.journal_revision_id::text=e.ref_value WHERE e.subject_id=$1 AND e.ref_kind='journal_revision' AND r.journal_id=$2 ON CONFLICT DO NOTHING")
            .bind(subject.0).bind(journal.0).bind(self.cognition.now(subject)).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM cognitive_use_events e USING journal_revisions r WHERE e.subject_id=$1 AND e.ref_kind='journal_revision' AND e.ref_value=r.journal_revision_id::text AND r.journal_id=$2")
            .bind(subject.0).bind(journal.0).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM resident_refs rr USING journal_revisions r WHERE rr.ref_kind='journal_revision' AND rr.ref_value=r.journal_revision_id::text AND r.subject_id=$1 AND r.journal_id=$2")
            .bind(subject.0).bind(journal.0).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM work_context_refs wr USING journal_revisions r WHERE wr.ref_kind='journal_revision' AND wr.ref_value=r.journal_revision_id::text AND r.subject_id=$1 AND r.journal_id=$2")
            .bind(subject.0).bind(journal.0).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("UPDATE model_workflow_operations SET snapshot='{}'::jsonb,proposal=NULL,outcome='{\"purged\":true}'::jsonb,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE subject_id=$1 AND owner='memory' AND lower(operation_key) IN (SELECT operation_id::text FROM mutation_receipts WHERE subject_id=$1 AND result_kind='journal' AND result_ref=$2)")
            .bind(subject.0).bind(journal.0.to_string()).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM journal_objects WHERE subject_id=$1 AND journal_id=$2")
            .bind(subject.0)
            .bind(journal.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::text()).await?;
        commit_receipt(
            &mut tx,
            subject,
            operation,
            "purged",
            Some(&journal.0.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }
}
