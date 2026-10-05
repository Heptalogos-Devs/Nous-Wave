//! Rebuildable long-term aggregation signals, never an Authority strength or truth field.
use super::*;
use nous_configuration::*;
use std::collections::{BTreeSet, HashMap};
pub const ACCRETION: ConfigKey<AccretionPolicy> = ConfigKey::new("maintenance.accretion");
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccretionPolicy {
    pub enabled: bool,
    pub max_centers: usize,
    pub max_members: usize,
    pub generic_degree: usize,
    pub priority_bonus: u32,
    pub merge_overlap: f64,
    pub split_coherence: f64,
}
impl Default for AccretionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_centers: 64,
            max_members: 32,
            generic_degree: 32,
            priority_bonus: 10,
            merge_overlap: 0.8,
            split_coherence: 0.35,
        }
    }
}
pub(super) fn register_accretion_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        ACCRETION,
        "memory",
        "Derived cognitive accretion catalog and maintenance priority limits.",
        AccretionPolicy::default(),
        ConfigExposure::Developer,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |p| {
            if !(1..=64).contains(&p.max_centers)
                || !(1..=64).contains(&p.max_members)
                || !(4..=256).contains(&p.generic_degree)
                || p.priority_bonus > 20
                || !p.merge_overlap.is_finite()
                || !(0.5..=1.0).contains(&p.merge_overlap)
                || !p.split_coherence.is_finite()
                || !(0.0..=0.8).contains(&p.split_coherence)
            {
                return Err(Error::Invalid("invalid accretion envelope".into()));
            }
            Ok(())
        },
    )
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccretionSignal {
    pub independent_roots: usize,
    pub attached_cognition: usize,
    pub cross_episode_recurrence: usize,
    pub observed_span_seconds: i64,
    pub association_degree: usize,
    pub relation_diversity: usize,
    pub meaningful_use: u64,
    pub counterevidence: u64,
    pub semantic_coherence: Option<f64>,
    pub usefulness: f64,
    pub review_priority: u32,
    pub broad_center: bool,
    pub partial: bool,
    pub member_keys: Vec<String>,
}
impl MemoryService {
    #[expect(
        clippy::too_many_lines,
        reason = "one bounded derived read combines independent evidence, recurrence, usage and coherence"
    )]
    pub async fn accretion_signals(
        &self,
        subject: SubjectId,
        centers: &[CognitiveRef],
    ) -> Result<HashMap<CognitiveRef, AccretionSignal>> {
        let policy = self
            .configuration
            .snapshot_for_subject(subject)?
            .get(ACCRETION)?;
        if centers.len() > policy.max_centers {
            return Err(Error::Invalid("accretion center budget exceeded".into()));
        }
        if !policy.enabled {
            return Ok(HashMap::new());
        }
        let mut result = HashMap::new();
        for center in centers {
            self.store.validate_reference(subject, center).await?;
            let mut members = self
                .accretion_members(subject, center, policy.max_members)
                .await?;
            let total = members.1;
            let partial = total > members.0.len();
            let refs = std::mem::take(&mut members.0);
            let mut roots = BTreeSet::new();
            let mut occurrences = BTreeSet::new();
            let mut counterevidence = 0u64;
            for reference in &refs {
                let direct = RevisionSupport::CognitionDependency(CognitionDependency {
                    target_revision: reference.clone(),
                    support_role: SupportRole::Direct,
                });
                let summary = self
                    .provenance_summary(subject, std::slice::from_ref(&direct))
                    .await?;
                roots.extend(
                    summary
                        .roots
                        .into_iter()
                        .filter(|r| r.certainty == EvidenceRootCertainty::Known)
                        .map(|r| r.root_key),
                );
                let (source_ids, contradictions) =
                    self.accretion_sources(subject, reference).await?;
                occurrences.extend(source_ids);
                counterevidence += contradictions;
            }
            let (kinds, values): (Vec<_>, Vec<_>) = refs.iter().map(reference_parts).unzip();
            let use_rows=sqlx::query("WITH members AS (SELECT * FROM unnest($2::text[],$3::text[]) AS m(kind,value)) SELECT use_kind,count(*)::bigint count FROM cognitive_use_events e WHERE e.subject_id=$1 AND EXISTS(SELECT 1 FROM members m WHERE m.kind=e.ref_kind AND m.value=e.ref_value) GROUP BY use_kind")
                .bind(subject.0).bind(&kinds).bind(&values).fetch_all(self.store.pool()).await.map_err(db)?;
            let mut meaningful = 0u64;
            for row in use_rows {
                let count = u64::try_from(row.try_get::<i64, _>("count").map_err(db)?)
                    .map_err(|_| Error::Infrastructure("invalid use count".into()))?;
                let kind: String = row.try_get("use_kind").map_err(db)?;
                if matches!(
                    kind.as_str(),
                    "referenced" | "acted_on" | "result_supported" | "corrected" | "pinned"
                ) {
                    meaningful += count;
                }
                if kind == "result_refuted" {
                    counterevidence += count;
                }
            }
            let episodes:i64=sqlx::query_scalar("WITH members AS (SELECT * FROM unnest($2::text[],$3::text[]) AS m(kind,value)) SELECT count(DISTINCT r.episode_revision_id)::bigint FROM episode_revisions r JOIN episode_objects o ON o.current_revision_id=r.episode_revision_id JOIN episode_revision_members e USING(episode_revision_id) WHERE r.subject_id=$1 AND o.acceptance_state='accepted' AND o.integrity_state='valid' AND o.suppression_state='normal' AND o.purge_state='normal' AND (EXISTS(SELECT 1 FROM members m WHERE m.kind=e.ref_kind AND m.value=e.ref_value) OR (e.ref_kind='occurrence' AND e.ref_value=ANY($4::text[])))")
                .bind(subject.0).bind(&kinds).bind(&values).bind(occurrences.iter().map(ToString::to_string).collect::<Vec<_>>()).fetch_one(self.store.pool()).await.map_err(db)?;
            let span:i64=sqlx::query_scalar("SELECT COALESCE(extract(epoch FROM max(observed_at)-min(observed_at))::bigint,0) FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=ANY($2::uuid[])")
                .bind(subject.0).bind(occurrences.iter().copied().collect::<Vec<_>>()).fetch_one(self.store.pool()).await.map_err(db)?;
            let (kind, value) = reference_parts(center);
            let degree=sqlx::query("SELECT count(*)::bigint degree,count(DISTINCT relation_kind)::bigint diversity,count(*) FILTER(WHERE polarity='negative')::bigint negative FROM association_evidence a WHERE a.subject_id=$1 AND a.revoked_at IS NULL AND ((a.from_ref_kind=$2 AND a.from_ref=$3) OR (a.to_ref_kind=$2 AND a.to_ref=$3) OR ($2='tag' AND ((a.from_ref_kind='tag' AND canonical_tag($1,CASE WHEN a.from_ref_kind='tag' THEN a.from_ref::uuid END)=$4) OR (a.to_ref_kind='tag' AND canonical_tag($1,CASE WHEN a.to_ref_kind='tag' THEN a.to_ref::uuid END)=$4))))")
                .bind(subject.0).bind(kind).bind(value).bind(if let CognitiveRef::Tag(id)=center {Some(id.0)}else {None}).fetch_one(self.store.pool()).await.map_err(db)?;
            let degree_count = usize::try_from(degree.try_get::<i64, _>("degree").map_err(db)?)
                .map_err(|_| Error::Infrastructure("invalid degree".into()))?;
            counterevidence += u64::try_from(degree.try_get::<i64, _>("negative").map_err(db)?)
                .map_err(|_| Error::Infrastructure("invalid negative count".into()))?;
            let descriptors = self
                .store
                .query_descriptors(subject, &refs.iter().take(32).cloned().collect::<Vec<_>>())
                .await?;
            let texts = descriptors.into_iter().map(|d| d.text).collect::<Vec<_>>();
            let coherence = if let Some(first) = texts.first() {
                let scores = self
                    .store
                    .cached_semantic_scores(subject, first, &texts)
                    .await?;
                let values = scores.into_iter().skip(1).flatten().collect::<Vec<_>>();
                if values.is_empty() {
                    None
                } else {
                    Some(values.iter().sum::<f64>() / values.len() as f64)
                }
            } else {
                None
            };
            let broad = degree_count.max(total) > policy.generic_degree
                || coherence.is_some_and(|c| c < policy.split_coherence);
            let evidence = (roots.len().min(4) as f64 / 4.0 + total.min(8) as f64 / 8.0) / 2.0;
            let recurrence = (usize::try_from(episodes).unwrap_or(0).min(3) as f64 / 3.0
                + (span.max(0) as f64 / 86400.0).min(1.0))
                / 2.0;
            let use_importance = (meaningful.min(16) as f64 / 16.0) * 0.15;
            let quality = coherence.map_or(0.5, |v| v.clamp(0.0, 1.0));
            let mut usefulness =
                ((evidence * 0.55 + recurrence * 0.15) * quality + use_importance).min(1.0);
            usefulness /= 1.0 + counterevidence as f64;
            if broad {
                usefulness *= 0.25;
            }
            let review_priority = if broad {
                50
            } else {
                20 + (usefulness * f64::from(policy.priority_bonus)).round() as u32
            };
            result.insert(
                center.clone(),
                AccretionSignal {
                    independent_roots: roots.len(),
                    attached_cognition: total,
                    cross_episode_recurrence: usize::try_from(episodes).unwrap_or(0),
                    observed_span_seconds: span,
                    association_degree: degree_count,
                    relation_diversity: usize::try_from(
                        degree.try_get::<i64, _>("diversity").map_err(db)?,
                    )
                    .unwrap_or(0),
                    meaningful_use: meaningful,
                    counterevidence,
                    semantic_coherence: coherence,
                    usefulness,
                    review_priority,
                    broad_center: broad,
                    partial,
                    member_keys: refs.iter().map(ToString::to_string).collect(),
                },
            );
        }
        Ok(result)
    }
    async fn accretion_sources(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<(BTreeSet<Uuid>, u64)> {
        let supports = match reference {
            CognitiveRef::MemoryRevision(id) => self.load_supports(*id).await?,
            CognitiveRef::CognitiveSchemaRevision(id) => self
                .schema_links(subject, *id)
                .await?
                .into_iter()
                .map(|link| link.support)
                .collect(),
            CognitiveRef::EpisodeRevision(id) => {
                self.episode_revision(subject, *id).await?.supports
            }
            CognitiveRef::JournalRevision(id) => self
                .journal_revision(subject, *id)
                .await?
                .points
                .into_iter()
                .flat_map(|point| point.supports)
                .collect(),
            _ => vec![],
        };
        let mut occurrences = BTreeSet::new();
        let mut contradictions = 0;
        for support in supports {
            match support {
                RevisionSupport::Evidence(e) => {
                    occurrences.insert(e.occurrence_id.0);
                    contradictions += u64::from(e.support_role == SupportRole::Contradiction);
                }
                RevisionSupport::CognitionDependency(d) => {
                    contradictions += u64::from(d.support_role == SupportRole::Contradiction)
                }
                _ => {}
            }
        }
        Ok((occurrences, contradictions))
    }
    async fn accretion_members(
        &self,
        subject: SubjectId,
        center: &CognitiveRef,
        limit: usize,
    ) -> Result<(Vec<CognitiveRef>, usize)> {
        let members = match center {
            CognitiveRef::Tag(_) => {
                "SELECT 'memory_revision'::text kind,m.memory_revision_id::text value FROM memory_revision_tags m JOIN memory_revisions r USING(memory_revision_id) WHERE r.subject_id=$1 AND canonical_tag($1,m.tag_id)=canonical_tag($1,$2) UNION SELECT a.from_ref_kind,a.from_ref FROM association_evidence a WHERE a.subject_id=$1 AND a.to_ref_kind='tag' AND canonical_tag($1,CASE WHEN a.to_ref_kind='tag' THEN a.to_ref::uuid END)=canonical_tag($1,$2) AND a.relation_kind='tag_attachment' AND a.polarity='positive' AND a.revoked_at IS NULL"
            }
            CognitiveRef::CognitiveSchema(_) | CognitiveRef::CognitiveSchemaRevision(_) => {
                "SELECT support_kind kind,support_ref value FROM cognitive_schema_evidence_links l JOIN cognitive_schema_revisions r USING(schema_revision_id) WHERE r.subject_id=$1 AND r.schema_revision_id=$2 AND l.revoked_at IS NULL AND support_kind IN ('memory_revision','cognitive_schema_revision','episode_revision','journal_revision')"
            }
            CognitiveRef::MemoryRevision(_) => return Ok((vec![center.clone()], 1)),
            _ => {
                return Err(Error::Invalid(
                    "accretion center must be Tag, Schema or exact Memory".into(),
                ));
            }
        };
        let live = "SELECT 'memory_revision'::text kind,current_revision_id::text value FROM memory_objects WHERE subject_id=$1 AND acceptance_state='accepted' AND integrity_state='valid' AND suppression_state='normal' AND purge_state='normal' UNION SELECT 'cognitive_schema_revision',current_revision_id::text FROM cognitive_schemas WHERE subject_id=$1 AND acceptance_state='accepted' AND integrity_state='valid' AND suppression_state='normal' AND purge_state='normal' UNION SELECT 'episode_revision',current_revision_id::text FROM episode_objects WHERE subject_id=$1 AND acceptance_state='accepted' AND integrity_state='valid' AND suppression_state='normal' AND purge_state='normal' UNION SELECT 'journal_revision',current_revision_id::text FROM journal_objects WHERE subject_id=$1 AND acceptance_state='accepted' AND integrity_state='valid' AND suppression_state='normal' AND purge_state='normal'";
        let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new("WITH live AS (");
        query.push(live).push("),members AS (").push(members).push("),eligible AS(SELECT DISTINCT kind,value FROM members JOIN live USING(kind,value)) SELECT *,count(*) OVER() total FROM eligible ORDER BY kind,value LIMIT $3");
        let id = match center {
            CognitiveRef::Tag(id) => id.0,
            CognitiveRef::CognitiveSchemaRevision(id) => id.0,
            CognitiveRef::CognitiveSchema(_) => {
                let (r, _, _) = self.store.bind_exact_reference(subject, center).await?;
                let CognitiveRef::CognitiveSchemaRevision(id) = r else {
                    unreachable!()
                };
                id.0
            }
            _ => unreachable!(),
        };
        let rows = query
            .build()
            .bind(subject.0)
            .bind(id)
            .bind(
                i64::try_from(limit)
                    .map_err(|_| Error::Invalid("member budget exceeded".into()))?,
            )
            .fetch_all(self.store.pool())
            .await
            .map_err(db)?;
        let total = rows
            .first()
            .map(|r| r.try_get::<i64, _>("total").map_err(db))
            .transpose()?
            .unwrap_or(0);
        let refs = rows
            .into_iter()
            .map(|r| {
                parse_reference(
                    &r.try_get::<String, _>("kind").map_err(db)?,
                    &r.try_get::<String, _>("value").map_err(db)?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Ok((
            refs,
            usize::try_from(total)
                .map_err(|_| Error::Infrastructure("invalid member count".into()))?,
        ))
    }
    pub async fn prioritize_topology_needs(&self, subject: SubjectId) -> Result<()> {
        let policy = self
            .configuration
            .snapshot_for_subject(subject)?
            .get(ACCRETION)?;
        if !policy.enabled {
            return Ok(());
        }
        let rows=sqlx::query("SELECT need_id,scope_kind,scope_ref FROM maintenance_needs WHERE subject_id=$1 AND kind='topology_maintenance' AND state='pending' ORDER BY due_at,need_id LIMIT $2")
            .bind(subject.0).bind(i64::try_from(policy.max_centers).map_err(|_|Error::Invalid("center bound exceeded".into()))?).fetch_all(self.store.pool()).await.map_err(db)?;
        let refs = rows
            .iter()
            .map(|r| {
                parse_reference(
                    &r.try_get::<String, _>("scope_kind").map_err(db)?,
                    &r.try_get::<String, _>("scope_ref").map_err(db)?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let centers = refs
            .iter()
            .filter(|reference| {
                matches!(
                    reference,
                    CognitiveRef::MemoryRevision(_)
                        | CognitiveRef::CognitiveSchemaRevision(_)
                        | CognitiveRef::CognitiveSchema(_)
                        | CognitiveRef::Tag(_)
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let signals = self.accretion_signals(subject, &centers).await?;
        for (row, reference) in rows.into_iter().zip(refs) {
            if let Some(signal) = signals.get(&reference) {
                sqlx::query("UPDATE maintenance_needs SET priority=GREATEST(priority,$3) WHERE subject_id=$1 AND need_id=$2 AND state='pending'").bind(subject.0).bind(row.try_get::<Uuid,_>("need_id").map_err(db)?).bind(i32::try_from(signal.review_priority).map_err(|_|Error::Invalid("priority exceeded".into()))?).execute(self.store.pool()).await.map_err(db)?;
            }
        }
        Ok(())
    }
}
