use super::*;
use std::collections::BTreeSet;

impl ConsolidationMemoryContent {
    fn input(&self, request: &LongitudinalConsolidationInput) -> ExplicitMemoryInput {
        ExplicitMemoryInput {
            producer: Some(request.producer.clone()),
            operation_id: request.operation_id,
            subject: request.subject,
            cognitive_role: self.cognitive_role,
            formation_mode: self.formation_mode,
            grounding_occurrence_id: self.grounding_occurrence_id,
            semantic_role: self.semantic_role.clone(),
            representation_text: self.representation_text.clone(),
            title: self.title.clone(),
            supports: self.supports.clone(),
            aboutness: self.aboutness.clone(),
            tags: vec![],
            valid_time: self.valid_time.clone(),
            epistemic_class: self.epistemic_class,
        }
    }
}
impl ConsolidationSchemaContent {
    fn input(
        &self,
        request: &LongitudinalConsolidationInput,
        formed: DateTime<Utc>,
    ) -> CreateSchemaInput {
        CreateSchemaInput {
            operation_id: request.operation_id,
            subject: request.subject,
            title: self.title.clone(),
            structural_claim: self.structural_claim.clone(),
            applicability_scope: self.applicability_scope.clone(),
            boundary_definition: self.boundary_definition.clone(),
            formed_at: formed,
            formation_kind: self.formation_kind,
            evidence_links: self.evidence_links.clone(),
        }
    }
}

impl MemoryService {
    pub(super) async fn validate_consolidation_action(
        &self,
        request: &LongitudinalConsolidationInput,
        action: &LongitudinalConsolidationAction,
        allowed: &BTreeSet<String>,
        formed: DateTime<Utc>,
    ) -> Result<()> {
        let supports = match action {
            LongitudinalConsolidationAction::Skip { reason } => {
                if reason.trim().is_empty() || reason.len() > 8192 {
                    return Err(Error::Invalid(
                        "Consolidation skip reason is out of bounds".into(),
                    ));
                }
                return Ok(());
            }
            LongitudinalConsolidationAction::CreateMemory { content }
            | LongitudinalConsolidationAction::ReviseMemory { content, .. } => {
                let input = content.input(request);
                self.validate_consolidation_aboutness(request.subject, &input.aboutness)
                    .await?;
                input.validate()?;
                self.validate_supports_for_subject(request.subject, &input.supports)
                    .await?;
                self.validate_formation_semantics(
                    request.subject,
                    input.formation_mode,
                    &input.supports,
                )
                .await?;
                input.supports
            }
            LongitudinalConsolidationAction::CreateSchema { content }
            | LongitudinalConsolidationAction::ReviseSchema { content, .. } => {
                self.validate_consolidation_aboutness(
                    request.subject,
                    &content.applicability_scope.aboutness,
                )
                .await?;
                self.validate_schema_formation(&content.input(request, formed))
                    .await?;
                content
                    .evidence_links
                    .iter()
                    .map(|link| link.support.clone())
                    .collect()
            }
            LongitudinalConsolidationAction::LinkRelation { .. } => return Ok(()),
        };
        if supports.len() > 128
            || supports
                .iter()
                .any(|support| !allowed.contains(&support.canonical_key()))
        {
            return Err(Error::Invalid(
                "Consolidation supports are outside its exact source catalog".into(),
            ));
        }
        Ok(())
    }

    async fn validate_consolidation_aboutness(
        &self,
        subject: SubjectId,
        aboutness: &[EntityRef],
    ) -> Result<()> {
        if aboutness.len() > 128 {
            return Err(Error::Invalid(
                "Consolidation aboutness is out of bounds".into(),
            ));
        }
        for entity in aboutness {
            if !self
                .store
                .reference_in_subject(subject, &CognitiveRef::Entity(entity.clone()))
                .await?
            {
                return Err(Error::Invalid(
                    "Consolidation cannot invent an EntityRef".into(),
                ));
            }
        }
        Ok(())
    }

    pub(super) async fn apply_consolidation_action_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &LongitudinalConsolidationInput,
        action: &LongitudinalConsolidationAction,
        prior: &[Option<CognitiveRef>],
        formed: DateTime<Utc>,
        producer: Uuid,
    ) -> Result<(Option<CognitiveRef>, bool)> {
        let reference = match action {
            LongitudinalConsolidationAction::Skip { .. } => return Ok((None, false)),
            LongitudinalConsolidationAction::CreateMemory { content } => {
                let (_, revision) = self
                    .create_memory_in(tx, &content.input(request), formed)
                    .await?;
                CognitiveRef::MemoryRevision(revision)
            }
            LongitudinalConsolidationAction::ReviseMemory {
                target,
                intent,
                content,
            } => {
                require_context_target(request, target)?;
                scope::validate_current_cognition_in(tx, request.subject, target).await?;
                let CognitiveRef::MemoryRevision(parent) = target.reference else {
                    return Err(Error::Invalid(
                        "Memory revision target must be a MemoryRevision".into(),
                    ));
                };
                let row=sqlx::query("SELECT r.memory_id,o.cognitive_role FROM memory_revisions r JOIN memory_objects o USING(memory_id) WHERE r.subject_id=$1 AND r.memory_revision_id=$2")
                    .bind(request.subject.0).bind(parent.0).fetch_one(&mut **tx).await.map_err(db)?;
                if row.get::<String, _>("cognitive_role") != content.cognitive_role.as_str() {
                    return Err(Error::Invalid(
                        "Memory revision cannot change cognitive role".into(),
                    ));
                }
                let input = ReviseMemoryInput {
                    producer: Some(request.producer.clone()),
                    operation_id: request.operation_id,
                    subject: request.subject,
                    memory_id: MemoryId(row.get("memory_id")),
                    expected_object_epoch: target.expected_epoch,
                    intent: *intent,
                    formation_mode: content.formation_mode,
                    grounding_occurrence_id: content.grounding_occurrence_id,
                    semantic_role: content.semantic_role.clone(),
                    representation_text: content.representation_text.clone(),
                    title: content.title.clone(),
                    supports: content.supports.clone(),
                    aboutness: content.aboutness.clone(),
                    valid_time: content.valid_time.clone(),
                    epistemic_class: content.epistemic_class,
                };
                let (revision, _) = self.revise_memory_in(tx, &input, formed).await?;
                CognitiveRef::MemoryRevision(revision)
            }
            LongitudinalConsolidationAction::CreateSchema { content } => {
                let (_, revision) = self
                    .create_schema_in(tx, &content.input(request, formed), Some(producer))
                    .await?;
                CognitiveRef::CognitiveSchemaRevision(revision)
            }
            LongitudinalConsolidationAction::ReviseSchema {
                target,
                intent,
                content,
            } => {
                let revision = self
                    .revise_consolidation_schema_in(
                        tx, request, target, *intent, content, formed, producer,
                    )
                    .await?;
                CognitiveRef::CognitiveSchemaRevision(revision)
            }
            LongitudinalConsolidationAction::LinkRelation { from, to, relation } => {
                return self
                    .link_consolidation_relation_in(tx, request, from, to, *relation, prior)
                    .await;
            }
        };
        Ok((Some(reference), true))
    }

    async fn revise_consolidation_schema_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &LongitudinalConsolidationInput,
        target: &ExpectedCognition,
        intent: RevisionIntent,
        content: &ConsolidationSchemaContent,
        formed: DateTime<Utc>,
        producer: Uuid,
    ) -> Result<CognitiveSchemaRevisionId> {
        require_context_target(request, target)?;
        scope::validate_current_cognition_in(tx, request.subject, target).await?;
        let CognitiveRef::CognitiveSchemaRevision(parent) = target.reference else {
            return Err(Error::Invalid(
                "Schema revision target must be a SchemaRevision".into(),
            ));
        };
        let row=sqlx::query("SELECT schema_id,revision_no,formation_kind,aboutness FROM cognitive_schema_revisions WHERE schema_revision_id=$1")
            .bind(parent.0).fetch_one(&mut **tx).await.map_err(db)?;
        if row.get::<String, _>("formation_kind") != content.formation_kind.as_str() {
            return Err(Error::Invalid(
                "Schema revision cannot change formation kind".into(),
            ));
        }
        let aboutness: Vec<String> = row.get("aboutness");
        if !aboutness.is_empty()
            && !content
                .applicability_scope
                .aboutness
                .iter()
                .any(|entity| aboutness.iter().any(|value| value == entity.as_str()))
        {
            return Err(Error::Invalid(
                "Schema revision changes its applicability identity".into(),
            ));
        }
        let schema_id = CognitiveSchemaId(row.get("schema_id"));
        let input = content.input(request, formed);
        let supports: Vec<_> = input
            .evidence_links
            .iter()
            .map(|link| link.support.clone())
            .collect();
        self.validate_object_dependency_cycle(
            request.subject,
            &format!("schema:{}", schema_id.0),
            &supports,
        )
        .await?;
        self.validate_supports_in_tx(tx, request.subject, &supports)
            .await?;
        let revision = CognitiveSchemaRevisionId::new();
        self.write_schema_revision_in(
            tx,
            &input,
            schema::SchemaRevisionWrite {
                schema_id,
                revision_id: revision,
                parent: Some(parent),
                intent: Some(intent),
                number: row.get::<i32, _>("revision_no") + 1,
                recorded_at: self.cognition.now(request.subject),
                producer: Some(producer),
            },
        )
        .await?;
        sqlx::query("UPDATE cognitive_schemas SET current_revision_id=$2,object_epoch=object_epoch+1,integrity_state='valid' WHERE schema_id=$1")
            .bind(schema_id.0).bind(revision.0).execute(&mut **tx).await.map_err(db)?;
        Ok(revision)
    }

    async fn link_consolidation_relation_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &LongitudinalConsolidationInput,
        from: &ConsolidationResultRef,
        to: &ConsolidationResultRef,
        relation: MemoryRelation,
        prior: &[Option<CognitiveRef>],
    ) -> Result<(Option<CognitiveRef>, bool)> {
        let from = resolve_result_ref(request, from, prior)?;
        let to = resolve_result_ref(request, to, prior)?;
        let (CognitiveRef::MemoryRevision(source), CognitiveRef::MemoryRevision(target)) =
            (&from, &to)
        else {
            return Err(Error::Invalid(
                "Strong Memory relations require exact Memory revisions".into(),
            ));
        };
        if source == target {
            return Err(Error::Invalid(
                "Memory relation cannot target itself".into(),
            ));
        }
        let changed=sqlx::query("INSERT INTO memory_revision_relations(from_revision_id,to_revision_id,relation,created_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(source.0).bind(target.0).bind(relation.as_str()).bind(self.cognition.now(request.subject)).execute(&mut **tx).await.map_err(db)?.rows_affected()>0;
        Ok((Some(from), changed))
    }
}
fn require_context_target(
    input: &LongitudinalConsolidationInput,
    target: &ExpectedCognition,
) -> Result<()> {
    if !input.context.iter().any(|context| {
        context.reference == target.reference && context.expected_epoch == target.expected_epoch
    }) {
        return Err(Error::Invalid(
            "Consolidation target is outside the bound current context".into(),
        ));
    }
    Ok(())
}
fn resolve_result_ref(
    input: &LongitudinalConsolidationInput,
    target: &ConsolidationResultRef,
    prior: &[Option<CognitiveRef>],
) -> Result<CognitiveRef> {
    match target {
        ConsolidationResultRef::Existing { reference } => {
            if !input
                .context
                .iter()
                .any(|context| &context.reference == reference)
            {
                return Err(Error::Invalid(
                    "Consolidation relation endpoint is outside context".into(),
                ));
            }
            Ok(reference.clone())
        }
        ConsolidationResultRef::Action { index } => {
            prior.get(*index).and_then(Clone::clone).ok_or_else(|| {
                Error::Invalid(
                    "Consolidation relation needs an earlier committed action result".into(),
                )
            })
        }
    }
}
