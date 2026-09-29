//! Private protocol adapters. Domain owners remain the semantic boundary.
mod authority;
mod convert;
mod evidence;
mod identity;
mod management;
mod material;
mod memory;
mod model;
mod query;
mod runtime;
mod schema;
mod subject;
mod topology;

use crate::NousRuntime;
use convert::*;
use nous_core::{Error, SessionId, SubjectId};
use nous_protocol::{kernel as k, public as p};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct KernelService(pub NousRuntime);

impl KernelService {
    pub fn require_memory(&self) -> nous_core::Result<&nous_memory::MemoryService> {
        self.0.require_memory()
    }

    pub async fn build_contribution_batch(
        &self,
        input: p::ProjectionRequest,
    ) -> nous_core::Result<p::Projection> {
        let subject = SubjectId(id(&input.subject_id)?);
        let session = SessionId(id(&input.session_id)?);
        let runtime = self.0.cognition.session(subject, session).await?;
        Ok(p::Projection {
            projection_id: uuid::Uuid::now_v7().to_string(),
            consumer_id: input.consumer_id,
            source_runtime_revision: runtime.runtime_revision,
            segments: Vec::new(),
            degradation: Vec::new(),
        })
    }
}

pub fn status(error: Error) -> Status {
    match error {
        Error::Invalid(message) => Status::invalid_argument(message),
        Error::NotFound(message) => Status::not_found(message),
        Error::Conflict(message) => Status::aborted(message),
        Error::FailedPrecondition(message) => Status::failed_precondition(message),
        Error::Unavailable(message) => Status::unavailable(message),
        Error::Internal(message) => {
            tracing::error!(%message, "kernel internal operation failed");
            Status::internal("Kernel operation failed")
        }
        Error::Infrastructure(message) => {
            tracing::error!(%message, "kernel operation failed");
            Status::internal("Kernel operation failed")
        }
    }
}
