use super::*;
use nous_persistence::database_error as db;
use std::collections::BTreeSet;

/// Authority-resolved provenance facts used by formation validation and
/// diagnostics.  The set is deliberately derived from immutable references;
/// it is not a mutable confidence or ranking field.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceSummary {
    pub roots: BTreeSet<EvidenceRoot>,
    pub normalized_inputs: BTreeSet<String>,
}

impl MemoryService {
    pub(crate) async fn validate_supports_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        supports: &[RevisionSupport],
    ) -> Result<()> {
        for support in supports {
            match support {
                RevisionSupport::Evidence(evidence) => {
                    sqlx::query("SELECT occurrence_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2 FOR SHARE")
                        .bind(subject.0).bind(evidence.occurrence_id.0)
                        .fetch_optional(&mut **tx).await.map_err(db)?
                        .ok_or_else(|| Error::Invalid("evidence occurrence is outside Subject".into()))?;
                }
                RevisionSupport::CognitionDependency(dependency) => {
                    let (query, id, _) = dependency_query(&dependency.target_revision, true)?;
                    let row = sqlx::query(query)
                        .bind(subject.0)
                        .bind(id)
                        .fetch_optional(&mut **tx)
                        .await
                        .map_err(db)?;
                    require_dependency_row(row)?;
                }
                RevisionSupport::Seed(_) => {
                    return Err(Error::Invalid(
                        "Memory revisions cannot use Cognitive Seed support".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn validate_object_dependency_cycle(
        &self,
        subject: SubjectId,
        owner_key: &str,
        supports: &[RevisionSupport],
    ) -> Result<()> {
        let mut stack = supports
            .iter()
            .filter_map(|support| match support {
                RevisionSupport::CognitionDependency(value) => Some(value.target_revision.clone()),
                RevisionSupport::Evidence(_) => None,
                RevisionSupport::Seed(_) => None,
            })
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        while let Some(reference) = stack.pop() {
            let object_key = self.revision_object_key(subject, &reference).await?;
            if object_key == owner_key {
                return Err(Error::Invalid(
                    "cognition dependency would create an object cycle".into(),
                ));
            }
            if !visited.insert(reference.to_string()) {
                continue;
            }
            for nested in self.revision_supports(subject, reference).await? {
                if let RevisionSupport::CognitionDependency(value) = nested {
                    stack.push(value.target_revision);
                }
            }
        }
        Ok(())
    }

    pub async fn provenance_summary(
        &self,
        subject: SubjectId,
        supports: &[RevisionSupport],
    ) -> Result<ProvenanceSummary> {
        let mut summary = ProvenanceSummary::default();
        let mut stack = Vec::new();
        for support in supports {
            match support {
                RevisionSupport::Evidence(evidence) => {
                    for root in self.evidence_roots(subject, evidence).await? {
                        summary
                            .normalized_inputs
                            .insert(format!("source:{}", root.root_key));
                        summary.roots.insert(root);
                    }
                }
                RevisionSupport::CognitionDependency(dependency) => {
                    let key = dependency.target_revision.to_string();
                    summary.normalized_inputs.insert(format!("revision:{key}"));
                    stack.push((dependency.target_revision.clone(), false));
                }
                RevisionSupport::Seed(_) => {
                    return Err(Error::Invalid(
                        "Memory revisions cannot use Cognitive Seed support".into(),
                    ));
                }
            }
        }

        let mut active = BTreeSet::new();
        let mut visited = BTreeSet::new();
        while let Some((reference, exit)) = stack.pop() {
            let key = reference.to_string();
            if exit {
                active.remove(&key);
                visited.insert(key);
                continue;
            }
            if active.contains(&key) {
                return Err(Error::FailedPrecondition(
                    "cognition dependency cycle detected".into(),
                ));
            }
            if visited.contains(&key) {
                continue;
            }
            self.validate_dependency_target(subject, &reference).await?;
            active.insert(key.clone());
            stack.push((reference.clone(), true));
            let nested = self.revision_supports(subject, reference).await?;
            for support in nested.into_iter().rev() {
                match support {
                    RevisionSupport::Evidence(evidence) => {
                        summary
                            .roots
                            .extend(self.evidence_roots(subject, &evidence).await?);
                    }
                    RevisionSupport::CognitionDependency(dependency) => {
                        stack.push((dependency.target_revision, false));
                    }
                    RevisionSupport::Seed(_) => {
                        return Err(Error::Invalid(
                            "Memory revisions cannot use Cognitive Seed support".into(),
                        ));
                    }
                }
            }
        }
        Ok(summary)
    }

    pub(crate) async fn validate_formation_semantics(
        &self,
        subject: SubjectId,
        mode: FormationMode,
        supports: &[RevisionSupport],
    ) -> Result<()> {
        if !matches!(mode, FormationMode::Synthesized) {
            return Ok(());
        }
        let summary = self.provenance_summary(subject, supports).await?;
        if summary.normalized_inputs.len() < 2 {
            return Err(Error::Invalid(
                "synthesized formation needs at least two normalized inputs".into(),
            ));
        }
        let independent_roots = summary
            .roots
            .iter()
            .filter(|root| matches!(root.certainty, EvidenceRootCertainty::Known))
            .count();
        if independent_roots < 2 {
            return Err(Error::Invalid(
                "synthesized formation needs two known independent provenance roots".into(),
            ));
        }
        Ok(())
    }

    async fn occurrence_root(
        &self,
        subject: SubjectId,
        occurrence: OccurrenceId,
    ) -> Result<EvidenceRoot> {
        let row = sqlx::query(
            "SELECT source_class,external_object_ref,artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2",
        )
        .bind(subject.0)
        .bind(occurrence.0)
        .fetch_optional(self.store.pool())
        .await
        .map_err(db)?
        .ok_or_else(|| Error::Invalid("evidence occurrence is outside Subject".into()))?;
        let source_class: String = row.try_get("source_class").map_err(db)?;
        let external: Option<String> = row.try_get("external_object_ref").map_err(db)?;
        let artifact: Option<Uuid> = row.try_get("artifact_id").map_err(db)?;
        if let Some(artifact) = artifact {
            return self.artifact_root(subject, artifact).await;
        }
        if let Some(external) = external {
            return Ok(EvidenceRoot {
                root_key: format!("external:{source_class}:{external}"),
                certainty: EvidenceRootCertainty::Known,
            });
        }
        Ok(EvidenceRoot {
            root_key: format!("occurrence:{}", occurrence.0),
            certainty: EvidenceRootCertainty::OccurrenceOnly,
        })
    }

    async fn evidence_roots(
        &self,
        subject: SubjectId,
        evidence: &EvidenceRef,
    ) -> Result<BTreeSet<EvidenceRoot>> {
        let representation = match evidence.locator {
            EvidenceLocator::DerivedRepresentation(id) => Some(id.0),
            EvidenceLocator::DerivedRegion(id) => Some(sqlx::query_scalar::<_,Uuid>("SELECT derived_representation_id FROM derived_regions WHERE subject_id=$1 AND derived_region_id=$2").bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(|| Error::Invalid("derived evidence region is outside Subject".into()))?),
            _ => None,
        };
        let Some(representation) = representation else {
            return Ok(BTreeSet::from([self
                .occurrence_root(subject, evidence.occurrence_id)
                .await?]));
        };
        let artifacts = sqlx::query_scalar::<_,Uuid>("SELECT DISTINCT sr.artifact_id FROM representation_source_regions($1,$2) roots JOIN source_regions sr USING(source_region_id) WHERE sr.subject_id=$1").bind(subject.0).bind(representation).fetch_all(self.store.pool()).await.map_err(db)?;
        if artifacts.is_empty() {
            return Err(Error::Invalid(
                "derived evidence has no valid source roots".into(),
            ));
        }
        let mut roots = BTreeSet::new();
        for artifact in artifacts {
            roots.insert(self.artifact_root(subject, artifact).await?);
        }
        Ok(roots)
    }

    async fn artifact_root(&self, subject: SubjectId, artifact: Uuid) -> Result<EvidenceRoot> {
        // Same Artifact and explicit external-source identity are dependency aliases.
        // Their connected lineage prevents copied content or versions of one source
        // from becoming independent corroboration through a different locator.
        let external = sqlx::query_scalar::<_,Option<String>>("WITH RECURSIVE lineage AS (SELECT occurrence_id,artifact_id,source_class,external_object_ref FROM observation_occurrences WHERE subject_id=$1 AND artifact_id=$2 UNION SELECT o.occurrence_id,o.artifact_id,o.source_class,o.external_object_ref FROM observation_occurrences o JOIN lineage p ON (o.artifact_id IS NOT NULL AND o.artifact_id=p.artifact_id) OR (o.external_object_ref IS NOT NULL AND o.external_object_ref=p.external_object_ref AND o.source_class=p.source_class) WHERE o.subject_id=$1) SELECT MIN('external:' || source_class || ':' || external_object_ref) FROM lineage WHERE external_object_ref IS NOT NULL").bind(subject.0).bind(artifact).fetch_one(self.store.pool()).await.map_err(db)?;
        Ok(EvidenceRoot {
            root_key: external.unwrap_or_else(|| format!("artifact:{artifact}")),
            certainty: EvidenceRootCertainty::Known,
        })
    }

    async fn validate_dependency_target(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<()> {
        let (query, id, _) = dependency_query(reference, false)?;
        let row = sqlx::query(query)
            .bind(subject.0)
            .bind(id)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?;
        require_dependency_row(row)
    }

    async fn revision_object_key(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<String> {
        let (query, id, kind) = dependency_query(reference, false)?;
        let row = sqlx::query(query)
            .bind(subject.0)
            .bind(id)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?
            .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into()))?;
        let object: Uuid = row.try_get("object_id").map_err(db)?;
        Ok(format!("{kind}:{object}"))
    }

    async fn revision_supports(
        &self,
        subject: SubjectId,
        reference: CognitiveRef,
    ) -> Result<Vec<RevisionSupport>> {
        match reference {
            CognitiveRef::MemoryRevision(revision) => self.load_supports(revision).await,
            CognitiveRef::CognitiveSchemaRevision(revision) => Ok(self
                .schema_links(subject, revision)
                .await?
                .into_iter()
                .map(|link| link.support)
                .collect()),
            CognitiveRef::EpisodeRevision(revision) => {
                Ok(self.episode_revision(subject, revision).await?.supports)
            }
            CognitiveRef::JournalRevision(revision) => Ok(self
                .journal_revision(subject, revision)
                .await?
                .points
                .into_iter()
                .flat_map(|point| point.supports)
                .collect()),
            _ => Err(Error::Invalid(
                "cognition dependency needs exact revision".into(),
            )),
        }
    }
}

fn dependency_query(
    reference: &CognitiveRef,
    lock: bool,
) -> Result<(&'static str, Uuid, &'static str)> {
    match reference {
        CognitiveRef::MemoryRevision(id) => Ok((
            if lock {
                "SELECT o.purge_state,r.memory_id AS object_id FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2 FOR SHARE OF o"
            } else {
                "SELECT o.purge_state,r.memory_id AS object_id FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2"
            },
            id.0,
            "memory",
        )),
        CognitiveRef::CognitiveSchemaRevision(id) => Ok((
            if lock {
                "SELECT o.purge_state,r.schema_id AS object_id FROM cognitive_schema_revisions r JOIN cognitive_schemas o USING(schema_id) WHERE o.subject_id=$1 AND r.schema_revision_id=$2 FOR SHARE OF o"
            } else {
                "SELECT o.purge_state,r.schema_id AS object_id FROM cognitive_schema_revisions r JOIN cognitive_schemas o USING(schema_id) WHERE o.subject_id=$1 AND r.schema_revision_id=$2"
            },
            id.0,
            "schema",
        )),
        CognitiveRef::EpisodeRevision(id) => Ok((
            if lock {
                "SELECT o.purge_state,r.episode_id AS object_id FROM episode_revisions r JOIN episode_objects o USING(episode_id) WHERE r.subject_id=$1 AND r.episode_revision_id=$2 FOR SHARE OF o"
            } else {
                "SELECT o.purge_state,r.episode_id AS object_id FROM episode_revisions r JOIN episode_objects o USING(episode_id) WHERE r.subject_id=$1 AND r.episode_revision_id=$2"
            },
            id.0,
            "episode",
        )),
        CognitiveRef::JournalRevision(id) => Ok((
            if lock {
                "SELECT o.purge_state,r.journal_id AS object_id FROM journal_revisions r JOIN journal_objects o USING(journal_id) WHERE r.subject_id=$1 AND r.journal_revision_id=$2 FOR SHARE OF o"
            } else {
                "SELECT o.purge_state,r.journal_id AS object_id FROM journal_revisions r JOIN journal_objects o USING(journal_id) WHERE r.subject_id=$1 AND r.journal_revision_id=$2"
            },
            id.0,
            "journal",
        )),
        _ => Err(Error::Invalid(
            "cognition dependency needs exact revision".into(),
        )),
    }
}

fn require_dependency_row(row: Option<sqlx::postgres::PgRow>) -> Result<()> {
    let state = row
        .map(|row| row.try_get::<String, _>("purge_state").map_err(db))
        .transpose()?;
    match state.as_deref() {
        None => Err(Error::Invalid(
            "cognition dependency is outside Subject".into(),
        )),
        Some("purging") => Err(Error::FailedPrecondition(
            "cognition dependency target is purging".into(),
        )),
        Some(_) => Ok(()),
    }
}
