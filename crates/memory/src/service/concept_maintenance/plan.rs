use super::*;
use sqlx::Row;
const CURRENT_FOCUS: &str = r#"
WITH current AS (
 SELECT 'memory_revision'::text kind,o.current_revision_id id,o.object_epoch epoch FROM memory_objects o WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
 UNION ALL SELECT 'cognitive_schema_revision',o.current_revision_id,o.object_epoch FROM cognitive_schemas o WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
 UNION ALL SELECT 'episode_revision',o.current_revision_id,o.object_epoch FROM episode_objects o WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
 UNION ALL SELECT 'journal_revision',o.current_revision_id,o.object_epoch FROM journal_objects o WHERE o.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal'
) SELECT epoch FROM current WHERE kind=$2 AND id=$3
"#;
impl MemoryService {
    pub(super) async fn current_concept_epoch(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<Option<i64>> {
        let (kind, value) = reference_parts(reference);
        let id: Uuid = value
            .parse()
            .map_err(|_| Error::Invalid("concept focus must be exact cognition".into()))?;
        sqlx::query_scalar(CURRENT_FOCUS)
            .bind(subject.0)
            .bind(kind)
            .bind(id)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "one local plan joins focus, one-hop endpoints and typed catalogs"
    )]
    pub async fn plan_concepts(
        &self,
        subject: SubjectId,
        focus: CognitiveRef,
    ) -> Result<ConceptPlan> {
        let snapshot = self.configuration.snapshot_for_subject(subject)?;
        let policy = snapshot.get(CONCEPT_MAINTENANCE)?;
        let config_digest = snapshot.digest_for(&[CONCEPT_MAINTENANCE.path(), ACCRETION.path()])?;
        let sequence = self.store.authority_seq(subject).await?;
        let epoch = self
            .current_concept_epoch(subject, &focus)
            .await?
            .ok_or_else(|| {
                Error::NotFound("concept focus is no longer current and eligible".into())
            })?;
        let text = self
            .store
            .query_descriptors(subject, std::slice::from_ref(&focus))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound("concept focus descriptor unavailable".into()))?
            .text
            .chars()
            .take(4096)
            .collect::<String>();
        let (source_supports, entities) = self.concept_focus_sources(subject, &focus).await?;
        let mut supports = BTreeMap::new();
        supports.insert(
            "s0".into(),
            AssociationSupport::Revision(RevisionSupport::CognitionDependency(
                CognitionDependency {
                    target_revision: focus.clone(),
                    support_role: SupportRole::Direct,
                },
            )),
        );
        let mut support_keys = std::collections::HashSet::new();
        for support in source_supports.into_iter().take(12) {
            if support_keys.insert(support.canonical_key()) {
                supports.insert(
                    format!("s{}", supports.len()),
                    AssociationSupport::Revision(support),
                );
            }
        }
        let mut references = BTreeMap::from([("c0".into(), focus.clone())]);
        let neighborhood = self
            .association_neighborhood(subject, focus.clone(), 16, 1)
            .await?;
        let mut partial = neighborhood.truncated;
        let provenance = self
            .provenance_summary(
                subject,
                &supports
                    .values()
                    .filter_map(|support| match support {
                        AssociationSupport::Revision(revision) => Some(revision.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            )
            .await?;
        let distinct_roots = provenance
            .roots
            .iter()
            .filter(|root| root.certainty == EvidenceRootCertainty::Known)
            .count();
        let focus_context = self
            .concept_focus_context(subject, &focus, &supports)
            .await?;
        let mut cognition = vec![
            serde_json::json!({"key":"c0","kind":reference_parts(&focus).0,"epoch":epoch,"text":text,"exactSupportKey":"s0","context":focus_context,"distinctRoots":distinct_roots}),
        ];
        for reference in neighborhood.nodes.iter().filter(|r| *r != &focus).take(8) {
            if !matches!(
                reference,
                CognitiveRef::MemoryRevision(_)
                    | CognitiveRef::CognitiveSchemaRevision(_)
                    | CognitiveRef::EpisodeRevision(_)
                    | CognitiveRef::JournalRevision(_)
            ) {
                continue;
            }
            if self
                .current_concept_epoch(subject, reference)
                .await?
                .is_none()
            {
                continue;
            }
            if let Some(descriptor) = self
                .store
                .query_descriptors(subject, std::slice::from_ref(reference))
                .await?
                .into_iter()
                .next()
            {
                let key = format!("c{}", cognition.len());
                let support_key = format!("s{}", supports.len());
                supports.insert(
                    support_key.clone(),
                    AssociationSupport::Revision(RevisionSupport::CognitionDependency(
                        CognitionDependency {
                            target_revision: reference.clone(),
                            support_role: SupportRole::Contextual,
                        },
                    )),
                );
                references.insert(key.clone(), reference.clone());
                cognition.push(serde_json::json!({"key":key,"kind":reference_parts(reference).0,"text":descriptor.text.chars().take(512).collect::<String>(),"exactSupportKey":support_key}));
            }
        }
        let mut entity_input = Vec::new();
        for descriptor in self
            .store
            .query_descriptors(
                subject,
                &entities
                    .into_iter()
                    .map(CognitiveRef::Entity)
                    .collect::<Vec<_>>(),
            )
            .await?
        {
            let key = format!("e{}", entity_input.len());
            references.insert(key.clone(), descriptor.reference);
            entity_input.push(serde_json::json!({"key":key,"text":descriptor.text.chars().take(256).collect::<String>()}));
        }
        let mut tags = self
            .concept_tag_candidates(subject, &focus, &text, policy.max_candidates)
            .await?;
        for tag in &tags {
            references.insert(tag.key.clone(), CognitiveRef::Tag(tag.target.tag_id));
        }
        let mut associations = Vec::new();
        for association in neighborhood.associations {
            let lookup = |reference: &CognitiveRef| {
                references
                    .iter()
                    .find(|(_, r)| *r == reference)
                    .map(|(key, _)| key.clone())
            };
            if let (Some(from), Some(to)) = (lookup(&association.from), lookup(&association.to)) {
                associations.push(ConceptAssociation {
                    key: format!("a{}", associations.len()),
                    id: association.association_evidence_id,
                    from,
                    to,
                    relation: association.relation_kind,
                });
            } else {
                partial = true;
            }
        }
        let (kind, value) = reference_parts(&focus);
        let use_rows=sqlx::query("SELECT consumer_ref,event_id,use_kind FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind=$2 AND ref_value=$3 AND use_kind IN ('referenced','acted_on','result_supported','corrected','pinned') ORDER BY occurred_at DESC,event_id LIMIT 4")
            .bind(subject.0).bind(kind).bind(value).fetch_all(self.store.pool()).await.map_err(db)?;
        for row in use_rows {
            supports.insert(
                format!("s{}", supports.len()),
                AssociationSupport::UseEvent(UseEventRef {
                    subject_id: subject,
                    consumer_ref: row.try_get("consumer_ref").map_err(db)?,
                    event_id: UseEventId(row.try_get("event_id").map_err(db)?),
                }),
            );
        }
        let accretion_policy = snapshot.get(ACCRETION)?;
        for tag in &mut tags {
            tag.accretion = self
                .accretion_signal(subject, &CognitiveRef::Tag(tag.target.tag_id))
                .await?;
        }
        let mut merge_candidates = Vec::new();
        let mut split_candidates = Vec::new();
        for (index, left) in tags.iter().enumerate() {
            if left.accretion.as_ref().is_some_and(|s| s.broad_center) {
                split_candidates.push(left.key.clone());
            }
            for right in tags.iter().skip(index + 1) {
                let (Some(a), Some(b)) = (&left.accretion, &right.accretion) else {
                    continue;
                };
                if a.partial || b.partial {
                    continue;
                }
                let a = a
                    .member_refs
                    .iter()
                    .collect::<std::collections::HashSet<_>>();
                let b = b
                    .member_refs
                    .iter()
                    .collect::<std::collections::HashSet<_>>();
                let union = a.union(&b).count();
                if union > 0 && a.intersection(&b).count() as f64 / union as f64 >= 0.8 {
                    merge_candidates.push((left.key.clone(), right.key.clone()));
                }
            }
        }
        let source_context = self.concept_source_text(subject, &supports).await?;
        let model_input = serde_json::json!({"focusKey":"c0","policy":policy,"partial":partial,"cognition":cognition,"entities":entity_input,
            "tags":tags.iter().map(|tag|serde_json::json!({"key":tag.key,"content":tag.content,"aliases":tag.aliases,"attached":tag.attached,"semanticSimilarity":tag.semantic_score,"accretion":tag.accretion.as_ref().map(|s|serde_json::json!({"distinctRoots":s.independent_roots,"currentMembers":s.attached_cognition,"episodeRecurrence":s.cross_episode_recurrence,"observedSpanSeconds":s.observed_span_seconds,"associationDegree":s.association_degree,"meaningfulUse":s.meaningful_use,"counterevidence":s.counterevidence,"coherence":s.semantic_coherence,"genericity":s.broad_center,"reviewPriority":s.review_priority(&accretion_policy),"partial":s.partial}))})).collect::<Vec<_>>(),
            "associations":associations.iter().map(|a|serde_json::json!({"key":a.key,"from":a.from,"to":a.to,"relation":a.relation})).collect::<Vec<_>>(),
            "supports":supports.iter().map(|(key,s)|serde_json::json!({"key":key,"kind":match s {AssociationSupport::Revision(RevisionSupport::CognitionDependency(_))=>"exact_cognition",AssociationSupport::Revision(_)=>"source_evidence",AssociationSupport::UseEvent(_)=>"meaningful_use"}})).collect::<Vec<_>>(),"sourceContext":source_context,"mergeCandidates":merge_candidates,"splitCandidates":split_candidates});
        if self.store.authority_seq(subject).await? != sequence {
            return Err(Error::Conflict("concept planning snapshot changed".into()));
        }
        Ok(ConceptPlan {
            subject,
            authority_seq: sequence,
            config_digest,
            policy,
            references,
            tags,
            associations,
            supports,
            model_input,
        })
    }
    async fn concept_tag_candidates(
        &self,
        subject: SubjectId,
        focus: &CognitiveRef,
        text: &str,
        limit: usize,
    ) -> Result<Vec<ConceptTag>> {
        let (kind, value) = reference_parts(focus);
        let rows=sqlx::query("SELECT t.tag_id,t.current_revision_id,r.label,r.description,r.kind_hint,COALESCE(v.aliases,ARRAY[]::text[]) aliases, EXISTS(SELECT 1 FROM association_evidence a WHERE a.subject_id=$1 AND a.from_ref_kind=$2 AND a.from_ref=$3 AND a.to_ref_kind='tag' AND canonical_tag($1,CASE WHEN a.to_ref_kind='tag' THEN a.to_ref::uuid END)=t.tag_id AND a.relation_kind='tag_attachment' AND a.polarity='positive' AND a.revoked_at IS NULL) OR EXISTS(SELECT 1 FROM memory_revision_tags m WHERE $2='memory_revision' AND m.memory_revision_id::text=$3 AND canonical_tag($1,m.tag_id)=t.tag_id) attached, ts_rank_cd(to_tsvector('simple',r.label||' '||COALESCE(r.description,'')||' '||COALESCE(v.display_name,'')||' '||array_to_string(COALESCE(v.aliases,ARRAY[]::text[]),' ')),to_tsquery('simple',replace(plainto_tsquery('simple',$4)::text,' & ',' | ')))::float8 lexical_score FROM tags t JOIN tag_revisions r ON r.tag_revision_id=t.current_revision_id LEFT JOIN lexical_bindings b ON b.object_kind='tag' AND b.canonical_ref=t.tag_id::text LEFT JOIN lexical_visibility v ON v.lexical_ref=b.lexical_ref AND v.subject_id=t.subject_id WHERE t.subject_id=$1 AND t.status='active' ORDER BY attached DESC,lexical_score DESC,t.created_at DESC,t.tag_id LIMIT 32")
            .bind(subject.0).bind(kind).bind(value).bind(text).fetch_all(self.store.pool()).await.map_err(db)?;
        let mut candidates = rows
            .into_iter()
            .map(|row| {
                Ok(ConceptTag {
                    key: String::new(),
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
                        .take(8)
                        .map(|alias| alias.chars().take(128).collect())
                        .collect(),
                    attached: row.try_get("attached").map_err(db)?,
                    lexical_score: row.try_get("lexical_score").map_err(db)?,
                    semantic_score: None,
                    accretion: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let texts = candidates
            .iter()
            .map(|t| {
                format!(
                    "{} {}",
                    t.content.label,
                    t.content.description.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>();
        let scores = self
            .store
            .cached_semantic_scores(subject, text, &texts)
            .await?;
        for (candidate, score) in candidates.iter_mut().zip(scores) {
            candidate.semantic_score = score;
        }
        candidates.retain(|tag| {
            tag.attached
                || tag.lexical_score > 0.0
                || tag.semantic_score.is_some_and(|score| score >= 0.5)
        });
        candidates.sort_by(|a, b| {
            b.attached
                .cmp(&a.attached)
                .then_with(|| b.lexical_score.total_cmp(&a.lexical_score))
                .then_with(|| {
                    b.semantic_score
                        .unwrap_or(-1.0)
                        .total_cmp(&a.semantic_score.unwrap_or(-1.0))
                })
                .then_with(|| a.target.tag_id.0.cmp(&b.target.tag_id.0))
        });
        candidates.truncate(limit);
        for (index, tag) in candidates.iter_mut().enumerate() {
            tag.key = format!("t{index}");
            tag.content.label = tag.content.label.chars().take(256).collect();
            tag.content.description = tag
                .content
                .description
                .take()
                .map(|text| text.chars().take(512).collect());
        }
        Ok(candidates)
    }
}
