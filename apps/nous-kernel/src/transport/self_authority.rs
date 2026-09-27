use super::*;
use nous_core::{Error, OperationId, Result, SubjectId};
use nous_self_domain::{
    CreateNarrativeIdentity, CreateSelfFacet, MutateSelfLifecycle, NarrativeReferenceInput,
    ReviseNarrativeIdentity, ReviseSelfFacet, SelfFacetKind,
};
use nous_self_service::{NarrativeIdentityView, SelfFacetView};

fn optional_uuid(value: Option<String>) -> Result<Option<uuid::Uuid>> {
    value.map(|value| id(&value)).transpose()
}

fn required_time(value: Option<prost_types::Timestamp>) -> Result<chrono::DateTime<chrono::Utc>> {
    time(Some(required(value, "formed_at")?))?
        .ok_or_else(|| Error::Invalid("formed_at is required".into()))
}

fn facet_view(value: SelfFacetView) -> p::SelfFacet {
    p::SelfFacet {
        self_facet_id: value.object.self_facet_id.0.to_string(),
        subject_id: value.object.subject_id.0.to_string(),
        kind: enum_name(value.object.kind),
        key: value.object.key,
        current_revision_id: value.object.current_revision_id.0.to_string(),
        object_epoch: value.object.object_epoch,
        acceptance_state: enum_name(value.object.acceptance_state),
        integrity_state: enum_name(value.object.integrity_state),
        suppression_state: enum_name(value.object.suppression_state),
        purge_state: enum_name(value.object.purge_state),
        revision_no: value.revision.revision_no,
        parent_revision_id: value.revision.parent_revision_id.map(|id| id.0.to_string()),
        revision_intent: value.revision.revision_intent.map(enum_name),
        statement: value.revision.statement,
        scope: value.revision.scope,
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
        producer_signature_id: value
            .revision
            .producer_signature_id
            .map(|id| id.to_string()),
        created_at: Some(timestamp(value.object.created_at)),
    }
}

fn narrative_view(value: NarrativeIdentityView) -> p::NarrativeIdentity {
    p::NarrativeIdentity {
        narrative_identity_id: value.object.narrative_identity_id.0.to_string(),
        subject_id: value.object.subject_id.0.to_string(),
        key: value.object.key,
        current_revision_id: value.object.current_revision_id.0.to_string(),
        object_epoch: value.object.object_epoch,
        acceptance_state: enum_name(value.object.acceptance_state),
        integrity_state: enum_name(value.object.integrity_state),
        suppression_state: enum_name(value.object.suppression_state),
        purge_state: enum_name(value.object.purge_state),
        revision_no: value.revision.revision_no,
        parent_revision_id: value.revision.parent_revision_id.map(|id| id.0.to_string()),
        revision_intent: value.revision.revision_intent.map(enum_name),
        text: value.revision.text,
        valid_time: Some(temporal_proto(&value.revision.valid_time)),
        formed_at: Some(timestamp(value.revision.formed_at)),
        recorded_at: Some(timestamp(value.revision.recorded_at)),
        supports: value
            .revision
            .supports
            .into_iter()
            .map(support_proto)
            .collect(),
        references: value
            .revision
            .references
            .into_iter()
            .map(|reference| p::NarrativeReference {
                target_exact_ref: Some(to_ref(reference.target_exact_ref)),
                role: reference.role,
            })
            .collect(),
        producer_signature_id: value
            .revision
            .producer_signature_id
            .map(|id| id.to_string()),
        created_at: Some(timestamp(value.object.created_at)),
    }
}

fn narrative_refs(values: Vec<p::NarrativeReference>) -> Result<Vec<NarrativeReferenceInput>> {
    values
        .into_iter()
        .map(|value| {
            Ok(NarrativeReferenceInput {
                target_exact_ref: from_ref(required(value.target_exact_ref, "target_exact_ref")?)?,
                role: value.role,
            })
        })
        .collect()
}

impl KernelService {
    pub(super) async fn create_self_facet(
        &self,
        input: p::CreateSelfFacetRequest,
    ) -> Result<p::SelfFacet> {
        let subject = SubjectId(id(&input.subject_id)?);
        let value = self
            .0
            .self_cognition
            .create_facet(CreateSelfFacet {
                operation_id: OperationId(id(&input.operation_id)?),
                subject,
                kind: SelfFacetKind::try_from(input.kind.as_str())?,
                key: input.key,
                statement: input.statement,
                scope: input.scope,
                epistemic_class: enum_value(&input.epistemic_class)?,
                valid_time: temporal(input.valid_time)?,
                formed_at: required_time(input.formed_at)?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(facet_view(value))
    }

    pub(super) async fn get_self_facet(&self, input: p::ObjectRequest) -> Result<p::SelfFacet> {
        Ok(facet_view(
            self.0
                .self_cognition
                .facet(
                    SubjectId(id(&input.subject_id)?),
                    nous_core::SelfFacetId(id(&input.id)?),
                )
                .await?,
        ))
    }

    pub(super) async fn revise_self_facet(
        &self,
        input: p::ReviseSelfFacetRequest,
    ) -> Result<p::SelfFacet> {
        let value = self
            .0
            .self_cognition
            .revise_facet(ReviseSelfFacet {
                operation_id: OperationId(id(&input.operation_id)?),
                subject: SubjectId(id(&input.subject_id)?),
                self_facet_id: nous_core::SelfFacetId(id(&input.self_facet_id)?),
                expected_object_epoch: input.expected_object_epoch,
                parent_revision_id: nous_core::SelfFacetRevisionId(id(&input.parent_revision_id)?),
                revision_intent: enum_value(&input.revision_intent)?,
                statement: input.statement,
                scope: input.scope,
                epistemic_class: enum_value(&input.epistemic_class)?,
                valid_time: temporal(input.valid_time)?,
                formed_at: required_time(input.formed_at)?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(facet_view(value))
    }

    pub(super) async fn mutate_self_lifecycle(&self, input: p::SelfLifecycleRequest) -> Result<()> {
        self.0
            .self_cognition
            .lifecycle(MutateSelfLifecycle {
                operation_id: OperationId(id(&input.operation_id)?),
                subject: SubjectId(id(&input.subject_id)?),
                reference: from_ref(required(input.reference, "reference")?)?,
                expected_object_epoch: input.expected_object_epoch,
                operation: enum_value(&input.operation)?,
            })
            .await
    }

    pub(super) async fn create_narrative_identity(
        &self,
        input: p::CreateNarrativeIdentityRequest,
    ) -> Result<p::NarrativeIdentity> {
        let value = self
            .0
            .self_cognition
            .create_narrative(CreateNarrativeIdentity {
                operation_id: OperationId(id(&input.operation_id)?),
                subject: SubjectId(id(&input.subject_id)?),
                key: input.key,
                text: input.text,
                valid_time: temporal(input.valid_time)?,
                formed_at: required_time(input.formed_at)?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                references: narrative_refs(input.references)?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(narrative_view(value))
    }

    pub(super) async fn get_narrative_identity(
        &self,
        input: p::ObjectRequest,
    ) -> Result<p::NarrativeIdentity> {
        Ok(narrative_view(
            self.0
                .self_cognition
                .narrative(
                    SubjectId(id(&input.subject_id)?),
                    nous_core::NarrativeIdentityId(id(&input.id)?),
                )
                .await?,
        ))
    }

    pub(super) async fn revise_narrative_identity(
        &self,
        input: p::ReviseNarrativeIdentityRequest,
    ) -> Result<p::NarrativeIdentity> {
        let value = self
            .0
            .self_cognition
            .revise_narrative(ReviseNarrativeIdentity {
                operation_id: OperationId(id(&input.operation_id)?),
                subject: SubjectId(id(&input.subject_id)?),
                narrative_identity_id: nous_core::NarrativeIdentityId(id(
                    &input.narrative_identity_id
                )?),
                expected_object_epoch: input.expected_object_epoch,
                parent_revision_id: nous_core::NarrativeIdentityRevisionId(id(
                    &input.parent_revision_id
                )?),
                revision_intent: enum_value(&input.revision_intent)?,
                text: input.text,
                valid_time: temporal(input.valid_time)?,
                formed_at: required_time(input.formed_at)?,
                supports: input
                    .supports
                    .into_iter()
                    .map(support)
                    .collect::<Result<_>>()?,
                references: narrative_refs(input.references)?,
                producer_signature_id: optional_uuid(input.producer_signature_id)?,
            })
            .await?;
        Ok(narrative_view(value))
    }
}
