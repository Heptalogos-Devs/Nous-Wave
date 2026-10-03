use super::*;
use std::collections::BTreeSet;
impl MemoryService {
    pub(super) async fn validate_consolidation_scope_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        input: &LongitudinalConsolidationInput,
    ) -> Result<()> {
        if !matches!(
            input.source.reference,
            CognitiveRef::EpisodeRevision(_) | CognitiveRef::JournalRevision(_)
        ) {
            return Err(Error::Invalid(
                "Consolidation source must be an exact Episode or Journal revision".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        for expected in std::iter::once(&input.source).chain(&input.context) {
            if !seen.insert(expected.reference.to_string()) {
                return Err(Error::Invalid(
                    "duplicate consolidation context reference".into(),
                ));
            }
            validate_current_cognition_in(tx, input.subject, expected).await?;
        }
        if input.context.iter().any(|context| {
            !matches!(
                context.reference,
                CognitiveRef::MemoryRevision(_) | CognitiveRef::CognitiveSchemaRevision(_)
            )
        }) {
            return Err(Error::Invalid(
                "Consolidation context requires Memory or Schema revisions".into(),
            ));
        }
        Ok(())
    }
    pub(crate) async fn consolidation_support_catalog(
        &self,
        input: &LongitudinalConsolidationInput,
    ) -> Result<BTreeSet<String>> {
        let scopes: Vec<_> = std::iter::once(&input.source)
            .chain(&input.context)
            .map(|value| value.reference.clone())
            .collect();
        Ok(self
            .consolidation_catalog_supports(input.subject, &scopes)
            .await?
            .into_iter()
            .map(|support| support.canonical_key())
            .collect())
    }
    pub async fn consolidation_catalog_supports(
        &self,
        subject: SubjectId,
        scopes: &[CognitiveRef],
    ) -> Result<Vec<RevisionSupport>> {
        let mut allowed = std::collections::BTreeMap::new();
        let mut visited = BTreeSet::new();
        let mut pending = scopes.to_vec();
        while let Some(reference) = pending.pop() {
            if !visited.insert(reference.to_string()) {
                continue;
            }
            let direct = RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: reference.clone(),
                support_role: SupportRole::Direct,
            });
            allowed.insert(direct.canonical_key(), direct);
            let supports = match reference {
                CognitiveRef::EpisodeRevision(id) => {
                    let episode = self.episode_revision(subject, id).await?;
                    allowed.extend(
                        episode
                            .members
                            .iter()
                            .filter_map(member_support)
                            .map(|support| (support.canonical_key(), support)),
                    );
                    episode.supports
                }
                CognitiveRef::JournalRevision(id) => self
                    .journal_revision(subject, id)
                    .await?
                    .points
                    .into_iter()
                    .flat_map(|point| point.supports)
                    .collect(),
                CognitiveRef::MemoryRevision(id) => self.load_supports(id).await?,
                CognitiveRef::CognitiveSchemaRevision(id) => self
                    .schema_links(subject, id)
                    .await?
                    .into_iter()
                    .map(|link| link.support)
                    .collect(),
                _ => {
                    return Err(Error::Invalid(
                        "Consolidation dependency is not an exact cognition revision".into(),
                    ));
                }
            };
            for support in supports {
                allowed.insert(support.canonical_key(), support.clone());
                if let RevisionSupport::CognitionDependency(dependency) = support {
                    pending.push(dependency.target_revision);
                }
            }
        }
        Ok(allowed.into_values().collect())
    }
}
pub(super) async fn validate_current_cognition_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    expected: &ExpectedCognition,
) -> Result<()> {
    let (query, id) = match expected.reference {
        CognitiveRef::MemoryRevision(id) => (
            "SELECT o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2 FOR SHARE OF o",
            id.0,
        ),
        CognitiveRef::CognitiveSchemaRevision(id) => (
            "SELECT o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state FROM cognitive_schema_revisions r JOIN cognitive_schemas o USING(schema_id) WHERE o.subject_id=$1 AND r.schema_revision_id=$2 FOR SHARE OF o",
            id.0,
        ),
        CognitiveRef::EpisodeRevision(id) => (
            "SELECT o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state FROM episode_revisions r JOIN episode_objects o USING(episode_id) WHERE r.subject_id=$1 AND r.episode_revision_id=$2 FOR SHARE OF o",
            id.0,
        ),
        CognitiveRef::JournalRevision(id) => (
            "SELECT o.current_revision_id,o.object_epoch,o.acceptance_state,o.integrity_state,o.suppression_state,o.purge_state FROM journal_revisions r JOIN journal_objects o USING(journal_id) WHERE r.subject_id=$1 AND r.journal_revision_id=$2 FOR SHARE OF o",
            id.0,
        ),
        _ => {
            return Err(Error::Invalid(
                "Consolidation requires exact revision identities".into(),
            ));
        }
    };
    let row = sqlx::query(query)
        .bind(subject.0)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::Conflict("Consolidation source or target disappeared".into()))?;
    if row.get::<Uuid, _>("current_revision_id") != id
        || row.get::<i64, _>("object_epoch") != expected.expected_epoch
        || row.get::<String, _>("acceptance_state") != "accepted"
        || row.get::<String, _>("integrity_state") != "valid"
        || row.get::<String, _>("suppression_state") != "normal"
        || row.get::<String, _>("purge_state") != "normal"
    {
        return Err(Error::Conflict(
            "Consolidation source or target is stale".into(),
        ));
    }
    Ok(())
}

fn member_support(member: &EpisodeMember) -> Option<RevisionSupport> {
    match member.reference {
        CognitiveRef::Occurrence(id) => Some(RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        })),
        _ => None,
    }
}
