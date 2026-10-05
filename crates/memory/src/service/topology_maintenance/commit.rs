use super::*;
use sqlx::{Postgres, Transaction};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyOutcome {
    pub status: String,
    pub changes: usize,
    pub results: BTreeMap<String, CognitiveRef>,
}
#[derive(Debug, Clone)]
pub struct CommitTopologyInput {
    pub operation_id: OperationId,
    pub claimed: nous_runtime::MaintenanceNeed,
    pub plan: TopologyPlan,
    pub proposal: TopologyProposal,
    pub producer: ProducerSignature,
}
impl MemoryService {
    #[expect(
        clippy::too_many_lines,
        reason = "one atomic proposal owns lease, catalog fence and receipt"
    )]
    pub async fn commit_topology(&self, input: CommitTopologyInput) -> Result<TopologyOutcome> {
        let subject = input.plan.subject;
        let digest = operation_digest(
            "topology_maintenance",
            subject,
            &serde_json::json!({"plan":input.plan,"proposal":input.proposal,"producer":AuthorityStore::canonical_producer(&input.producer)?}),
        )?;
        let mut mutation =
            match self
                .start_mutation(subject, input.operation_id, "topology_maintenance", &digest)
                .await?
            {
                MutationStart::Replay(receipt) => {
                    return serde_json::from_str(&receipt.result_ref.ok_or_else(|| {
                        Error::Infrastructure("missing topology receipt".into())
                    })?)
                    .map_err(|e| Error::Infrastructure(e.to_string()));
                }
                MutationStart::Active(mutation) => mutation,
            };
        if input.claimed.subject_id != subject
            || input.claimed.kind != "topology_maintenance"
            || input.claimed.scope_kind != reference_parts(&input.plan.focus).0
            || input.claimed.scope_ref != reference_parts(&input.plan.focus).1
        {
            return Err(Error::Invalid(
                "topology maintenance claim does not match focus".into(),
            ));
        }
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM maintenance_needs WHERE subject_id=$1 AND need_id=$2 AND state='leased' AND lease_token=$3 AND lease_until>clock_timestamp() AND trigger_revision=$4 AND kind='topology_maintenance' AND scope_kind=$5 AND scope_ref=$6)")
            .bind(subject.0).bind(input.claimed.need_id).bind(input.claimed.lease_token).bind(i64::try_from(input.claimed.trigger_revision).map_err(|_|Error::Invalid("invalid trigger revision".into()))?).bind(&input.claimed.scope_kind).bind(&input.claimed.scope_ref).fetch_one(&mut **mutation.tx()).await.map_err(db)?;
        if !valid {
            return Err(Error::Conflict("topology lease expired or replaced".into()));
        }
        let sequence: i64 =
            sqlx::query_scalar("SELECT authority_seq FROM subjects WHERE subject_id=$1 FOR UPDATE")
                .bind(subject.0)
                .fetch_one(&mut **mutation.tx())
                .await
                .map_err(db)?;
        if sequence != input.plan.authority_seq {
            return Err(Error::Conflict("topology catalog is stale".into()));
        }
        let current = self
            .plan_topology(subject, input.plan.focus.clone())
            .await?;
        if current.catalog_identity() != input.plan.catalog_identity() {
            return Err(Error::Conflict("topology catalog or policy changed".into()));
        }
        let mut expanded = 0;
        let mut actions = HashSet::new();
        for action in &input.proposal.actions {
            expanded += match action {
                TopologyAction::SplitTag { children, .. } => children.len(),
                _ => 1,
            };
            let mut value =
                serde_json::to_value(action).map_err(|e| Error::Infrastructure(e.to_string()))?;
            if let Some(object) = value.as_object_mut() {
                object.remove("reason");
            }
            if !actions.insert(value.to_string()) {
                return Err(Error::Invalid("duplicate topology action".into()));
            }
        }
        if expanded == 0 || expanded > current.policy.max_actions {
            return Err(Error::Invalid("topology action budget exceeded".into()));
        }
        if input.proposal.actions.len() > 1
            && input
                .proposal
                .actions
                .iter()
                .any(|a| matches!(a, TopologyAction::NoChange))
        {
            return Err(Error::Invalid("no_change cannot mix with mutations".into()));
        }
        let producer = AuthorityStore::register_producer_in(mutation.tx(), &input.producer).await?;
        let mut refs: HashMap<String, CognitiveRef> = current
            .cognition
            .iter()
            .map(|c| (c.key.clone(), c.reference.clone()))
            .chain(
                current
                    .tags
                    .iter()
                    .map(|t| (t.key.clone(), CognitiveRef::Tag(t.target.tag_id))),
            )
            .chain(
                current
                    .entities
                    .iter()
                    .map(|(key, (entity, _))| (key.clone(), CognitiveRef::Entity(entity.clone()))),
            )
            .collect();
        let mut results = BTreeMap::new();
        let mut changes = 0;
        for action in &input.proposal.actions {
            changes += self
                .apply_topology_action_in(
                    mutation.tx(),
                    &input,
                    &current,
                    action,
                    producer,
                    &mut refs,
                    &mut results,
                )
                .await?;
        }
        if changes > 0 {
            mutation
                .invalidate(ProjectionInvalidation::topology())
                .await?;
        }
        let outcome = TopologyOutcome {
            status: if changes == 0 {
                "no_change"
            } else {
                "committed"
            }
            .into(),
            changes,
            results,
        };
        let result =
            serde_json::to_string(&outcome).map_err(|e| Error::Infrastructure(e.to_string()))?;
        mutation
            .commit("topology_maintenance", Some(&result), None, None)
            .await?;
        Ok(outcome)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "typed proposal interpreter shares one owner transaction and result namespace"
    )]
    async fn apply_topology_action_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &CommitTopologyInput,
        plan: &TopologyPlan,
        action: &TopologyAction,
        producer: Uuid,
        refs: &mut HashMap<String, CognitiveRef>,
        results: &mut BTreeMap<String, CognitiveRef>,
    ) -> Result<usize> {
        let subject = plan.subject;
        match action {
            TopologyAction::NoChange => Ok(0),
            TopologyAction::ReuseTag { tag_key } => {
                tag_target(plan, tag_key)?;
                Ok(0)
            }
            TopologyAction::CreateTag {
                key,
                content,
                support_keys,
                reason,
            } => {
                new_key(refs, key)?;
                reason_valid(reason)?;
                let supports = selected_revisions(plan, support_keys)?;
                require_cognition_support(&supports)?;
                self.validate_supports_in_tx(tx, subject, &supports).await?;
                let tag = self
                    .create_tag_in(
                        tx,
                        subject,
                        content,
                        "topology_maintenance",
                        Some(&input.producer),
                    )
                    .await?;
                let reference = CognitiveRef::Tag(tag.tag_id);
                refs.insert(key.clone(), reference.clone());
                results.insert(key.clone(), reference);
                Ok(1)
            }
            TopologyAction::ReviseTag {
                tag_key,
                content,
                support_keys,
                reason,
            } => {
                reason_valid(reason)?;
                let supports = selected_revisions(plan, support_keys)?;
                require_cognition_support(&supports)?;
                self.validate_supports_in_tx(tx, subject, &supports).await?;
                self.revise_tag_in(
                    tx,
                    subject,
                    &tag_target(plan, tag_key)?,
                    content,
                    Some(&input.producer),
                )
                .await?;
                Ok(1)
            }
            TopologyAction::AttachTag {
                cognition_key,
                tag_key,
                support_keys,
                reason,
            } => {
                reason_valid(reason)?;
                let from = reference(refs, cognition_key)?;
                let to = reference(refs, tag_key)?;
                if !matches!(to, CognitiveRef::Tag(_)) || !is_cognition(&from) {
                    return Err(Error::Invalid(
                        "attachment requires exact cognition and Tag".into(),
                    ));
                }
                self.topology_association_in(
                    tx,
                    subject,
                    from,
                    to,
                    &TopologyRelation::TagAttachment,
                    plan,
                    support_keys,
                    input.operation_id,
                    producer,
                )
                .await?;
                Ok(1)
            }
            TopologyAction::CreateAssociation {
                from_key,
                to_key,
                relation,
                support_keys,
                reason,
            } => {
                reason_valid(reason)?;
                self.topology_association_in(
                    tx,
                    subject,
                    reference(refs, from_key)?,
                    reference(refs, to_key)?,
                    relation,
                    plan,
                    support_keys,
                    input.operation_id,
                    producer,
                )
                .await?;
                Ok(1)
            }
            TopologyAction::DetachTag {
                association_key,
                support_keys,
                reason,
            }
            | TopologyAction::RevokeAssociation {
                association_key,
                support_keys,
                reason,
            } => {
                reason_valid(reason)?;
                let supports = selected_revisions(plan, support_keys)?;
                require_cognition_support(&supports)?;
                self.validate_supports_in_tx(tx, subject, &supports).await?;
                let association = plan
                    .associations
                    .iter()
                    .find(|a| &a.key == association_key)
                    .ok_or_else(|| Error::Invalid("association key outside catalog".into()))?;
                if matches!(action, TopologyAction::DetachTag { .. })
                    && association.relation != "tag_attachment"
                {
                    return Err(Error::Invalid("detach_tag requires attachment".into()));
                }
                let changed=sqlx::query("UPDATE association_evidence SET revoked_at=$3 WHERE subject_id=$1 AND association_evidence_id=$2 AND revoked_at IS NULL").bind(subject.0).bind(association.id.0).bind(self.cognition.now(subject)).execute(&mut **tx).await.map_err(db)?.rows_affected();
                if changed != 1 {
                    return Err(Error::Conflict(
                        "association changed during maintenance".into(),
                    ));
                }
                Ok(1)
            }
            TopologyAction::MergeTags {
                survivor_key,
                retired_keys,
                support_keys,
                reason,
            } => {
                reason_valid(reason)?;
                self.merge_tags_in(
                    tx,
                    subject,
                    &MergeTagsInput {
                        operation_id: input.operation_id,
                        survivor: tag_target(plan, survivor_key)?,
                        retired: retired_keys
                            .iter()
                            .map(|key| tag_target(plan, key))
                            .collect::<Result<Vec<_>>>()?,
                        supports: selected_revisions(plan, support_keys)?,
                    },
                )
                .await?;
                Ok(1)
            }
            TopologyAction::SplitTag {
                tag_key,
                children,
                support_keys,
                reason,
            } => {
                reason_valid(reason)?;
                let mut keys = HashSet::new();
                for child in children {
                    new_key(refs, &child.key)?;
                    if !keys.insert(&child.key) {
                        return Err(Error::Invalid("duplicate split result key".into()));
                    }
                }
                let tags = self
                    .split_tag_in(
                        tx,
                        subject,
                        &SplitTagInput {
                            operation_id: input.operation_id,
                            parent: tag_target(plan, tag_key)?,
                            children: children.iter().map(|c| c.content.clone()).collect(),
                            supports: selected_revisions(plan, support_keys)?,
                        },
                        Some(&input.producer),
                    )
                    .await?;
                for (child, tag) in children.iter().zip(tags) {
                    let reference = CognitiveRef::Tag(tag.tag_id);
                    refs.insert(child.key.clone(), reference.clone());
                    results.insert(child.key.clone(), reference);
                }
                Ok(children.len())
            }
        }
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "association contract binds endpoints, relation and provenance within the proposal transaction"
    )]
    async fn topology_association_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        subject: SubjectId,
        from: CognitiveRef,
        to: CognitiveRef,
        relation: &TopologyRelation,
        plan: &TopologyPlan,
        keys: &[String],
        operation: OperationId,
        producer: Uuid,
    ) -> Result<()> {
        if from == to {
            return Err(Error::Invalid("topology self-loop rejected".into()));
        }
        let supports = selected_supports(plan, keys)?;
        let revisions = supports
            .iter()
            .filter_map(|s| {
                if let AssociationSupport::Revision(r) = s {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        require_cognition_support(&revisions)?;
        self.validate_supports_in_tx(tx, subject, &revisions)
            .await?;
        let exact = revisions
            .iter()
            .filter_map(|s| {
                if let RevisionSupport::CognitionDependency(d) = s {
                    Some(d.target_revision.clone())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        if [from.clone(), to.clone()]
            .iter()
            .any(|reference| is_cognition(reference) && !exact.contains(reference))
        {
            return Err(Error::Invalid(
                "association must support each cognition endpoint".into(),
            ));
        }
        if matches!(relation, TopologyRelation::TagAttachment)
            && (!is_cognition(&from) || !matches!(to, CognitiveRef::Tag(_)))
        {
            return Err(Error::Invalid("invalid tag attachment endpoints".into()));
        }
        if !matches!(relation, TopologyRelation::TagAttachment) && exact.len() < 2 {
            return Err(Error::Invalid(
                "semantic association needs two exact cognition inputs".into(),
            ));
        }
        self.validate_relation_proof_in(
            tx,
            subject,
            &from,
            &to,
            relation,
            &exact,
            plan.policy.max_supports,
        )
        .await?;
        if self
            .provenance_summary(subject, &revisions)
            .await?
            .roots
            .is_empty()
        {
            return Err(Error::Invalid(
                "derived association has no provenance roots".into(),
            ));
        }
        let (fk, fv) = reference_parts(&from);
        let (tk, tv) = reference_parts(&to);
        let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM association_evidence WHERE subject_id=$1 AND from_ref_kind=$2 AND from_ref=$3 AND to_ref_kind=$4 AND to_ref=$5 AND relation_kind=$6 AND revoked_at IS NULL)")
            .bind(subject.0).bind(fk).bind(fv).bind(tk).bind(tv).bind(relation.as_str()).fetch_one(&mut **tx).await.map_err(db)?;
        if duplicate {
            return Err(Error::Invalid(
                "duplicate active topology association".into(),
            ));
        }
        self.insert_association_in(
            tx,
            subject,
            &CreateAssociationRequest {
                operation_id: operation,
                from,
                to,
                relation_kind: relation.as_str().into(),
                polarity: AssociationPolarity::Positive,
                support_class: AssociationSupportClass::CognitiveDerivation,
                supports,
                producer_signature_id: Some(producer),
                valid_time: TemporalExtent::Unknown,
            },
        )
        .await?;
        Ok(())
    }
}
fn selected_supports(plan: &TopologyPlan, keys: &[String]) -> Result<Vec<AssociationSupport>> {
    if keys.is_empty() || keys.len() > 16 || keys.iter().collect::<HashSet<_>>().len() != keys.len()
    {
        return Err(Error::Invalid("invalid topology support selection".into()));
    }
    keys.iter()
        .map(|key| {
            plan.supports
                .get(key)
                .cloned()
                .ok_or_else(|| Error::Invalid("support key outside catalog".into()))
        })
        .collect()
}
fn selected_revisions(plan: &TopologyPlan, keys: &[String]) -> Result<Vec<RevisionSupport>> {
    selected_supports(plan, keys)?
        .into_iter()
        .map(|s| {
            if let AssociationSupport::Revision(r) = s {
                Ok(r)
            } else {
                Err(Error::Invalid(
                    "operation requires revision supports".into(),
                ))
            }
        })
        .collect()
}
fn require_cognition_support(supports: &[RevisionSupport]) -> Result<()> {
    if !supports
        .iter()
        .any(|s| matches!(s, RevisionSupport::CognitionDependency(_)))
    {
        return Err(Error::Invalid(
            "topology action requires exact cognition support".into(),
        ));
    }
    Ok(())
}
fn is_cognition(reference: &CognitiveRef) -> bool {
    matches!(
        reference,
        CognitiveRef::MemoryRevision(_)
            | CognitiveRef::CognitiveSchemaRevision(_)
            | CognitiveRef::EpisodeRevision(_)
            | CognitiveRef::JournalRevision(_)
    )
}
fn reference(refs: &HashMap<String, CognitiveRef>, key: &str) -> Result<CognitiveRef> {
    refs.get(key)
        .cloned()
        .ok_or_else(|| Error::Invalid("reference key outside catalog".into()))
}
fn tag_target(plan: &TopologyPlan, key: &str) -> Result<TagExpectation> {
    plan.tags
        .iter()
        .find(|tag| tag.key == key)
        .map(|tag| tag.target.clone())
        .ok_or_else(|| Error::Invalid("Tag key outside catalog".into()))
}
fn new_key(refs: &HashMap<String, CognitiveRef>, key: &str) -> Result<()> {
    if !key.starts_with("new_")
        || key.len() > 36
        || !key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        || refs.contains_key(key)
    {
        return Err(Error::Invalid(
            "invalid or duplicate new concept key".into(),
        ));
    }
    Ok(())
}
fn reason_valid(reason: &str) -> Result<()> {
    if reason.trim().is_empty() || reason.chars().count() > 1024 {
        return Err(Error::Invalid(
            "topology reason requires 1..1024 characters".into(),
        ));
    }
    Ok(())
}
