use crate::*;
use nous_authority_store::database_error as db;

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
        let mut tx = self.store.begin().await?;
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
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
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
        let mut phase1 = self.store.begin().await?;
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
                AuthorityStore::invalidate_in(&mut phase1, subject, ProjectionInvalidation::all())
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
        let mut tx = self.store.begin().await?;
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
            sqlx::query("INSERT INTO cognition_dependency_invalidations(subject_id,dependent_kind,dependent_ref,invalidated_by_kind,invalidated_by_ref,reason,created_at) SELECT $1,'memory_revision',memory_revision_id::text,'memory_revision',$2,'support_purged',$3 FROM memory_revision_dependencies WHERE target_ref_kind='memory_revision' AND target_ref=$2 ON CONFLICT DO NOTHING")
                .bind(subject.0).bind(revision.to_string()).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("INSERT INTO cognition_dependency_invalidations(subject_id,dependent_kind,dependent_ref,invalidated_by_kind,invalidated_by_ref,reason,created_at) SELECT $1,'cognitive_schema_revision',schema_revision_id::text,'memory_revision',$2,'support_purged',$3 FROM cognitive_schema_evidence_links WHERE support_kind='memory_revision' AND support_ref=$2 AND revoked_at IS NULL ON CONFLICT DO NOTHING")
                .bind(subject.0).bind(revision.to_string()).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("UPDATE cognitive_schemas SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE schema_id IN (SELECT schema_id FROM cognitive_schema_revisions r JOIN cognitive_schema_evidence_links l ON l.schema_revision_id=r.schema_revision_id WHERE l.support_kind='memory_revision' AND l.support_ref=$1 AND l.revoked_at IS NULL) AND purge_state='normal'")
                .bind(revision.to_string()).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("UPDATE cognitive_schema_evidence_links SET revoked_at=COALESCE(revoked_at,$3) WHERE support_kind='memory_revision' AND support_ref=$1 AND revoked_at IS NULL AND subject_id=$2")
                .bind(revision.to_string()).bind(subject.0).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("UPDATE memory_objects SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE memory_id IN (SELECT r.memory_id FROM memory_revision_dependencies d JOIN memory_revisions r ON r.memory_revision_id=d.memory_revision_id WHERE d.target_ref_kind='memory_revision' AND d.target_ref=$1) AND memory_id<>$2").bind(revision.to_string()).bind(memory.0).execute(&mut *tx).await.map_err(db)?;
            let use_rows=sqlx::query("SELECT subject_id,consumer_ref,event_id,request_digest FROM cognitive_use_events WHERE ref_kind='memory_revision' AND ref_value=$1").bind(revision.to_string()).fetch_all(&mut *tx).await.map_err(db)?;
            for row in use_rows {
                sqlx::query("INSERT INTO purged_use_receipts(subject_id,consumer_ref,event_id,request_digest,purged_at) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING").bind(row.try_get::<Uuid,_>("subject_id").map_err(db)?).bind(row.try_get::<String,_>("consumer_ref").map_err(db)?).bind(row.try_get::<Uuid,_>("event_id").map_err(db)?).bind(row.try_get::<String,_>("request_digest").map_err(db)?).bind(Utc::now()).execute(&mut *tx).await.map_err(db)?;
            }
            sqlx::query("DELETE FROM cognitive_use_events WHERE ref_kind='memory_revision' AND ref_value=$1").bind(revision.to_string()).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("DELETE FROM association_evidence WHERE subject_id=$1 AND association_evidence_id IN (SELECT association_evidence_id FROM association_evidence_supports WHERE support_kind='memory_revision' AND support_ref=$2)")
                .bind(subject.0).bind(revision.to_string()).execute(&mut *tx).await.map_err(db)?;
        }
        sqlx::query("DELETE FROM association_evidence WHERE subject_id=$1 AND ((from_ref_kind='memory_revision' AND from_ref IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$2)) OR (to_ref_kind='memory_revision' AND to_ref IN (SELECT memory_revision_id::text FROM memory_revisions WHERE memory_id=$2)))").bind(subject.0).bind(memory.0).execute(&mut *tx).await.map_err(db)?;
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
