use super::*;
use nous_cognitive_runtime::{CheckpointWrite, RuntimeMutation, UseFeedback, UseFeedbackEvent};
use nous_core::{Result, SessionId, SubjectId};

#[tonic::async_trait]
impl k::runtime_store_service_server::RuntimeStoreService for KernelService {
    async fn check_references(
        &self,
        request: Request<k::CheckReferencesRequest>,
    ) -> std::result::Result<Response<k::CheckReferencesResponse>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            if input.references.len() > 256 {
                return Err(Error::Invalid("reference bound exceeded".into()));
            }
            let subject = SubjectId(id(&input.subject_id)?);
            self.0.store.require_subject(subject).await?;
            let mut valid = vec![];
            let mut invalid = vec![];
            for reference in input.references {
                if self
                    .0
                    .store
                    .reference_in_subject(subject, &from_ref(reference.clone())?)
                    .await?
                {
                    valid.push(reference);
                } else {
                    invalid.push(reference);
                }
            }
            Ok(k::CheckReferencesResponse { valid, invalid })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn read_runtime(
        &self,
        request: Request<k::ReadRuntimeRequest>,
    ) -> std::result::Result<Response<k::ReadRuntimeResponse>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let subject = SubjectId(id(&input.subject_id)?);
            let session = SessionId(id(&input.session_id)?);
            let snapshot = self
                .0
                .cognition
                .runtime_snapshot(subject, session, &input.owner_kind)
                .await?;
            let checkpoints = snapshot
                .checkpoints
                .into_iter()
                .map(|c| k::Checkpoint {
                    owner_kind: c.owner_kind,
                    owner_key: c.owner_key,
                    schema_version: c.schema_version,
                    revision: c.revision,
                    payload: c.payload,
                })
                .collect();
            Ok(k::ReadRuntimeResponse {
                checkpoints,
                session: Some(k::RuntimeHeader {
                    session_id: session.0.to_string(),
                    subject_id: subject.0.to_string(),
                    runtime_revision: snapshot.revision,
                    closed: snapshot.closed,
                    active_focus_id: snapshot.active_focus_key,
                }),
            })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn mutate_runtime(
        &self,
        request: Request<k::MutateRuntimeRequest>,
    ) -> std::result::Result<Response<k::MutateRuntimeResponse>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let runtime_revision = self
                .0
                .cognition
                .mutate_runtime(RuntimeMutation {
                    subject: SubjectId(id(&input.subject_id)?),
                    session: SessionId(id(&input.session_id)?),
                    expected_runtime_revision: input.expected_runtime_revision,
                    checkpoints: input
                        .checkpoints
                        .into_iter()
                        .map(|c| CheckpointWrite {
                            owner_kind: c.owner_kind,
                            owner_key: c.owner_key,
                            schema_version: c.schema_version,
                            expected_revision: c.revision,
                            payload: c.payload,
                        })
                        .collect(),
                    foreground: input.change_foreground.then_some(input.foreground_key),
                })
                .await?;
            Ok(k::MutateRuntimeResponse { runtime_revision })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
}
impl KernelService {
    pub(super) async fn report_use(
        &self,
        input: p::ReportUseRequest,
    ) -> Result<p::ReportUseResponse> {
        if input.events.len() > 256 || input.consumer_ref.is_empty() {
            return Err(Error::Invalid("invalid use feedback bounds".into()));
        }
        self.0
            .cognition
            .use_feedback(UseFeedback {
                subject: SubjectId(id(&input.subject_id)?),
                session_id: input
                    .session_id
                    .as_deref()
                    .map(id)
                    .transpose()?
                    .map(SessionId),
                consumer_ref: input.consumer_ref,
                events: input
                    .events
                    .into_iter()
                    .map(|event| {
                        Ok(UseFeedbackEvent {
                            event_id: nous_core::UseEventId(id(&event.event_id)?),
                            reference: from_ref(required(event.reference, "reference")?)?,
                            use_kind: enum_value(&event.kind)?,
                            occurred_at: time(Some(event.occurred_at.ok_or_else(|| {
                                Error::Invalid("occurred_at is required".into())
                            })?))?
                            .ok_or_else(|| Error::Invalid("occurred_at is invalid".into()))?,
                            context: object(event.context),
                        })
                    })
                    .collect::<Result<_>>()?,
            })
            .await
            .map(
                |(accepted_count, duplicate_count, session_runtime_revision)| {
                    p::ReportUseResponse {
                        accepted_count,
                        duplicate_count,
                        session_runtime_revision,
                    }
                },
            )
    }
}
