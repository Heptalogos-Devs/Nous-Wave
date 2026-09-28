use super::*;

impl SocialService {
    pub async fn relationship(
        &self,
        subject: SubjectId,
        id: RelationshipAssertionId,
    ) -> Result<SocialRelationshipView> {
        let row = sqlx::query("SELECT a.relation_type_id,a.from_kind,a.from_entity_ref,a.to_kind,a.to_entity_ref,a.current_revision_id,a.object_epoch,a.acceptance_state,a.integrity_state,a.suppression_state,a.purge_state,a.created_at,r.revision_no,r.parent_revision_id,r.revision_intent,r.degree_kind,r.degree_value,r.epistemic_class,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id FROM relationship_assertions a JOIN relationship_revisions r ON r.relationship_revision_id=a.current_revision_id WHERE a.subject_id=$1 AND a.relationship_id=$2")
            .bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(|| Error::NotFound("relationship not found".into()))?;
        let revision_id = RelationshipRevisionId(row.try_get("current_revision_id").map_err(db)?);
        let from = party_from_columns(
            row.try_get("from_kind").map_err(db)?,
            row.try_get("from_entity_ref").map_err(db)?,
        )?;
        let to = party_from_columns(
            row.try_get("to_kind").map_err(db)?,
            row.try_get("to_entity_ref").map_err(db)?,
        )?;
        Ok(SocialRelationshipView {
            assertion: RelationshipAssertion {
                relationship_id: id,
                subject_id: subject,
                relation_type_id: RelationTypeId(row.try_get("relation_type_id").map_err(db)?),
                from,
                to,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: row.try_get("acceptance_state").map_err(db)?,
                integrity_state: row.try_get("integrity_state").map_err(db)?,
                suppression_state: row.try_get("suppression_state").map_err(db)?,
                purge_state: row.try_get("purge_state").map_err(db)?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision: RelationshipRevision {
                relationship_revision_id: revision_id,
                relationship_id: id,
                subject_id: subject,
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(RelationshipRevisionId),
                revision_intent: row.try_get("revision_intent").map_err(db)?,
                degree: row.try_get("degree_value").map_err(db)?,
                epistemic_class: parse_enum(row.try_get("epistemic_class").map_err(db)?)?,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                supports: Vec::new(),
            },
        })
    }

    pub async fn relation_type(
        &self,
        subject: SubjectId,
        id: RelationTypeId,
    ) -> Result<RelationTypeDefinition> {
        let row = sqlx::query("SELECT relation_type_id,key,allowed_from_kinds,allowed_to_kinds,view_kind,inverse_key,degree_kind,degree_config,temporal_kind,source_seed_version_id,source_seed_path,created_at FROM relation_type_definitions WHERE subject_id=$1 AND relation_type_id=$2")
            .bind(subject.0)
            .bind(id.0)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("relation type not found".into()))?;
        load_relation_type_row(&row, subject)
    }

    pub async fn convention(
        &self,
        subject: SubjectId,
        id: LanguageConventionId,
    ) -> Result<SocialConventionView> {
        let row = sqlx::query("SELECT c.key,c.expression,c.scope_kind,c.scope_refs,c.context_scope,c.topic_scope,c.current_revision_id,c.object_epoch,c.acceptance_state,c.integrity_state,c.suppression_state,c.purge_state,c.created_at,r.revision_no,r.parent_revision_id,r.revision_intent,r.meaning,r.pragmatic_role,r.epistemic_class,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id,r.formation_policy_digest FROM language_conventions c JOIN language_convention_revisions r ON r.convention_revision_id=c.current_revision_id WHERE c.subject_id=$1 AND c.convention_id=$2")
            .bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(|| Error::NotFound("LanguageConvention not found".into()))?;
        let revision_id =
            LanguageConventionRevisionId(row.try_get("current_revision_id").map_err(db)?);
        let scope = scope_from_columns(
            row.try_get("scope_kind").map_err(db)?,
            row.try_get("scope_refs").map_err(db)?,
        )?;
        Ok(SocialConventionView {
            convention: LanguageConvention {
                convention_id: id,
                subject_id: subject,
                key: row.try_get("key").map_err(db)?,
                expression: row.try_get("expression").map_err(db)?,
                scope,
                context_scope: row.try_get("context_scope").map_err(db)?,
                topic_scope: row.try_get("topic_scope").map_err(db)?,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: row.try_get("acceptance_state").map_err(db)?,
                integrity_state: row.try_get("integrity_state").map_err(db)?,
                suppression_state: row.try_get("suppression_state").map_err(db)?,
                purge_state: row.try_get("purge_state").map_err(db)?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision: LanguageConventionRevision {
                convention_revision_id: revision_id,
                convention_id: id,
                subject_id: subject,
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(LanguageConventionRevisionId),
                revision_intent: row.try_get("revision_intent").map_err(db)?,
                meaning: row.try_get("meaning").map_err(db)?,
                pragmatic_role: row.try_get("pragmatic_role").map_err(db)?,
                epistemic_class: parse_enum(row.try_get("epistemic_class").map_err(db)?)?,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                formation_policy_digest: row.try_get("formation_policy_digest").map_err(db)?,
                supports: Vec::new(),
            },
            formation_evidence: Vec::new(),
        })
    }
}
