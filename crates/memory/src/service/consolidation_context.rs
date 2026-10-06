use super::*;
use std::collections::BTreeSet;
impl MemoryService {
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
