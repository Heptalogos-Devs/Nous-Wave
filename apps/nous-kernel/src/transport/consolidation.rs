use super::*;
use nous_core::{EntityRef, OperationId, Result, TagId};
use nous_memory::{
    ConsolidationMemoryContent, ConsolidationResultRef, ConsolidationSchemaContent,
    ExpectedCognition, LongitudinalConsolidationAction, LongitudinalConsolidationInput,
    SchemaEvidenceLinkInput, SchemaScope,
};

fn expected(value: k::ExpectedCognition) -> Result<ExpectedCognition> {
    Ok(ExpectedCognition {
        reference: from_ref(required(value.reference, "reference")?)?,
        expected_epoch: value.expected_epoch,
    })
}
fn memory_content(value: k::ConsolidationMemoryContent) -> Result<ConsolidationMemoryContent> {
    Ok(ConsolidationMemoryContent {
        cognitive_role: enum_value(&value.cognitive_role)?,
        formation_mode: enum_value(&value.formation_mode)?,
        grounding_occurrence_id: value
            .grounding_occurrence_id
            .as_deref()
            .map(id)
            .transpose()?
            .map(nous_core::OccurrenceId),
        semantic_role: value.semantic_role,
        representation_text: value.text,
        title: value.title,
        supports: value
            .supports
            .into_iter()
            .map(support)
            .collect::<Result<_>>()?,
        aboutness: value
            .aboutness
            .into_iter()
            .map(EntityRef::new)
            .collect::<Result<_>>()?,
        valid_time: temporal(value.valid_time)?,
        epistemic_class: enum_value(&value.epistemic_class)?,
    })
}
fn schema_content(value: k::ConsolidationSchemaContent) -> Result<ConsolidationSchemaContent> {
    let scope = required(value.applicability_scope, "applicability_scope")?;
    Ok(ConsolidationSchemaContent {
        title: value.title,
        structural_claim: value.structural_claim,
        applicability_scope: SchemaScope {
            description: scope.description,
            aboutness: scope
                .aboutness
                .into_iter()
                .map(EntityRef::new)
                .collect::<Result<_>>()?,
            tags: scope
                .tags
                .iter()
                .map(|value| id(value).map(TagId))
                .collect::<Result<_>>()?,
            valid_time: temporal(scope.valid_time)?,
        },
        boundary_definition: value.boundary_definition,
        formation_kind: enum_value(&value.formation_kind)?,
        evidence_links: value
            .evidence_links
            .into_iter()
            .map(|link| {
                if !link.link_id.is_empty() {
                    return Err(Error::Invalid(
                        "Consolidation evidence link IDs are owner-assigned".into(),
                    ));
                }
                Ok(SchemaEvidenceLinkInput {
                    role: enum_value(&link.role)?,
                    support: support(required(link.support, "link.support")?)?,
                })
            })
            .collect::<Result<_>>()?,
    })
}
fn result_ref(value: k::ConsolidationResultRef) -> Result<ConsolidationResultRef> {
    Ok(match required(value.target, "relation target")? {
        k::consolidation_result_ref::Target::Reference(reference) => {
            ConsolidationResultRef::Existing {
                reference: from_ref(reference)?,
            }
        }
        k::consolidation_result_ref::Target::ActionIndex(index) => ConsolidationResultRef::Action {
            index: index as usize,
        },
    })
}
fn action(value: k::LongitudinalConsolidationAction) -> Result<LongitudinalConsolidationAction> {
    use k::longitudinal_consolidation_action::Action;
    Ok(match required(value.action, "consolidation action")? {
        Action::Skip(value) => LongitudinalConsolidationAction::Skip {
            reason: value.reason,
        },
        Action::CreateMemory(value) => LongitudinalConsolidationAction::CreateMemory {
            content: memory_content(required(value.content, "content")?)?,
        },
        Action::ReviseMemory(value) => LongitudinalConsolidationAction::ReviseMemory {
            target: expected(required(value.target, "target")?)?,
            intent: enum_value(&value.intent)?,
            content: memory_content(required(value.content, "content")?)?,
        },
        Action::CreateSchema(value) => LongitudinalConsolidationAction::CreateSchema {
            content: schema_content(required(value.content, "content")?)?,
        },
        Action::ReviseSchema(value) => LongitudinalConsolidationAction::ReviseSchema {
            target: expected(required(value.target, "target")?)?,
            intent: enum_value(&value.intent)?,
            content: schema_content(required(value.content, "content")?)?,
        },
        Action::LinkRelation(value) => LongitudinalConsolidationAction::LinkRelation {
            from: result_ref(required(value.from, "from")?)?,
            to: result_ref(required(value.to, "to")?)?,
            relation: enum_value(&value.relation)?,
        },
    })
}
impl KernelService {
    pub(super) async fn commit_longitudinal_consolidation(
        &self,
        input: k::CommitLongitudinalConsolidationRequest,
    ) -> Result<k::CommitLongitudinalConsolidationResponse> {
        let outcome = self
            .require_memory()?
            .commit_longitudinal_consolidation(LongitudinalConsolidationInput {
                operation_id: OperationId(id(&input.operation_id)?),
                subject: SubjectId(id(&input.subject_id)?),
                expected_authority_seq: input.expected_authority_seq,
                source: expected(required(input.source, "source")?)?,
                context: input
                    .context
                    .into_iter()
                    .map(expected)
                    .collect::<Result<_>>()?,
                producer: from_producer(required(input.producer, "producer")?)?,
                actions: input
                    .actions
                    .into_iter()
                    .map(action)
                    .collect::<Result<_>>()?,
            })
            .await?;
        Ok(k::CommitLongitudinalConsolidationResponse {
            status: outcome.status,
            authority_seq: outcome.authority_seq,
            results: outcome
                .results
                .into_iter()
                .enumerate()
                .map(|(index, reference)| k::ConsolidationActionResult {
                    action_index: index as u32,
                    reference: reference.map(to_ref),
                })
                .collect(),
        })
    }
}
