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
    #[expect(
        clippy::excessive_nesting,
        reason = "transactional target checks keep exact revision kind handling together"
    )]
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
                        .bind(subject.0)
                        .bind(evidence.occurrence_id.0)
                        .fetch_optional(&mut **tx)
                        .await
                        .map_err(db)?
                        .ok_or_else(|| Error::Invalid("evidence occurrence is outside Subject".into()))?;
                }
                RevisionSupport::CognitionDependency(dependency) => {
                    match dependency.target_revision {
                        CognitiveRef::MemoryRevision(revision) => {
                            let row = sqlx::query("SELECT o.purge_state FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2 FOR SHARE")
                                .bind(subject.0).bind(revision.0).fetch_optional(&mut **tx).await.map_err(db)?
                                .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into()))?;
                            if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
                                return Err(Error::FailedPrecondition(
                                    "cognition dependency target is purging".into(),
                                ));
                            }
                        }
                        CognitiveRef::CognitiveSchemaRevision(revision) => {
                            let row = sqlx::query("SELECT s.purge_state FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2 FOR SHARE")
                                .bind(subject.0).bind(revision.0).fetch_optional(&mut **tx).await.map_err(db)?
                                .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into()))?;
                            if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
                                return Err(Error::FailedPrecondition(
                                    "cognition dependency target is purging".into(),
                                ));
                            }
                        }
                        _ => {
                            return Err(Error::Invalid(
                                "cognition dependency needs exact revision".into(),
                            ));
                        }
                    }
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
                return Err(Error::FailedPrecondition(
                    "cognition dependency would create an object cycle".into(),
                ));
            }
            if !visited.insert(reference.to_string()) {
                continue;
            }
            for nested in self.revision_supports(reference).await? {
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
                    let root = self
                        .occurrence_root(subject, evidence.occurrence_id)
                        .await?;
                    summary.roots.insert(root.clone());
                    summary
                        .normalized_inputs
                        .insert(format!("source:{}", root.root_key));
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
            let nested = self.revision_supports(reference).await?;
            for support in nested.into_iter().rev() {
                match support {
                    RevisionSupport::Evidence(evidence) => {
                        let root = self
                            .occurrence_root(subject, evidence.occurrence_id)
                            .await?;
                        summary.roots.insert(root);
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
        if let Some(external) = external {
            return Ok(EvidenceRoot {
                root_key: format!("external:{source_class}:{external}"),
                certainty: EvidenceRootCertainty::Known,
            });
        }
        if let Some(artifact) = artifact {
            return Ok(EvidenceRoot {
                root_key: format!("artifact:{artifact}"),
                certainty: EvidenceRootCertainty::Known,
            });
        }
        Ok(EvidenceRoot {
            root_key: format!("occurrence:{}", occurrence.0),
            certainty: EvidenceRootCertainty::OccurrenceOnly,
        })
    }

    async fn validate_dependency_target(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<()> {
        match reference {
            CognitiveRef::MemoryRevision(revision) => {
                let row = sqlx::query(
                    "SELECT o.purge_state FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2",
                )
                .bind(subject.0)
                .bind(revision.0)
                .fetch_optional(self.store.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into()))?;
                if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
                    return Err(Error::FailedPrecondition(
                        "cognition dependency target is purging".into(),
                    ));
                }
            }
            CognitiveRef::CognitiveSchemaRevision(revision) => {
                let row = sqlx::query(
                    "SELECT s.purge_state FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2",
                )
                .bind(subject.0)
                .bind(revision.0)
                .fetch_optional(self.store.pool())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into()))?;
                if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
                    return Err(Error::FailedPrecondition(
                        "cognition dependency target is purging".into(),
                    ));
                }
            }
            _ => {
                return Err(Error::Invalid(
                    "cognition dependency needs exact revision".into(),
                ));
            }
        }
        Ok(())
    }

    async fn revision_object_key(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<String> {
        match reference {
            CognitiveRef::MemoryRevision(revision) => sqlx::query_scalar::<_, Uuid>(
                "SELECT r.memory_id FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2",
            )
            .bind(subject.0)
            .bind(revision.0)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?
            .map(|id| format!("memory:{id}"))
            .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into())),
            CognitiveRef::CognitiveSchemaRevision(revision) => sqlx::query_scalar::<_, Uuid>(
                "SELECT r.schema_id FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2",
            )
            .bind(subject.0)
            .bind(revision.0)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?
            .map(|id| format!("schema:{id}"))
            .ok_or_else(|| Error::Invalid("cognition dependency is outside Subject".into())),
            _ => Err(Error::Invalid("cognition dependency needs exact revision".into())),
        }
    }

    async fn revision_supports(&self, reference: CognitiveRef) -> Result<Vec<RevisionSupport>> {
        match reference {
            CognitiveRef::MemoryRevision(revision) => self.load_supports(revision).await,
            CognitiveRef::CognitiveSchemaRevision(revision) => Ok(self
                .schema_links(self.subject_for_schema_revision(revision).await?, revision)
                .await?
                .into_iter()
                .map(|link| link.support)
                .collect()),
            _ => Err(Error::Invalid(
                "cognition dependency needs exact revision".into(),
            )),
        }
    }

    async fn subject_for_schema_revision(
        &self,
        revision: CognitiveSchemaRevisionId,
    ) -> Result<SubjectId> {
        sqlx::query_scalar(
            "SELECT s.subject_id FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE r.schema_revision_id=$1",
        )
        .bind(revision.0)
        .fetch_optional(self.store.pool())
        .await
        .map_err(db)?
        .map(SubjectId)
        .ok_or_else(|| Error::Invalid("schema revision does not exist".into()))
    }
}
