use super::*;
mod actions;
mod scope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedCognition {
    pub reference: CognitiveRef,
    pub expected_epoch: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationMemoryContent {
    pub cognitive_role: CognitiveRole,
    pub formation_mode: FormationMode,
    pub grounding_occurrence_id: Option<OccurrenceId>,
    pub semantic_role: String,
    pub representation_text: String,
    pub title: Option<String>,
    pub supports: Vec<RevisionSupport>,
    pub aboutness: Vec<EntityRef>,
    pub valid_time: TemporalExtent,
    pub epistemic_class: EpistemicClass,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationSchemaContent {
    pub title: Option<String>,
    pub structural_claim: String,
    pub applicability_scope: SchemaScope,
    pub boundary_definition: String,
    pub formation_kind: SchemaFormationKind,
    pub evidence_links: Vec<SchemaEvidenceLinkInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConsolidationResultRef {
    Existing { reference: CognitiveRef },
    Action { index: usize },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LongitudinalConsolidationAction {
    Skip {
        reason: String,
    },
    CreateMemory {
        content: ConsolidationMemoryContent,
    },
    ReviseMemory {
        target: ExpectedCognition,
        intent: RevisionIntent,
        content: ConsolidationMemoryContent,
    },
    CreateSchema {
        content: ConsolidationSchemaContent,
    },
    ReviseSchema {
        target: ExpectedCognition,
        intent: RevisionIntent,
        content: ConsolidationSchemaContent,
    },
    LinkRelation {
        from: ConsolidationResultRef,
        to: ConsolidationResultRef,
        relation: MemoryRelation,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongitudinalConsolidationInput {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub expected_authority_seq: i64,
    pub source: ExpectedCognition,
    pub context: Vec<ExpectedCognition>,
    pub producer: ProducerSignature,
    pub actions: Vec<LongitudinalConsolidationAction>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongitudinalConsolidationOutcome {
    pub status: String,
    pub authority_seq: i64,
    pub results: Vec<Option<CognitiveRef>>,
}
impl MemoryService {
    pub async fn commit_longitudinal_consolidation(
        &self,
        input: LongitudinalConsolidationInput,
    ) -> Result<LongitudinalConsolidationOutcome> {
        if input.operation_id.0.is_nil()
            || input.actions.is_empty()
            || input.actions.len() > 16
            || input.context.len() > 32
            || input.producer.operation != CapabilityOperation::MemoryConsolidationText
        {
            return Err(Error::Invalid(
                "invalid longitudinal consolidation envelope".into(),
            ));
        }
        self.store.require_subject(input.subject).await?;
        let started = self.cognition.now(input.subject);
        let digest = operation_digest("longitudinal_consolidation", input.subject, &input)?;
        let mut mutation = match self
            .start_mutation(
                input.subject,
                input.operation_id,
                "longitudinal_consolidation",
                &digest,
            )
            .await?
        {
            MutationStart::Replay(receipt) => {
                if receipt.state != "committed" {
                    return Err(Error::Unavailable("Consolidation is in progress".into()));
                }
                let outcome = serde_json::from_str(&receipt.result_ref.ok_or_else(|| {
                    Error::Infrastructure("Consolidation receipt has no result".into())
                })?)
                .map_err(|error| {
                    Error::Infrastructure(format!("invalid consolidation receipt: {error}"))
                })?;
                return Ok(outcome);
            }
            MutationStart::Active(mutation) => mutation,
        };
        let max_actions =
            self.configuration
                .snapshot_for_subject(input.subject)?
                .get(super::longitudinal_policy::CONSOLIDATION_MAX_ACTIONS)? as usize;
        if input.actions.len() > max_actions {
            return Err(Error::Invalid(
                "Consolidation action count exceeds policy".into(),
            ));
        }
        self.validate_consolidation_scope_in(mutation.tx(), &input)
            .await?;
        let sequence: i64 =
            sqlx::query_scalar("SELECT authority_seq FROM subjects WHERE subject_id=$1 FOR UPDATE")
                .bind(input.subject.0)
                .fetch_one(&mut **mutation.tx())
                .await
                .map_err(db)?;
        if sequence != input.expected_authority_seq {
            return Err(Error::Conflict("Consolidation snapshot is stale".into()));
        }
        let allowed = self.consolidation_support_catalog(&input).await?;
        let formed = self
            .formation_time_in(mutation.tx(), input.subject, input.operation_id, started)
            .await?;
        let producer = AuthorityStore::register_producer_in(mutation.tx(), &input.producer).await?;
        let mut results = Vec::with_capacity(input.actions.len());
        let mut changed = false;
        for action in &input.actions {
            self.validate_consolidation_action(&input, action, &allowed)
                .await?;
            let (result, mutation) = self
                .apply_consolidation_action_in(
                    mutation.tx(),
                    &input,
                    action,
                    &results,
                    formed,
                    producer,
                )
                .await?;
            results.push(result);
            changed |= mutation;
        }
        let authority_seq = if changed {
            mutation.invalidate(ProjectionInvalidation::all()).await?
        } else {
            sequence
        };
        if changed {
            for reference in results.iter().flatten() {
                self.enqueue_topology_in(
                    mutation.tx(),
                    input.subject,
                    reference.clone(),
                    authority_seq,
                )
                .await?;
            }
            self.invalidate_consolidation_targets_in(mutation.tx(), &input, authority_seq)
                .await?;
        }
        let outcome = LongitudinalConsolidationOutcome {
            status: if changed { "committed" } else { "no_change" }.into(),
            authority_seq,
            results,
        };
        let json = serde_json::to_string(&outcome)
            .map_err(|error| Error::Infrastructure(error.to_string()))?;

        mutation
            .commit("longitudinal_consolidation", Some(&json), None, None)
            .await?;
        Ok(outcome)
    }
    async fn invalidate_consolidation_targets_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        input: &LongitudinalConsolidationInput,
        sequence: i64,
    ) -> Result<()> {
        for action in &input.actions {
            let reference = match action {
                LongitudinalConsolidationAction::ReviseMemory { target, .. }
                | LongitudinalConsolidationAction::ReviseSchema { target, .. } => &target.reference,
                _ => continue,
            };
            let (kind, revision) = match reference {
                CognitiveRef::MemoryRevision(id) => ("memory_revision", id.0),
                CognitiveRef::CognitiveSchemaRevision(id) => ("cognitive_schema_revision", id.0),
                _ => {
                    return Err(Error::Infrastructure(
                        "invalid consolidation revised target".into(),
                    ));
                }
            };
            self.invalidate_cognition_dependents_in(
                tx,
                input.subject,
                kind,
                &[revision],
                sequence,
                "source_revised",
            )
            .await?;
        }
        Ok(())
    }
}
