use super::*;
use nous_core::{
    CognitiveSchemaId, EntityRef, OperationId, Result, SubjectId, TagId, TemporalExtent,
};
use nous_memory_domain::{
    CognitionDependency, CreateSchemaInput, EvidenceLocator, EvidenceRef, ReviseSchemaInput,
    RevisionSupport, SchemaEvidenceLinkInput, SchemaFormationKind, SchemaScope,
};
use nous_memory_service::schema::SchemaView;

fn temporal(value: Option<p::TemporalExtent>) -> Result<TemporalExtent> {
    let Some(value) = value else {
        return Ok(TemporalExtent::Unknown);
    };
    Ok(match value.value {
        Some(p::temporal_extent::Value::Instant(value)) => TemporalExtent::Instant {
            at: time(Some(value))?.ok_or_else(|| Error::Invalid("invalid instant".into()))?,
        },
        Some(p::temporal_extent::Value::Interval(value)) => TemporalExtent::Interval {
            start: time(value.start)?,
            end: time(value.end)?,
        },
        None => TemporalExtent::Unknown,
    })
}

fn temporal_proto(value: &TemporalExtent) -> p::TemporalExtent {
    let value = match value {
        TemporalExtent::Unknown => None,
        TemporalExtent::Instant { at } => Some(p::temporal_extent::Value::Instant(timestamp(*at))),
        TemporalExtent::Interval { start, end } => {
            Some(p::temporal_extent::Value::Interval(p::TimeInterval {
                start: start.map(timestamp),
                end: end.map(timestamp),
            }))
        }
    };
    p::TemporalExtent { value }
}

pub(super) fn support(value: p::RevisionSupport) -> Result<RevisionSupport> {
    match value
        .support
        .ok_or_else(|| Error::Invalid("empty schema support".into()))?
    {
        p::revision_support::Support::Evidence(value) => {
            let locator = match value
                .locator
                .ok_or_else(|| Error::Invalid("empty evidence locator".into()))?
            {
                p::evidence_ref::Locator::WholeOccurrence(_) => EvidenceLocator::WholeOccurrence,
                p::evidence_ref::Locator::SourceRegionId(value) => {
                    EvidenceLocator::SourceRegion(nous_core::SourceRegionId(id(&value)?))
                }
                p::evidence_ref::Locator::DerivedRepresentationId(value) => {
                    EvidenceLocator::DerivedRepresentation(nous_core::DerivedRepresentationId(id(
                        &value,
                    )?))
                }
                p::evidence_ref::Locator::DerivedRegionId(value) => {
                    EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(id(&value)?))
                }
            };
            Ok(RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: nous_core::OccurrenceId(id(&value.occurrence_id)?),
                locator,
                support_role: enum_value(&value.support_role)?,
            }))
        }
        p::revision_support::Support::CognitionDependency(value) => {
            Ok(RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: from_ref(required(value.target_revision, "target_revision")?)?,
                support_role: enum_value(&value.support_role)?,
            }))
        }
    }
}

fn schema_input(
    subject: SubjectId,
    operation_id: OperationId,
    value: p::CognitiveSchema,
    evidence_links: Vec<p::SchemaEvidenceLink>,
) -> Result<CreateSchemaInput> {
    let scope = value
        .applicability_scope
        .ok_or_else(|| Error::Invalid("applicability_scope is required".into()))?;
    let formed_at =
        time(value.formed_at)?.ok_or_else(|| Error::Invalid("formed_at is required".into()))?;
    let formation_kind = if value.formation_kind.is_empty() {
        SchemaFormationKind::ExplicitImport
    } else {
        enum_value(&value.formation_kind)?
    };
    Ok(CreateSchemaInput {
        operation_id,
        subject,
        title: (!value.title.is_empty()).then_some(value.title),
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
                .into_iter()
                .map(|value| Ok(TagId(id(&value)?)))
                .collect::<Result<_>>()?,
            valid_time: temporal(scope.valid_time)?,
        },
        boundary_definition: value.boundary_definition,
        formed_at,
        formation_kind,
        evidence_links: evidence_links
            .into_iter()
            .map(|link| {
                Ok(SchemaEvidenceLinkInput {
                    role: enum_value(&link.role)?,
                    support: support(required(link.support, "support")?)?,
                })
            })
            .collect::<Result<_>>()?,
    })
}

fn schema_view(value: SchemaView) -> p::CognitiveSchema {
    let revision = &value.revision;
    p::CognitiveSchema {
        schema_id: value.schema.schema_id.0.to_string(),
        subject_id: value.schema.subject_id.0.to_string(),
        current_revision_id: value.schema.current_revision_id.0.to_string(),
        object_epoch: value.schema.object_epoch,
        acceptance_state: enum_name(value.schema.acceptance_state),
        integrity_state: enum_name(value.schema.integrity_state),
        suppression_state: enum_name(value.schema.suppression_state),
        purge_state: enum_name(value.schema.purge_state),
        title: revision.title.clone().unwrap_or_default(),
        structural_claim: revision.structural_claim.clone(),
        applicability_scope: Some(p::SchemaScope {
            description: revision.applicability_scope.description.clone(),
            aboutness: revision
                .applicability_scope
                .aboutness
                .iter()
                .map(|value| value.as_str().to_owned())
                .collect(),
            tags: revision
                .applicability_scope
                .tags
                .iter()
                .map(|value| value.0.to_string())
                .collect(),
            valid_time: Some(temporal_proto(&revision.applicability_scope.valid_time)),
        }),
        boundary_definition: revision.boundary_definition.clone(),
        evidence_links: value
            .evidence_links
            .into_iter()
            .map(|link| p::SchemaEvidenceLink {
                link_id: link.link_id.0.to_string(),
                role: enum_name(link.role),
                support: Some(support_proto(link.support)),
            })
            .collect(),
        formed_at: Some(timestamp(revision.formed_at)),
        recorded_at: Some(timestamp(revision.recorded_at)),
        formation_kind: enum_name(revision.formation_kind),
    }
}

pub(super) fn support_proto(value: RevisionSupport) -> p::RevisionSupport {
    let support = match value {
        RevisionSupport::Evidence(value) => {
            let locator = match value.locator {
                EvidenceLocator::WholeOccurrence => p::evidence_ref::Locator::WholeOccurrence(true),
                EvidenceLocator::SourceRegion(id) => {
                    p::evidence_ref::Locator::SourceRegionId(id.0.to_string())
                }
                EvidenceLocator::DerivedRepresentation(id) => {
                    p::evidence_ref::Locator::DerivedRepresentationId(id.0.to_string())
                }
                EvidenceLocator::DerivedRegion(id) => {
                    p::evidence_ref::Locator::DerivedRegionId(id.0.to_string())
                }
            };
            p::revision_support::Support::Evidence(p::EvidenceRef {
                occurrence_id: value.occurrence_id.0.to_string(),
                locator: Some(locator),
                support_role: enum_name(value.support_role),
            })
        }
        RevisionSupport::CognitionDependency(value) => {
            p::revision_support::Support::CognitionDependency(p::CognitionDependency {
                target_revision: Some(to_ref(value.target_revision)),
                support_role: enum_name(value.support_role),
            })
        }
    };
    p::RevisionSupport {
        support: Some(support),
    }
}

impl KernelService {
    pub(super) async fn create_cognitive_schema(
        &self,
        input: p::CreateCognitiveSchemaRequest,
    ) -> Result<p::CognitiveSchema> {
        let subject = SubjectId(id(&input.subject_id)?);
        let schema = required(input.schema, "schema")?;
        Ok(schema_view(
            self.require_memory()?
                .create_schema(schema_input(
                    subject,
                    OperationId(id(&input.operation_id)?),
                    schema,
                    input.evidence_links,
                )?)
                .await?,
        ))
    }

    pub(super) async fn get_cognitive_schema(
        &self,
        input: p::GetCognitiveSchemaRequest,
    ) -> Result<p::CognitiveSchema> {
        Ok(schema_view(
            self.require_memory()?
                .schema(
                    SubjectId(id(&input.subject_id)?),
                    CognitiveSchemaId(id(&input.schema_id)?),
                )
                .await?,
        ))
    }

    pub(super) async fn add_schema_evidence(
        &self,
        input: p::AddSchemaEvidenceRequest,
    ) -> Result<p::CognitiveSchema> {
        let subject = SubjectId(id(&input.subject_id)?);
        let link = required(input.link, "link")?;
        let support = support(required(link.support, "support")?)?;
        Ok(schema_view(
            self.require_memory()?
                .add_schema_evidence(
                    subject,
                    CognitiveSchemaId(id(&input.schema_id)?),
                    OperationId(id(&input.operation_id)?),
                    input.expected_object_epoch,
                    SchemaEvidenceLinkInput {
                        role: enum_value(&link.role)?,
                        support,
                    },
                )
                .await?,
        ))
    }

    pub(super) async fn revise_cognitive_schema(
        &self,
        input: p::ReviseCognitiveSchemaRequest,
    ) -> Result<p::CognitiveSchema> {
        let subject = SubjectId(id(&input.subject_id)?);
        let schema = required(input.schema, "schema")?;
        let scope = schema
            .applicability_scope
            .clone()
            .ok_or_else(|| Error::Invalid("applicability_scope is required".into()))?;
        let formed_at = time(schema.formed_at)?
            .ok_or_else(|| Error::Invalid("formed_at is required".into()))?;
        let copy_link_ids = input
            .copy_link_ids
            .iter()
            .map(|value| Ok(nous_core::SchemaEvidenceLinkId(id(value)?)))
            .collect::<Result<Vec<_>>>()?;
        let value = ReviseSchemaInput {
            operation_id: OperationId(id(&input.operation_id)?),
            subject,
            schema_id: CognitiveSchemaId(id(&input.schema_id)?),
            expected_object_epoch: input.expected_object_epoch,
            intent: enum_value(&input.intent)?,
            title: (!schema.title.is_empty()).then_some(schema.title),
            structural_claim: schema.structural_claim,
            applicability_scope: SchemaScope {
                description: scope.description,
                aboutness: scope
                    .aboutness
                    .into_iter()
                    .map(EntityRef::new)
                    .collect::<Result<_>>()?,
                tags: scope
                    .tags
                    .into_iter()
                    .map(|value| Ok(TagId(id(&value)?)))
                    .collect::<Result<_>>()?,
                valid_time: temporal(scope.valid_time)?,
            },
            boundary_definition: schema.boundary_definition,
            formed_at,
            copy_link_ids,
        };
        Ok(schema_view(
            self.require_memory()?.revise_schema(value).await?,
        ))
    }

    pub(super) async fn split_cognitive_schema(
        &self,
        input: p::SplitCognitiveSchemaRequest,
    ) -> Result<p::SplitCognitiveSchemaResponse> {
        let subject = SubjectId(id(&input.subject_id)?);
        let operation = OperationId(id(&input.operation_id)?);
        let children = input
            .children
            .into_iter()
            .map(|child| {
                let links = child.evidence_links.clone();
                schema_input(subject, operation, child, links)
            })
            .collect::<Result<Vec<_>>>()?;
        let values = self
            .require_memory()?
            .split_schemas(
                subject,
                CognitiveSchemaId(id(&input.schema_id)?),
                operation,
                input.expected_object_epoch,
                children,
            )
            .await?;
        Ok(p::SplitCognitiveSchemaResponse {
            children: values.into_iter().map(schema_view).collect(),
        })
    }

    pub(super) async fn merge_cognitive_schemas(
        &self,
        input: p::MergeCognitiveSchemasRequest,
    ) -> Result<p::CognitiveSchema> {
        let subject = SubjectId(id(&input.subject_id)?);
        let operation = OperationId(id(&input.operation_id)?);
        let sources = input
            .schema_ids
            .iter()
            .map(|value| Ok(CognitiveSchemaId(id(value)?)))
            .collect::<Result<Vec<_>>>()?;
        let merged_value = required(input.merged, "merged")?;
        let merged = schema_input(
            subject,
            operation,
            merged_value.clone(),
            merged_value.evidence_links.clone(),
        )?;
        Ok(schema_view(
            self.require_memory()?
                .merge_schemas(
                    subject,
                    operation,
                    sources,
                    input.expected_object_epochs,
                    merged,
                )
                .await?,
        ))
    }
}
