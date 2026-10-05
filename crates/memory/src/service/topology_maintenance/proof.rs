//! Relation-specific structural witnesses; observation time and use count are not event truth.
use super::*;
use sqlx::{Postgres, Transaction};
use std::collections::{HashMap, HashSet};
#[derive(Default, Clone)]
struct Facts {
    occurrences: HashSet<Uuid>,
    references: HashSet<CognitiveRef>,
    procedural: bool,
    outcome: bool,
}
impl Facts {
    fn add_member(&mut self, reference: CognitiveRef, pending: &mut Vec<CognitiveRef>) {
        match reference {
            CognitiveRef::Occurrence(id) => {
                self.occurrences.insert(id.0);
            }
            other if is_exact(&other) => pending.push(other),
            _ => {}
        }
    }
}
impl MemoryService {
    pub(super) async fn validate_relation_proof_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: SubjectId,
        from: &CognitiveRef,
        to: &CognitiveRef,
        relation: &TopologyRelation,
        exact: &HashSet<CognitiveRef>,
        limit: usize,
    ) -> Result<()> {
        if matches!(
            relation,
            TopologyRelation::Related | TopologyRelation::TagAttachment
        ) {
            return Ok(());
        }
        let left = self
            .relation_facts_in(tx, subject, from.clone(), limit)
            .await?;
        let right = self
            .relation_facts_in(tx, subject, to.clone(), limit)
            .await?;
        match relation {
            TopologyRelation::CoOccurs => {
                let common = left
                    .occurrences
                    .iter()
                    .any(|id| right.occurrences.contains(id));
                if !common
                    && !self
                        .episode_witness_in(tx, subject, from, to, &left, &right, exact, false)
                        .await?
                {
                    return Err(Error::Invalid(
                        "co-occurrence needs a shared occurrence or selected Episode witness"
                            .into(),
                    ));
                }
            }
            TopologyRelation::Sequence => {
                let ordered = self
                    .episode_witness_in(tx, subject, from, to, &left, &right, exact, true)
                    .await?;
                let event_order = match (
                    event_bounds_in(tx, subject, &left.occurrences).await?,
                    event_bounds_in(tx, subject, &right.occurrences).await?,
                ) {
                    (Some((start, end)), Some((next_start, _))) => {
                        start < next_start && end <= next_start
                    }
                    _ => false,
                };
                if !ordered && !event_order {
                    return Err(Error::Invalid(
                        "sequence needs selected Episode order or disjoint occurred-time witnesses"
                            .into(),
                    ));
                }
            }
            TopologyRelation::Procedural | TopologyRelation::SharedOutcome => {
                let mut witnessed = false;
                for reference in exact {
                    let facts = self
                        .relation_facts_in(tx, subject, reference.clone(), limit)
                        .await?;
                    let classified = if matches!(relation, TopologyRelation::Procedural) {
                        facts.procedural
                    } else {
                        facts.outcome
                    };
                    if classified && covers(&facts, &left) && covers(&facts, &right) {
                        witnessed = true;
                        break;
                    }
                }
                if !witnessed {
                    return Err(Error::Invalid("procedure/outcome needs a classified cognition witness covering both endpoints".into()));
                }
            }
            _ => {}
        }
        Ok(())
    }
    async fn relation_facts_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: SubjectId,
        reference: CognitiveRef,
        limit: usize,
    ) -> Result<Facts> {
        let mut facts = Facts::default();
        let root = reference.clone();
        let mut pending = vec![reference];
        while let Some(reference) = pending.pop() {
            if !facts.references.insert(reference.clone()) {
                continue;
            }
            if facts.references.len() > limit {
                return Err(Error::Invalid(
                    "relation proof input budget exceeded".into(),
                ));
            }
            let supports = match reference {
                CognitiveRef::MemoryRevision(id) => {
                    let row=sqlx::query("SELECT r.semantic_role,o.cognitive_role FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2")
                        .bind(subject.0).bind(id.0).fetch_one(&mut **tx).await.map_err(db)?;
                    facts.procedural |= reference == root
                        && row.try_get::<String, _>("cognitive_role").map_err(db)?
                            == "procedural_experience";
                    let role: String = row.try_get("semantic_role").map_err(db)?;
                    facts.outcome |= reference == root
                        && matches!(role.as_str(), "outcome" | "result" | "outcome_summary");
                    self.load_supports(id).await?
                }
                CognitiveRef::CognitiveSchemaRevision(id) => self
                    .schema_links(subject, id)
                    .await?
                    .into_iter()
                    .map(|link| link.support)
                    .collect(),
                CognitiveRef::EpisodeRevision(id) => {
                    let episode = self.episode_revision(subject, id).await?;
                    for member in episode.members {
                        facts.procedural |= reference == root && member.role == "procedure";
                        facts.outcome |= reference == root && member.role == "outcome";
                        facts.add_member(member.reference, &mut pending);
                    }
                    episode.supports
                }
                CognitiveRef::JournalRevision(id) => {
                    let journal = self.journal_revision(subject, id).await?;
                    journal
                        .points
                        .into_iter()
                        .flat_map(|point| point.supports)
                        .collect()
                }
                CognitiveRef::Tag(tag) => {
                    let rows=sqlx::query("SELECT DISTINCT a.from_ref_kind kind,a.from_ref value FROM association_evidence a WHERE a.subject_id=$1 AND a.to_ref_kind='tag' AND canonical_tag($1,CASE WHEN a.to_ref_kind='tag' THEN a.to_ref::uuid END)=canonical_tag($1,$2) AND a.relation_kind='tag_attachment' AND a.polarity='positive' AND a.revoked_at IS NULL UNION SELECT 'memory_revision',m.memory_revision_id::text FROM memory_revision_tags m JOIN memory_revisions r USING(memory_revision_id) WHERE r.subject_id=$1 AND canonical_tag($1,m.tag_id)=canonical_tag($1,$2) LIMIT $3")
                        .bind(subject.0).bind(tag.0).bind(i64::try_from(limit+1).map_err(|_|Error::Invalid("proof bound exceeded".into()))?).fetch_all(&mut **tx).await.map_err(db)?;
                    for row in rows {
                        pending.push(parse_reference(
                            &row.try_get::<String, _>("kind").map_err(db)?,
                            &row.try_get::<String, _>("value").map_err(db)?,
                        )?);
                    }
                    vec![]
                }
                _ => vec![],
            };
            for support in supports {
                match support {
                    RevisionSupport::Evidence(e) => {
                        facts.occurrences.insert(e.occurrence_id.0);
                    }
                    RevisionSupport::CognitionDependency(d) => pending.push(d.target_revision),
                    _ => {}
                }
            }
        }
        Ok(facts)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "selected Episode witness checks endpoint membership and direction"
    )]
    async fn episode_witness_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: SubjectId,
        from: &CognitiveRef,
        to: &CognitiveRef,
        left: &Facts,
        right: &Facts,
        exact: &HashSet<CognitiveRef>,
        ordered: bool,
    ) -> Result<bool> {
        for reference in exact {
            let CognitiveRef::EpisodeRevision(id) = reference else {
                continue;
            };
            let rows=sqlx::query("SELECT m.ref_kind,m.ref_value,m.ordinal FROM episode_revision_members m JOIN episode_revisions r USING(episode_revision_id) WHERE r.subject_id=$1 AND m.episode_revision_id=$2 ORDER BY m.ordinal LIMIT 257")
                .bind(subject.0).bind(id.0).fetch_all(&mut **tx).await.map_err(db)?;
            if rows.len() > 256 {
                return Err(Error::Invalid(
                    "Episode relation proof scope exceeded".into(),
                ));
            }
            let mut positions = HashMap::new();
            for row in rows {
                positions.insert(
                    parse_reference(
                        &row.try_get::<String, _>("ref_kind").map_err(db)?,
                        &row.try_get::<String, _>("ref_value").map_err(db)?,
                    )?,
                    row.try_get::<i32, _>("ordinal").map_err(db)?,
                );
            }
            let a = endpoint_positions(from, left, &positions);
            let b = endpoint_positions(to, right, &positions);
            if !a.is_empty() && !b.is_empty() && (!ordered || a.iter().max() < b.iter().min()) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
fn is_exact(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::MemoryRevision(_)
            | CognitiveRef::CognitiveSchemaRevision(_)
            | CognitiveRef::EpisodeRevision(_)
            | CognitiveRef::JournalRevision(_)
    )
}
fn covers(witness: &Facts, endpoint: &Facts) -> bool {
    witness
        .references
        .iter()
        .any(|r| endpoint.references.contains(r))
        || witness
            .occurrences
            .iter()
            .any(|id| endpoint.occurrences.contains(id))
}
fn endpoint_positions(
    reference: &CognitiveRef,
    facts: &Facts,
    positions: &HashMap<CognitiveRef, i32>,
) -> Vec<i32> {
    if let Some(position) = positions.get(reference) {
        return vec![*position];
    }
    positions
        .iter()
        .filter_map(|(reference, position)| match reference {
            CognitiveRef::Occurrence(id) if facts.occurrences.contains(&id.0) => Some(*position),
            other if facts.references.contains(other) => Some(*position),
            _ => None,
        })
        .collect()
}
async fn event_bounds_in(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    ids: &HashSet<Uuid>,
) -> Result<Option<(DateTime<Utc>, DateTime<Utc>)>> {
    if ids.is_empty() {
        return Ok(None);
    }
    let rows=sqlx::query("SELECT occurred_time_kind,occurred_time_start,occurred_time_end FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=ANY($2::uuid[])")
        .bind(subject.0).bind(ids.iter().copied().collect::<Vec<_>>()).fetch_all(&mut **tx).await.map_err(db)?;
    if rows.len() != ids.len() {
        return Ok(None);
    }
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for row in rows {
        let kind: String = row.try_get("occurred_time_kind").map_err(db)?;
        let (start, end): (Option<DateTime<Utc>>, Option<DateTime<Utc>>) = (
            row.try_get("occurred_time_start").map_err(db)?,
            row.try_get("occurred_time_end").map_err(db)?,
        );
        let (Some(start), Some(end)) = (start, if kind == "instant" { start } else { end }) else {
            return Ok(None);
        };
        if kind == "unknown" {
            return Ok(None);
        }
        starts.push(start);
        ends.push(end);
    }
    Ok(Some((
        *starts.iter().min().unwrap(),
        *ends.iter().max().unwrap(),
    )))
}
