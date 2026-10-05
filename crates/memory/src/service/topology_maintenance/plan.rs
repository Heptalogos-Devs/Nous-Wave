use super::*;
use sqlx::Row;
const CURRENT_COGNITION: &str = r#"
WITH current AS (
 SELECT 'memory_revision'::text kind,r.memory_revision_id id,o.object_epoch epoch,left(r.representation_text,4096) text,r.recorded_at FROM memory_objects o JOIN memory_revisions r ON r.memory_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
 UNION ALL
 SELECT 'cognitive_schema_revision',r.schema_revision_id,o.object_epoch,left(r.structural_claim||' '||r.applicability_description||' '||r.boundary_definition,4096),r.recorded_at FROM cognitive_schemas o JOIN cognitive_schema_revisions r ON r.schema_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
 UNION ALL
 SELECT 'episode_revision',r.episode_revision_id,o.object_epoch,left(COALESCE(r.title,'')||' '||r.boundary_explanation,4096),r.recorded_at FROM episode_objects o JOIN episode_revisions r ON r.episode_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
 UNION ALL
 SELECT 'journal_revision',r.journal_revision_id,o.object_epoch,left(COALESCE(r.title,'')||' '||r.narrative,4096),r.recorded_at FROM journal_objects o JOIN journal_revisions r ON r.journal_revision_id=o.current_revision_id WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
)
SELECT * FROM current ORDER BY CASE WHEN kind=$2 AND id=$3 THEN 0 ELSE 1 END,recorded_at DESC,kind,id LIMIT $4
"#;
impl MemoryService {
    #[expect(
        clippy::too_many_lines,
        reason = "one bounded owner catalog with explicit source families"
    )]
    pub async fn plan_topology(
        &self,
        subject: SubjectId,
        focus: CognitiveRef,
    ) -> Result<TopologyPlan> {
        let snapshot = self.configuration.snapshot_for_subject(subject)?;
        let policy = snapshot.get(TOPOLOGY_MAINTENANCE)?;
        let config_digest = snapshot.digest_for(&[TOPOLOGY_MAINTENANCE.path()])?;
        let sequence = self.store.authority_seq(subject).await?;
        let (focus_kind, focus_value) = reference_parts(&focus);
        if !matches!(
            focus,
            CognitiveRef::MemoryRevision(_)
                | CognitiveRef::CognitiveSchemaRevision(_)
                | CognitiveRef::EpisodeRevision(_)
                | CognitiveRef::JournalRevision(_)
        ) {
            return Err(Error::Invalid(
                "topology focus must be exact cognition".into(),
            ));
        }
        let focus_id: Uuid = focus_value
            .parse()
            .map_err(|_| Error::Invalid("invalid topology focus".into()))?;
        let rows = sqlx::query(CURRENT_COGNITION)
            .bind(subject.0)
            .bind(focus_kind)
            .bind(focus_id)
            .bind(
                i64::try_from(policy.max_cognition + 1)
                    .map_err(|_| Error::Invalid("catalog bound exceeded".into()))?,
            )
            .fetch_all(self.store.pool())
            .await
            .map_err(db)?;
        let mut partial = rows.len() > policy.max_cognition;
        let mut cognition = Vec::new();
        let mut supports = BTreeMap::new();
        let mut entity_refs = std::collections::BTreeSet::new();
        let mut remaining = policy.text_chars;
        let mut roots = BTreeMap::new();
        for (index, row) in rows.into_iter().take(policy.max_cognition).enumerate() {
            let reference = parse_reference(
                &row.try_get::<String, _>("kind").map_err(db)?,
                &row.try_get::<Uuid, _>("id").map_err(db)?.to_string(),
            )?;
            let key = format!("c{index}");
            let raw: String = row.try_get("text").map_err(db)?;
            let text = raw
                .chars()
                .take(remaining.min(if index == 0 { 4096 } else { 512 }))
                .collect::<String>();
            remaining = remaining.saturating_sub(text.chars().count());
            partial |= text != raw;
            let direct = RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: reference.clone(),
                support_role: SupportRole::Direct,
            });
            let summary = self
                .provenance_summary(subject, std::slice::from_ref(&direct))
                .await?;
            let mut provenance_roots = Vec::new();
            for root in summary.roots.into_iter().take(16) {
                let next = format!("r{}", roots.len());
                provenance_roots.push(roots.entry(root.root_key).or_insert(next).clone());
            }
            if supports.len() < policy.max_supports {
                supports.insert(
                    format!("s{}", supports.len()),
                    AssociationSupport::Revision(direct),
                );
            }
            let mut direct_sources = Vec::new();
            match reference {
                CognitiveRef::MemoryRevision(id) => {
                    direct_sources = self.load_supports(id).await?;
                    let entities:Vec<String>=sqlx::query_scalar("SELECT entity_ref FROM memory_revision_aboutness WHERE memory_revision_id=$1 ORDER BY entity_ref LIMIT 16").bind(id.0).fetch_all(self.store.pool()).await.map_err(db)?;
                    entity_refs.extend(entities);
                }
                CognitiveRef::CognitiveSchemaRevision(id) => {
                    direct_sources = self
                        .schema_links(subject, id)
                        .await?
                        .into_iter()
                        .map(|link| link.support)
                        .collect();
                }
                CognitiveRef::EpisodeRevision(id) => {
                    let episode = self.episode_revision(subject, id).await?;
                    direct_sources = episode.supports;
                    direct_sources.extend(episode.members.into_iter().take(16).filter_map(
                        |member| match member.reference {
                            CognitiveRef::Occurrence(id) => {
                                Some(RevisionSupport::Evidence(EvidenceRef {
                                    occurrence_id: id,
                                    locator: EvidenceLocator::WholeOccurrence,
                                    support_role: SupportRole::Direct,
                                }))
                            }
                            _ => None,
                        },
                    ));
                }
                CognitiveRef::JournalRevision(id) => {
                    let journal = self.journal_revision(subject, id).await?;
                    direct_sources = journal
                        .points
                        .into_iter()
                        .flat_map(|point| point.supports)
                        .take(16)
                        .collect();
                }
                _ => {}
            }
            for support in direct_sources {
                if supports.len() == policy.max_supports {
                    partial = true;
                    break;
                }
                supports.insert(
                    format!("s{}", supports.len()),
                    AssociationSupport::Revision(support),
                );
            }
            let (kind, value) = reference_parts(&reference);
            let use_rows=sqlx::query("SELECT use_kind,count(*)::bigint count FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind=$2 AND ref_value=$3 GROUP BY use_kind ORDER BY use_kind")
                .bind(subject.0).bind(&kind).bind(&value).fetch_all(self.store.pool()).await.map_err(db)?;
            let mut use_summary = BTreeMap::new();
            for row in use_rows {
                use_summary.insert(
                    row.try_get("use_kind").map_err(db)?,
                    u64::try_from(row.try_get::<i64, _>("count").map_err(db)?)
                        .map_err(|_| Error::Infrastructure("invalid use count".into()))?,
                );
            }
            let uses=sqlx::query("SELECT consumer_ref,event_id FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind=$2 AND ref_value=$3 AND use_kind IN ('referenced','acted_on','result_supported','corrected','pinned') ORDER BY occurred_at DESC,event_id LIMIT 4")
                .bind(subject.0).bind(kind).bind(value).fetch_all(self.store.pool()).await.map_err(db)?;
            for row in uses {
                if supports.len() == policy.max_supports {
                    partial = true;
                    break;
                }
                supports.insert(
                    format!("s{}", supports.len()),
                    AssociationSupport::UseEvent(UseEventRef {
                        subject_id: subject,
                        consumer_ref: row.try_get("consumer_ref").map_err(db)?,
                        event_id: UseEventId(row.try_get("event_id").map_err(db)?),
                    }),
                );
            }
            cognition.push(TopologyCognition {
                key,
                reference,
                epoch: row.try_get("epoch").map_err(db)?,
                text,
                semantic_similarity: None,
                use_summary,
                provenance_roots,
            });
        }
        if cognition.first().is_none_or(|c| c.reference != focus) {
            return Err(Error::NotFound(
                "topology focus is no longer current and eligible".into(),
            ));
        }
        let texts = cognition.iter().map(|c| c.text.clone()).collect::<Vec<_>>();
        let scores = self
            .store
            .cached_semantic_scores(subject, &texts[0], &texts)
            .await?;
        partial |= scores.iter().any(Option::is_none);
        for (c, score) in cognition.iter_mut().zip(scores) {
            c.semantic_similarity = score;
        }
        let rows=sqlx::query("SELECT t.tag_id,t.current_revision_id,r.label,r.description,r.kind_hint,COALESCE(v.aliases,ARRAY[]::text[]) aliases FROM tags t JOIN tag_revisions r ON r.tag_revision_id=t.current_revision_id LEFT JOIN lexical_bindings b ON b.object_kind='tag' AND b.canonical_ref=t.tag_id::text LEFT JOIN lexical_visibility v ON v.lexical_ref=b.lexical_ref AND v.subject_id=t.subject_id WHERE t.subject_id=$1 AND t.status='active' ORDER BY t.tag_id LIMIT $2")
            .bind(subject.0).bind(i64::try_from(policy.max_tags+1).map_err(|_|Error::Invalid("tag bound exceeded".into()))?).fetch_all(self.store.pool()).await.map_err(db)?;
        partial |= rows.len() > policy.max_tags;
        let mut tags = Vec::new();
        for (index, row) in rows.into_iter().take(policy.max_tags).enumerate() {
            tags.push(TopologyTag {
                key: format!("t{index}"),
                target: TagExpectation {
                    tag_id: TagId(row.try_get("tag_id").map_err(db)?),
                    expected_revision_id: row.try_get("current_revision_id").map_err(db)?,
                },
                content: TagContent {
                    label: row.try_get("label").map_err(db)?,
                    description: row.try_get("description").map_err(db)?,
                    kind_hint: row.try_get("kind_hint").map_err(db)?,
                },
                aliases: row
                    .try_get::<Vec<String>, _>("aliases")
                    .map_err(db)?
                    .into_iter()
                    .take(16)
                    .collect(),
            });
        }
        let mut entities = BTreeMap::new();
        let entity_refs = entity_refs
            .into_iter()
            .take(16)
            .map(|r| EntityRef::new(r).map(CognitiveRef::Entity))
            .collect::<Result<Vec<_>>>()?;
        for (index, descriptor) in self
            .store
            .query_descriptors(subject, &entity_refs)
            .await?
            .into_iter()
            .enumerate()
        {
            if let CognitiveRef::Entity(entity) = descriptor.reference {
                entities.insert(format!("e{index}"), (entity, descriptor.text));
            }
        }
        let refs = cognition
            .iter()
            .map(|c| (c.reference.clone(), c.key.clone()))
            .chain(
                tags.iter()
                    .map(|t| (CognitiveRef::Tag(t.target.tag_id), t.key.clone())),
            )
            .chain(
                entities
                    .iter()
                    .map(|(key, (entity, _))| (CognitiveRef::Entity(entity.clone()), key.clone())),
            )
            .collect::<std::collections::HashMap<_, _>>();
        let neighborhood = self
            .association_neighborhood(subject, focus.clone(), 64, 1)
            .await?;
        partial |= neighborhood.truncated;
        let mut associations = Vec::new();
        for association in neighborhood.associations {
            let (Some(from), Some(to)) = (refs.get(&association.from), refs.get(&association.to))
            else {
                partial = true;
                continue;
            };
            if associations.len() == policy.max_associations {
                partial = true;
                break;
            }
            associations.push(TopologyAssociation {
                key: format!("a{}", associations.len()),
                id: association.association_evidence_id,
                from: from.clone(),
                to: to.clone(),
                relation: association.relation_kind,
            });
        }
        if self.store.authority_seq(subject).await? != sequence {
            return Err(Error::Conflict("topology planning snapshot changed".into()));
        }
        Ok(TopologyPlan {
            subject,
            focus,
            authority_seq: sequence,
            config_digest,
            policy,
            cognition,
            tags,
            associations,
            entities,
            supports,
            partial,
        })
    }
}
