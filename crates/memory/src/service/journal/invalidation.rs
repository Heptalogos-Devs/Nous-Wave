use super::*;

impl MemoryService {
    pub(crate) async fn invalidate_episode_journals_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        episodes: &[Uuid],
        sequence: i64,
        reason: &str,
    ) -> Result<()> {
        let revisions: Vec<String> = sqlx::query_scalar("SELECT episode_revision_id::text FROM episode_revisions WHERE subject_id=$1 AND episode_id=ANY($2::uuid[])")
            .bind(subject.0).bind(episodes).fetch_all(&mut **tx).await.map_err(db)?;
        let journals = sqlx::query("SELECT DISTINCT o.journal_id,o.current_revision_id FROM journal_objects o JOIN journal_revision_sources s ON s.journal_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND s.ref_kind='episode_revision' AND s.ref_value=ANY($2::text[]) AND o.purge_state='normal' ORDER BY o.journal_id")
            .bind(subject.0).bind(&revisions).fetch_all(&mut **tx).await.map_err(db)?;
        let now = self.cognition.now(subject);
        for row in journals {
            let journal: Uuid = row.get("journal_id");
            let revision: Uuid = row.get("current_revision_id");
            sqlx::query("UPDATE journal_objects SET integrity_state='revalidation_required',object_epoch=object_epoch+1 WHERE subject_id=$1 AND journal_id=$2 AND integrity_state<>'revalidation_required'")
                .bind(subject.0).bind(journal).execute(&mut **tx).await.map_err(db)?;
            sqlx::query("INSERT INTO cognition_dependency_invalidations(subject_id,dependent_kind,dependent_ref,invalidated_by_kind,invalidated_by_ref,reason,created_at) SELECT $1,'journal_revision',$2,'episode_revision',ref_value,$3,$4 FROM journal_revision_sources WHERE journal_revision_id=$5 AND ref_kind='episode_revision' AND ref_value=ANY($6::text[]) ON CONFLICT DO NOTHING")
                .bind(subject.0).bind(revision.to_string()).bind(reason).bind(now).bind(revision).bind(&revisions).execute(&mut **tx).await.map_err(db)?;
            self.cognition
                .enqueue_maintenance_in(
                    tx,
                    &nous_runtime::MaintenanceRequest {
                        subject,
                        kind: "journal_revalidate".into(),
                        scope_kind: "journal".into(),
                        scope_ref: journal.to_string(),
                        trigger_authority_seq: sequence,
                        due_at: now,
                        priority: 80,
                    },
                )
                .await?;
        }
        Ok(())
    }
}
