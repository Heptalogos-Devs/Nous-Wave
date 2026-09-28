use super::*;
use nous_core::{EntityRef, OperationId, Result, SubjectId};
use nous_social_domain::{
    ConventionFormationEvidence, DegreeSemantics, FormationEvidenceKind, RelationTypeDefinition,
    SocialEntityKind, SocialParty, SocialScope, TemporalSemantics, ViewSemantics,
};
use nous_social_service::{
    CreateLanguageConvention, CreateRelationship,
    RegisterRelationType as RegisterRelationTypeInput, SocialConventionView,
    SocialRelationshipView,
};
use uuid::Uuid;

fn optional_uuid(value: Option<String>) -> Result<Option<Uuid>> {
    value.map(|value| id(&value)).transpose()
}

fn party(value: p::SocialParty) -> Result<SocialParty> {
    match (value.kind.as_str(), value.entity_ref) {
        ("subject", None) => Ok(SocialParty::SubjectSelf),
        ("person", Some(reference)) => Ok(SocialParty::Entity {
            entity_kind: SocialEntityKind::Person,
            entity_ref: EntityRef::new(reference)?,
        }),
        ("group", Some(reference)) => Ok(SocialParty::Entity {
            entity_kind: SocialEntityKind::Group,
            entity_ref: EntityRef::new(reference)?,
        }),
        ("community", Some(reference)) => Ok(SocialParty::Entity {
            entity_kind: SocialEntityKind::Community,
            entity_ref: EntityRef::new(reference)?,
        }),
        ("channel", Some(reference)) => Ok(SocialParty::Entity {
            entity_kind: SocialEntityKind::Channel,
            entity_ref: EntityRef::new(reference)?,
        }),
        _ => Err(Error::Invalid("invalid SocialParty".into())),
    }
}

fn degree(kind: &str, value: serde_json::Value) -> Result<DegreeSemantics> {
    match kind {
        "none" => Ok(DegreeSemantics::None),
        "ordinal" => Ok(DegreeSemantics::Ordinal {
            levels: serde_json::from_value(value)
                .map_err(|error| Error::Invalid(error.to_string()))?,
        }),
        "bounded_scalar" => Ok(DegreeSemantics::BoundedScalar {
            min: value
                .get("min")
                .and_then(|value| value.as_f64())
                .ok_or_else(|| Error::Invalid("degree min required".into()))?,
            max: value
                .get("max")
                .and_then(|value| value.as_f64())
                .ok_or_else(|| Error::Invalid("degree max required".into()))?,
        }),
        "typed_state" => Ok(DegreeSemantics::TypedState {
            states: serde_json::from_value(value)
                .map_err(|error| Error::Invalid(error.to_string()))?,
        }),
        _ => Err(Error::Invalid("invalid degree semantics".into())),
    }
}

fn view(value: &str, inverse_key: Option<String>) -> Result<ViewSemantics> {
    match value {
        "directed" => Ok(ViewSemantics::Directed),
        "symmetric" => Ok(ViewSemantics::SymmetricView),
        "inverse" => Ok(ViewSemantics::InverseView {
            inverse_key: inverse_key
                .ok_or_else(|| Error::Invalid("inverse_key required".into()))?,
        }),
        _ => Err(Error::Invalid("invalid relation type view".into())),
    }
}

fn temporal_kind(value: &str) -> Result<TemporalSemantics> {
    match value {
        "state" => Ok(TemporalSemantics::State),
        "interval" => Ok(TemporalSemantics::Interval),
        "instant" => Ok(TemporalSemantics::Instant),
        _ => Err(Error::Invalid("invalid temporal semantics".into())),
    }
}

fn scope(kind: String, refs: Vec<String>) -> Result<SocialScope> {
    let reference = |value: &String| EntityRef::new(value.clone());
    match kind.as_str() {
        "person" => Ok(SocialScope::Person(reference(refs.first().ok_or_else(
            || Error::Invalid("person scope reference required".into()),
        )?)?)),
        "group" => Ok(SocialScope::Group(reference(refs.first().ok_or_else(
            || Error::Invalid("group scope reference required".into()),
        )?)?)),
        "community" => Ok(SocialScope::Community(reference(
            refs.first()
                .ok_or_else(|| Error::Invalid("community scope reference required".into()))?,
        )?)),
        "channel" => Ok(SocialScope::Channel(reference(refs.first().ok_or_else(
            || Error::Invalid("channel scope reference required".into()),
        )?)?)),
        "dyad" if refs.len() == 2 => Ok(SocialScope::Dyad(
            party_string(&refs[0])?,
            party_string(&refs[1])?,
        )),
        _ => Err(Error::Invalid("invalid social scope".into())),
    }
}

fn party_string(value: &str) -> Result<SocialParty> {
    if value == "subject" {
        return Ok(SocialParty::SubjectSelf);
    }
    let (kind, reference) = value
        .split_once(':')
        .ok_or_else(|| Error::Invalid("invalid dyad party".into()))?;
    party(p::SocialParty {
        kind: kind.into(),
        entity_ref: Some(reference.into()),
    })
}

fn evidence(value: p::ConventionFormationEvidence) -> Result<ConventionFormationEvidence> {
    let kind = match value.kind.as_str() {
        "seed_direct" => FormationEvidenceKind::SeedDirect,
        "explicit_explanation" => FormationEvidenceKind::ExplicitExplanation,
        "explicit_confirmation" => FormationEvidenceKind::ExplicitConfirmation,
        "external_consistent_use" => FormationEvidenceKind::ExternalConsistentUse,
        "successful_understanding" => FormationEvidenceKind::SuccessfulUnderstanding,
        "repair_sequence" => FormationEvidenceKind::RepairSequence,
        "contextual" => FormationEvidenceKind::Contextual,
        _ => {
            return Err(Error::Invalid(
                "invalid convention formation evidence kind".into(),
            ));
        }
    };
    Ok(ConventionFormationEvidence {
        support_index: value.support_index,
        kind,
        external_actor: value.external_actor.map(EntityRef::new).transpose()?,
    })
}

fn relation_type(value: RelationTypeDefinition) -> p::RelationType {
    let (view, inverse_key) = match value.view_semantics {
        ViewSemantics::Directed => ("directed", None),
        ViewSemantics::SymmetricView => ("symmetric", None),
        ViewSemantics::InverseView { inverse_key } => ("inverse", Some(inverse_key)),
    };
    let (degree_kind, degree_config) = match value.degree_semantics {
        DegreeSemantics::None => ("none", serde_json::json!({})),
        DegreeSemantics::Ordinal { levels } => {
            ("ordinal", serde_json::to_value(levels).unwrap_or_default())
        }
        DegreeSemantics::BoundedScalar { min, max } => {
            ("bounded_scalar", serde_json::json!({"min":min,"max":max}))
        }
        DegreeSemantics::TypedState { states } => (
            "typed_state",
            serde_json::to_value(states).unwrap_or_default(),
        ),
    };
    p::RelationType {
        relation_type_id: value.relation_type_id.0.to_string(),
        subject_id: value.subject_id.0.to_string(),
        key: value.key,
        allowed_from_kinds: value.allowed_from_kinds,
        allowed_to_kinds: value.allowed_to_kinds,
        view: view.into(),
        inverse_key,
        degree_kind: degree_kind.into(),
        degree_config: to_object(degree_config),
        temporal: format_temporal(value.temporal_semantics).into(),
        created_at: Some(timestamp(value.created_at)),
    }
}

fn format_temporal(value: TemporalSemantics) -> &'static str {
    match value {
        TemporalSemantics::State => "state",
        TemporalSemantics::Interval => "interval",
        TemporalSemantics::Instant => "instant",
    }
}

fn relationship(value: SocialRelationshipView) -> p::Relationship {
    p::Relationship {
        relationship_id: value.assertion.relationship_id.0.to_string(),
        subject_id: value.assertion.subject_id.0.to_string(),
        relation_type_id: value.assertion.relation_type_id.0.to_string(),
        from: Some(party_proto(value.assertion.from)),
        to: Some(party_proto(value.assertion.to)),
        current_revision_id: value.assertion.current_revision_id.0.to_string(),
        object_epoch: value.assertion.object_epoch,
        acceptance_state: value.assertion.acceptance_state,
        integrity_state: value.assertion.integrity_state,
        suppression_state: value.assertion.suppression_state,
        purge_state: value.assertion.purge_state,
        revision_no: value.revision.revision_no,
        degree: value.revision.degree.and_then(to_object),
        epistemic_class: enum_name(value.revision.epistemic_class),
        valid_time: Some(temporal_proto(&value.revision.valid_time)),
        formed_at: Some(timestamp(value.revision.formed_at)),
        recorded_at: Some(timestamp(value.revision.recorded_at)),
        supports: value
            .revision
            .supports
            .into_iter()
            .map(support_proto)
            .collect(),
    }
}

fn party_proto(value: SocialParty) -> p::SocialParty {
    match value {
        SocialParty::SubjectSelf => p::SocialParty {
            kind: "subject".into(),
            entity_ref: None,
        },
        SocialParty::Entity {
            entity_kind,
            entity_ref,
        } => p::SocialParty {
            kind: entity_kind.as_str().into(),
            entity_ref: Some(entity_ref.as_str().into()),
        },
    }
}

fn convention(value: SocialConventionView) -> p::LanguageConvention {
    let (scope_kind, scope_refs) = match value.convention.scope {
        SocialScope::Person(reference) => ("person", vec![reference.as_str().into()]),
        SocialScope::Dyad(left, right) => ("dyad", vec![left.canonical(), right.canonical()]),
        SocialScope::Group(reference) => ("group", vec![reference.as_str().into()]),
        SocialScope::Community(reference) => ("community", vec![reference.as_str().into()]),
        SocialScope::Channel(reference) => ("channel", vec![reference.as_str().into()]),
    };
    p::LanguageConvention {
        convention_id: value.convention.convention_id.0.to_string(),
        subject_id: value.convention.subject_id.0.to_string(),
        key: value.convention.key,
        expression: value.convention.expression,
        scope_kind: scope_kind.into(),
        scope_refs,
        context_scope: value.convention.context_scope,
        topic_scope: value.convention.topic_scope,
        current_revision_id: value.convention.current_revision_id.0.to_string(),
        object_epoch: value.convention.object_epoch,
        acceptance_state: value.convention.acceptance_state,
        integrity_state: value.convention.integrity_state,
        suppression_state: value.convention.suppression_state,
        purge_state: value.convention.purge_state,
        revision_no: value.revision.revision_no,
        meaning: value.revision.meaning,
        pragmatic_role: value.revision.pragmatic_role,
        epistemic_class: enum_name(value.revision.epistemic_class),
        valid_time: Some(temporal_proto(&value.revision.valid_time)),
        formed_at: Some(timestamp(value.revision.formed_at)),
        recorded_at: Some(timestamp(value.revision.recorded_at)),
        supports: value
            .revision
            .supports
            .into_iter()
            .map(support_proto)
            .collect(),
        formation_evidence: value
            .formation_evidence
            .into_iter()
            .map(|item| p::ConventionFormationEvidence {
                support_index: item.support_index,
                kind: format_evidence(item.kind).into(),
                external_actor: item.external_actor.map(|value| value.as_str().into()),
            })
            .collect(),
        formation_policy_digest: value.revision.formation_policy_digest,
    }
}

fn format_evidence(value: FormationEvidenceKind) -> &'static str {
    match value {
        FormationEvidenceKind::SeedDirect => "seed_direct",
        FormationEvidenceKind::ExplicitExplanation => "explicit_explanation",
        FormationEvidenceKind::ExplicitConfirmation => "explicit_confirmation",
        FormationEvidenceKind::ExternalConsistentUse => "external_consistent_use",
        FormationEvidenceKind::SuccessfulUnderstanding => "successful_understanding",
        FormationEvidenceKind::RepairSequence => "repair_sequence",
        FormationEvidenceKind::Contextual => "contextual",
    }
}

impl KernelService {
    pub(super) async fn register_relation_type(
        &self,
        input: p::RegisterRelationTypeRequest,
    ) -> Result<p::RelationType> {
        let subject = SubjectId(id(&input.subject_id)?);
        let definition = self
            .0
            .social
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Social Cognition is disabled".into()))?
            .register_relation_type(RegisterRelationTypeInput {
                operation_id: OperationId(id(&input.operation_id)?),
                subject,
                key: input.key,
                allowed_from_kinds: input.allowed_from_kinds,
                allowed_to_kinds: input.allowed_to_kinds,
                view_semantics: view(&input.view, input.inverse_key)?,
                degree_semantics: degree(&input.degree_kind, object(input.degree_config))?,
                temporal_semantics: temporal_kind(&input.temporal)?,
                source_seed_version_id: None,
                source_seed_path: None,
            })
            .await?;
        Ok(relation_type(definition))
    }
    pub(super) async fn create_relationship(
        &self,
        input: p::CreateRelationshipRequest,
    ) -> Result<p::Relationship> {
        let subject = SubjectId(id(&input.subject_id)?);
        let value = self
            .0
            .social
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Social Cognition is disabled".into()))?
            .create_relationship(CreateRelationship {
                operation_id: OperationId(id(&input.operation_id)?),
                subject,
                relation_type_key: input.relation_type_key,
                from: party(required(input.from, "from")?)?,
                to: party(required(input.to, "to")?)?,
                degree: Some(object(input.degree)),
                epistemic_class: enum_value(&input.epistemic_class)?,
                valid_time: temporal(input.valid_time)?,
                formed_at: time(Some(required(input.formed_at, "formed_at")?))?
                    .ok_or_else(|| Error::Invalid("formed_at is required".into()))?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(relationship(value))
    }
    pub(super) async fn create_language_convention(
        &self,
        input: p::CreateLanguageConventionRequest,
    ) -> Result<p::LanguageConvention> {
        let subject = SubjectId(id(&input.subject_id)?);
        let value = self
            .0
            .social
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Social Cognition is disabled".into()))?
            .create_language_convention(CreateLanguageConvention {
                operation_id: OperationId(id(&input.operation_id)?),
                subject,
                key: input.key,
                expression: input.expression,
                scope: scope(input.scope_kind, input.scope_refs)?,
                context_scope: input.context_scope,
                topic_scope: input.topic_scope,
                meaning: input.meaning,
                pragmatic_role: input.pragmatic_role,
                epistemic_class: enum_value(&input.epistemic_class)?,
                valid_time: temporal(input.valid_time)?,
                formed_at: time(Some(required(input.formed_at, "formed_at")?))?
                    .ok_or_else(|| Error::Invalid("formed_at is required".into()))?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                formation_evidence: input
                    .formation_evidence
                    .into_iter()
                    .map(evidence)
                    .collect::<Result<_>>()?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(convention(value))
    }
    pub(super) async fn revise_relationship(
        &self,
        input: p::ReviseRelationshipRequest,
    ) -> Result<p::Relationship> {
        let subject = SubjectId(id(&input.subject_id)?);
        let value = self
            .0
            .social
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Social Cognition is disabled".into()))?
            .revise_relationship(nous_social_service::ReviseRelationship {
                operation_id: OperationId(id(&input.operation_id)?),
                subject,
                relationship_id: nous_core::RelationshipAssertionId(id(&input.relationship_id)?),
                expected_object_epoch: input.expected_object_epoch,
                parent_revision_id: nous_core::RelationshipRevisionId(id(
                    &input.parent_revision_id
                )?),
                revision_intent: input.revision_intent,
                degree: Some(object(input.degree)),
                epistemic_class: enum_value(&input.epistemic_class)?,
                valid_time: temporal(input.valid_time)?,
                formed_at: time(Some(required(input.formed_at, "formed_at")?))?
                    .ok_or_else(|| Error::Invalid("formed_at is required".into()))?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(relationship(value))
    }
    pub(super) async fn revise_language_convention(
        &self,
        input: p::ReviseLanguageConventionRequest,
    ) -> Result<p::LanguageConvention> {
        let subject = SubjectId(id(&input.subject_id)?);
        let value = self
            .0
            .social
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Social Cognition is disabled".into()))?
            .revise_language_convention(nous_social_service::ReviseLanguageConvention {
                operation_id: OperationId(id(&input.operation_id)?),
                subject,
                convention_id: nous_core::LanguageConventionId(id(&input.convention_id)?),
                expected_object_epoch: input.expected_object_epoch,
                parent_revision_id: nous_core::LanguageConventionRevisionId(id(
                    &input.parent_revision_id
                )?),
                revision_intent: input.revision_intent,
                meaning: input.meaning,
                pragmatic_role: input.pragmatic_role,
                epistemic_class: enum_value(&input.epistemic_class)?,
                valid_time: temporal(input.valid_time)?,
                formed_at: time(Some(required(input.formed_at, "formed_at")?))?
                    .ok_or_else(|| Error::Invalid("formed_at is required".into()))?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                formation_evidence: input
                    .formation_evidence
                    .into_iter()
                    .map(evidence)
                    .collect::<Result<_>>()?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(convention(value))
    }
    pub(super) async fn mutate_social_lifecycle(
        &self,
        input: p::SocialLifecycleRequest,
    ) -> Result<()> {
        let reference = from_ref(required(input.reference, "reference")?)?;
        self.0
            .social
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Social Cognition is disabled".into()))?
            .mutate_lifecycle(nous_social_service::MutateSocialLifecycle {
                operation_id: OperationId(id(&input.operation_id)?),
                subject: SubjectId(id(&input.subject_id)?),
                reference,
                expected_object_epoch: input.expected_object_epoch,
                operation: input.operation,
            })
            .await
    }
}

#[tonic::async_trait]
impl p::social_service_server::SocialService for KernelService {
    async fn register_relation_type(
        &self,
        request: Request<p::RegisterRelationTypeRequest>,
    ) -> std::result::Result<Response<p::RelationType>, Status> {
        KernelService::register_relation_type(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }

    async fn create_relationship(
        &self,
        request: Request<p::CreateRelationshipRequest>,
    ) -> std::result::Result<Response<p::Relationship>, Status> {
        KernelService::create_relationship(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }

    async fn create_language_convention(
        &self,
        request: Request<p::CreateLanguageConventionRequest>,
    ) -> std::result::Result<Response<p::LanguageConvention>, Status> {
        KernelService::create_language_convention(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }

    async fn revise_relationship(
        &self,
        request: Request<p::ReviseRelationshipRequest>,
    ) -> std::result::Result<Response<p::Relationship>, Status> {
        KernelService::revise_relationship(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }

    async fn revise_language_convention(
        &self,
        request: Request<p::ReviseLanguageConventionRequest>,
    ) -> std::result::Result<Response<p::LanguageConvention>, Status> {
        KernelService::revise_language_convention(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }

    async fn mutate_social_lifecycle(
        &self,
        request: Request<p::SocialLifecycleRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::mutate_social_lifecycle(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
}
