use super::*;
use sqlx::Row;
impl MemoryService {
    pub(super) async fn memory_topology_context(
        &self,
        subject: SubjectId,
        id: MemoryRevisionId,
    ) -> Result<TopologyContext> {
        let row=sqlx::query("SELECT o.cognitive_role,r.semantic_role,r.valid_time_kind,r.valid_time_start,r.valid_time_end FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2")
            .bind(subject.0).bind(id.0).fetch_one(self.store.pool()).await.map_err(db)?;
        Ok(TopologyContext {
            cognitive_role: Some(row.try_get("cognitive_role").map_err(db)?),
            semantic_role: Some(row.try_get("semantic_role").map_err(db)?),
            time: Some(temporal_from_columns(
                row.try_get("valid_time_kind").map_err(db)?,
                row.try_get("valid_time_start").map_err(db)?,
                row.try_get("valid_time_end").map_err(db)?,
            )?),
            ..Default::default()
        })
    }
    pub(super) async fn schema_topology_context(
        &self,
        subject: SubjectId,
        id: CognitiveSchemaRevisionId,
    ) -> Result<TopologyContext> {
        let row=sqlx::query("SELECT aboutness,valid_time_kind,valid_time_start,valid_time_end FROM cognitive_schema_revisions WHERE schema_revision_id=$1 AND schema_id IN (SELECT schema_id FROM cognitive_schemas WHERE subject_id=$2)")
            .bind(id.0).bind(subject.0).fetch_one(self.store.pool()).await.map_err(db)?;
        Ok(TopologyContext {
            time: Some(temporal_from_columns(
                row.try_get("valid_time_kind").map_err(db)?,
                row.try_get("valid_time_start").map_err(db)?,
                row.try_get("valid_time_end").map_err(db)?,
            )?),
            entities: row
                .try_get::<Vec<String>, _>("aboutness")
                .map_err(db)?
                .into_iter()
                .take(16)
                .map(EntityRef::new)
                .collect::<Result<Vec<_>>>()?,
            ..Default::default()
        })
    }
    pub(super) async fn topology_source_context(
        &self,
        subject: SubjectId,
        supports: &BTreeMap<String, AssociationSupport>,
    ) -> Result<BTreeMap<String, serde_json::Value>> {
        let mut result = BTreeMap::new();
        let mut occurrence_keys = BTreeMap::new();
        for (key, support) in supports {
            let AssociationSupport::Revision(RevisionSupport::Evidence(e)) = support else {
                continue;
            };
            let row=sqlx::query("SELECT o.source_class,o.occurred_time_kind,o.occurred_time_start,o.occurred_time_end,o.observed_at,a.content_hash,a.byte_length,(a.media_type LIKE 'text/%' OR a.media_type IN ('application/json','application/xml')) raw_text FROM observation_occurrences o LEFT JOIN artifacts a USING(artifact_id) WHERE o.subject_id=$1 AND o.occurrence_id=$2")
                .bind(subject.0).bind(e.occurrence_id.0).fetch_one(self.store.pool()).await.map_err(db)?;
            let text = if row.try_get::<Option<bool>, _>("raw_text").map_err(db)? == Some(true) {
                self.objects
                    .read_text_prefix(
                        &row.try_get::<String, _>("content_hash").map_err(db)?,
                        u64::try_from(row.try_get::<i64, _>("byte_length").map_err(db)?)
                            .map_err(|_| Error::Infrastructure("invalid source length".into()))?,
                        4096,
                    )
                    .await?
            } else {
                self.store
                    .query_descriptors(subject, &[CognitiveRef::Occurrence(e.occurrence_id)])
                    .await?
                    .into_iter()
                    .next()
                    .map(|descriptor| descriptor.text)
            };
            let next = format!("o{}", occurrence_keys.len());
            let occurrence_key = occurrence_keys.entry(e.occurrence_id.0).or_insert(next);
            let occurred = temporal_from_columns(
                row.try_get("occurred_time_kind").map_err(db)?,
                row.try_get("occurred_time_start").map_err(db)?,
                row.try_get("occurred_time_end").map_err(db)?,
            )?;
            result.insert(key.clone(),serde_json::json!({"occurrenceKey":occurrence_key,"text":text,"sourceClass":row.try_get::<String,_>("source_class").map_err(db)?,"occurredTime":occurred,"observedAt":row.try_get::<DateTime<Utc>,_>("observed_at").map_err(db)?,"supportRole":e.support_role}));
        }
        Ok(result)
    }
}
