// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::*;
use sqlx::Row;
impl MemoryService {
    pub(super) async fn concept_focus_sources(
        &self,
        subject: SubjectId,
        focus: &CognitiveRef,
    ) -> Result<(Vec<RevisionSupport>, Vec<EntityRef>)> {
        match focus {
            CognitiveRef::MemoryRevision(id) => {
                let entities:Vec<String> = sqlx::query_scalar("SELECT entity_ref FROM memory_revision_aboutness WHERE memory_revision_id=$1 ORDER BY entity_ref LIMIT 8")
                    .bind(id.0).fetch_all(self.store.pool()).await.map_err(db)?;
                Ok((
                    self.load_supports(*id).await?,
                    entities
                        .into_iter()
                        .map(EntityRef::new)
                        .collect::<Result<_>>()?,
                ))
            }
            CognitiveRef::CognitiveSchemaRevision(id) => {
                let entities: Vec<String> = sqlx::query_scalar(
                    "SELECT aboutness FROM cognitive_schema_revisions WHERE schema_revision_id=$1",
                )
                .bind(id.0)
                .fetch_one(self.store.pool())
                .await
                .map_err(db)?;
                Ok((
                    self.schema_links(subject, *id)
                        .await?
                        .into_iter()
                        .map(|link| link.support)
                        .collect(),
                    entities
                        .into_iter()
                        .take(8)
                        .map(EntityRef::new)
                        .collect::<Result<_>>()?,
                ))
            }
            CognitiveRef::EpisodeRevision(id) => {
                let episode = self.episode_revision(subject, *id).await?;
                let mut supports = episode.supports;
                supports.extend(episode.members.into_iter().take(8).filter_map(|member| {
                    match member.reference {
                        CognitiveRef::Occurrence(id) => {
                            Some(RevisionSupport::Evidence(EvidenceRef {
                                occurrence_id: id,
                                locator: EvidenceLocator::WholeOccurrence,
                                support_role: SupportRole::Direct,
                            }))
                        }
                        _ => None,
                    }
                }));
                Ok((supports, Vec::new()))
            }
            CognitiveRef::JournalRevision(id) => {
                let journal = self.journal_revision(subject, *id).await?;
                Ok((
                    journal
                        .points
                        .into_iter()
                        .flat_map(|point| point.supports)
                        .take(16)
                        .collect(),
                    Vec::new(),
                ))
            }
            _ => Err(Error::Invalid(
                "concept focus must be exact cognition".into(),
            )),
        }
    }
    pub(super) async fn concept_focus_context(
        &self,
        subject: SubjectId,
        focus: &CognitiveRef,
        supports: &BTreeMap<String, AssociationSupport>,
    ) -> Result<serde_json::Value> {
        let source_keys = supports
            .iter()
            .filter_map(|(key, support)| {
                matches!(
                    support,
                    AssociationSupport::Revision(RevisionSupport::Evidence(_))
                )
                .then_some(key)
            })
            .collect::<Vec<_>>();
        let context = match focus {
            CognitiveRef::MemoryRevision(id) => {
                let row=sqlx::query("SELECT o.cognitive_role,r.semantic_role,r.valid_time_kind,r.valid_time_start,r.valid_time_end FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2")
                    .bind(subject.0).bind(id.0).fetch_one(self.store.pool()).await.map_err(db)?;
                serde_json::json!({"cognitiveRole":row.try_get::<String,_>("cognitive_role").map_err(db)?,"semanticRole":row.try_get::<String,_>("semantic_role").map_err(db)?,"validTime":temporal_from_columns(row.try_get("valid_time_kind").map_err(db)?,row.try_get("valid_time_start").map_err(db)?,row.try_get("valid_time_end").map_err(db)?)?})
            }
            CognitiveRef::CognitiveSchemaRevision(id) => {
                let row=sqlx::query("SELECT applicability_description,boundary_definition,valid_time_kind,valid_time_start,valid_time_end FROM cognitive_schema_revisions WHERE schema_revision_id=$1")
                    .bind(id.0).fetch_one(self.store.pool()).await.map_err(db)?;
                serde_json::json!({"applicability":row.try_get::<String,_>("applicability_description").map_err(db)?.chars().take(512).collect::<String>(),"boundary":row.try_get::<String,_>("boundary_definition").map_err(db)?.chars().take(512).collect::<String>(),"validTime":temporal_from_columns(row.try_get("valid_time_kind").map_err(db)?,row.try_get("valid_time_start").map_err(db)?,row.try_get("valid_time_end").map_err(db)?)?})
            }
            CognitiveRef::EpisodeRevision(id) => {
                let episode = self.episode_revision(subject, *id).await?;
                let members=episode.members.into_iter().take(8).enumerate().map(|(ordinal,member)|{
                    let keys=supports.iter().filter_map(|(key,support)|match (&member.reference,support) {
                        (CognitiveRef::Occurrence(id),AssociationSupport::Revision(RevisionSupport::Evidence(e))) if *id==e.occurrence_id=>Some(key),
                        (reference,AssociationSupport::Revision(RevisionSupport::CognitionDependency(d))) if *reference==d.target_revision=>Some(key),
                        _=>None,
                    }).collect::<Vec<_>>();
                    serde_json::json!({"ordinal":ordinal,"role":member.role.chars().take(128).collect::<String>(),"supportKeys":keys})
                }).collect::<Vec<_>>();
                serde_json::json!({"experienceTime":episode.revision.experience_time,"orderedMembers":members})
            }
            CognitiveRef::JournalRevision(id) => {
                serde_json::json!({"temporalScope":self.journal_revision(subject,*id).await?.revision.temporal_scope})
            }
            _ => {
                return Err(Error::Invalid(
                    "concept focus must be exact cognition".into(),
                ));
            }
        };
        Ok(serde_json::json!({"sourceSupportKeys":source_keys,"ownerContext":context}))
    }
    pub(super) async fn concept_source_text(
        &self,
        subject: SubjectId,
        supports: &BTreeMap<String, AssociationSupport>,
    ) -> Result<serde_json::Value> {
        let mut result = serde_json::Map::new();
        let mut remaining: usize = 8192;
        for (key, support) in supports {
            let AssociationSupport::Revision(RevisionSupport::Evidence(e)) = support else {
                continue;
            };
            if remaining == 0 {
                break;
            }
            let row=sqlx::query("SELECT o.source_class,o.occurred_time_kind,o.occurred_time_start,o.occurred_time_end,o.observed_at,a.content_hash,a.byte_length,(a.media_type LIKE 'text/%' OR a.media_type IN ('application/json','application/xml')) raw_text FROM observation_occurrences o LEFT JOIN artifacts a USING(artifact_id) WHERE o.subject_id=$1 AND o.occurrence_id=$2")
                .bind(subject.0).bind(e.occurrence_id.0).fetch_one(self.store.pool()).await.map_err(db)?;
            let text = if row.try_get::<Option<bool>, _>("raw_text").map_err(db)? == Some(true) {
                self.objects
                    .read_text_prefix(
                        &row.try_get::<String, _>("content_hash").map_err(db)?,
                        u64::try_from(row.try_get::<i64, _>("byte_length").map_err(db)?)
                            .map_err(|_| Error::Infrastructure("invalid source size".into()))?,
                        remaining.min(2048) as u64,
                    )
                    .await?
                    .unwrap_or_default()
            } else {
                self.store
                    .query_descriptors(subject, &[CognitiveRef::Occurrence(e.occurrence_id)])
                    .await?
                    .into_iter()
                    .next()
                    .map(|d| d.text)
                    .unwrap_or_default()
            };
            let text = text.chars().take(remaining.min(2048)).collect::<String>();
            remaining -= text.chars().count();
            result.insert(key.clone(),serde_json::json!({"text":text,"occurredTime":temporal_from_columns(row.try_get("occurred_time_kind").map_err(db)?,row.try_get("occurred_time_start").map_err(db)?,row.try_get("occurred_time_end").map_err(db)?)?,"sourceClass":row.try_get::<String,_>("source_class").map_err(db)?,"observedAt":row.try_get::<DateTime<Utc>,_>("observed_at").map_err(db)?}));
        }
        Ok(serde_json::Value::Object(result))
    }
}
