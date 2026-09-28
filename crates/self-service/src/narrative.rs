use super::*;

impl SelfService {
    pub async fn create_narrative(
        &self,
        input: CreateNarrativeIdentity,
    ) -> Result<NarrativeIdentityView> {
        validate_narrative_create(&input)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("narrative_identity.create", input.subject, &input)?;
        let object_id = nous_core::NarrativeIdentityId::new();
        let revision_id = nous_core::NarrativeIdentityRevisionId::new();
        let now = Utc::now();
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity.create",
            &digest,
        )
        .await?
            && receipt.state == "committed"
        {
            let id = Uuid::parse_str(&receipt.result_ref.ok_or_else(|| {
                nous_core::Error::Infrastructure("Narrative receipt missing result".into())
            })?)
            .map_err(|_| nous_core::Error::Infrastructure("invalid Narrative receipt".into()))?;
            tx.commit().await.map_err(db)?;
            return self
                .narrative(input.subject, nous_core::NarrativeIdentityId(id))
                .await;
        }
        sqlx::query("SELECT subject_id FROM subjects WHERE subject_id=$1 FOR UPDATE")
            .bind(input.subject.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM narrative_identities WHERE subject_id=$1 AND key=$2)",
        )
        .bind(input.subject.0)
        .bind(&input.key)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?
        {
            return Err(nous_core::Error::Conflict(
                "Narrative Identity key already exists".into(),
            ));
        }
        validate_supports(&mut tx, input.subject, &input.supports).await?;
        validate_narrative_targets(&mut tx, input.subject, &input.references).await?;
        let (kind, start, end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO narrative_identities(narrative_identity_id,subject_id,key,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,$4,1,'accepted','valid','normal','normal',$5)").bind(object_id.0).bind(input.subject.0).bind(&input.key).bind(revision_id.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO narrative_identity_revisions(narrative_identity_revision_id,narrative_identity_id,subject_id,revision_no,parent_revision_id,revision_intent,text,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,1,NULL,NULL,$4,$5,$6,$7,$8,$9,$10)").bind(revision_id.0).bind(object_id.0).bind(input.subject.0).bind(&input.text).bind(kind).bind(start).bind(end).bind(input.formed_at).bind(now).bind(input.producer_signature_id).execute(&mut *tx).await.map_err(db)?;
        insert_supports(
            &mut tx,
            "narrative_identity_revision_supports",
            "narrative_identity_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        insert_narrative_references(&mut tx, revision_id.0, &input.references).await?;
        invalidate_projections(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity",
            &object_id.0.to_string(),
            revision_id.0,
            1,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.narrative(input.subject, object_id).await
    }

    pub async fn revise_narrative(
        &self,
        input: ReviseNarrativeIdentity,
    ) -> Result<NarrativeIdentityView> {
        validate_narrative_revision(&input)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("narrative_identity.revise", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity.revise",
            &digest,
        )
        .await?
            && receipt.state == "committed"
        {
            tx.commit().await.map_err(db)?;
            return self
                .narrative(input.subject, input.narrative_identity_id)
                .await;
        }
        let row=sqlx::query("SELECT current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state FROM narrative_identities WHERE subject_id=$1 AND narrative_identity_id=$2 FOR UPDATE").bind(input.subject.0).bind(input.narrative_identity_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||nous_core::Error::NotFound("Narrative Identity not found".into()))?;
        let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(nous_core::Error::Conflict(
                "Narrative Identity object_epoch is stale".into(),
            ));
        }
        if current != input.parent_revision_id.0 {
            return Err(nous_core::Error::Conflict(
                "Narrative Identity parent revision is not current".into(),
            ));
        }
        require_active_lifecycle(&row)?;
        validate_supports(&mut tx, input.subject, &input.supports).await?;
        validate_narrative_targets(&mut tx, input.subject, &input.references).await?;
        let revision_id = nous_core::NarrativeIdentityRevisionId::new();
        let next:i32=sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM narrative_identity_revisions WHERE narrative_identity_id=$1").bind(input.narrative_identity_id.0).fetch_one(&mut *tx).await.map_err(db)?;
        let now = Utc::now();
        let (kind, start, end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO narrative_identity_revisions(narrative_identity_revision_id,narrative_identity_id,subject_id,revision_no,parent_revision_id,revision_intent,text,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)").bind(revision_id.0).bind(input.narrative_identity_id.0).bind(input.subject.0).bind(next).bind(input.parent_revision_id.0).bind(enum_name(input.revision_intent)).bind(&input.text).bind(kind).bind(start).bind(end).bind(input.formed_at).bind(now).bind(input.producer_signature_id).execute(&mut *tx).await.map_err(db)?;
        insert_supports(
            &mut tx,
            "narrative_identity_revision_supports",
            "narrative_identity_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        insert_narrative_references(&mut tx, revision_id.0, &input.references).await?;
        sqlx::query("UPDATE narrative_identities SET current_revision_id=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND narrative_identity_id=$2").bind(input.subject.0).bind(input.narrative_identity_id.0).bind(revision_id.0).execute(&mut *tx).await.map_err(db)?;
        invalidate_projections(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "narrative_identity",
            &input.narrative_identity_id.0.to_string(),
            revision_id.0,
            epoch + 1,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.narrative(input.subject, input.narrative_identity_id)
            .await
    }
}
