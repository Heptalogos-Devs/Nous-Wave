// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::model::{private_workflow_owner, workflow_json};
use super::*;
use nous_core::Result;
use nous_persistence::database_error as db;
use sqlx::Row;
#[tonic::async_trait]
impl k::kernel_model_workflow_service_server::KernelModelWorkflowService for KernelService {
    async fn find_workflow(
        &self,
        request: Request<k::FindWorkflowRequest>,
    ) -> std::result::Result<Response<k::FoundWorkflow>, Status> {
        let input = request.into_inner();
        let result:Result<_>=async {
            private_workflow_owner(&input.owner)?;
            let row=sqlx::query("SELECT semantic_digest,snapshot,proposal,outcome FROM model_workflow_operations WHERE subject_id=$1 AND owner=$2 AND operation_key=$3").bind(id(&input.subject_id)?).bind(&input.owner).bind(&input.operation_key).fetch_optional(self.0.store.pool()).await.map_err(db)?;
            let Some(row)=row else{return Ok(k::FoundWorkflow{found:false,snapshot_json:None,proposal_json:None,outcome_json:None});};
            if row.try_get::<String,_>("semantic_digest").map_err(db)?!=input.semantic_digest{return Err(Error::Conflict("model operation identity has different semantic input".into()));}
            Ok(k::FoundWorkflow{found:true,snapshot_json:Some(row.try_get::<serde_json::Value,_>("snapshot").map_err(db)?.to_string()),proposal_json:row.try_get::<Option<serde_json::Value>,_>("proposal").map_err(db)?.map(|value|value.to_string()),outcome_json:row.try_get::<Option<serde_json::Value>,_>("outcome").map_err(db)?.map(|value|value.to_string())})
        }.await;
        result.map(Response::new).map_err(status)
    }
    async fn reserve_workflow(
        &self,
        request: Request<k::ReserveWorkflowRequest>,
    ) -> std::result::Result<Response<k::WorkflowReservation>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            let subject = SubjectId(id(&input.subject_id)?);
            if input.owner == "memory" {
                id(&input.operation_key)?;
                if self.0.memory.is_none() {
                    return Err(Error::Unavailable("Memory owner unavailable".into()));
                }
            }
            private_workflow_owner(&input.owner)?;
            let mut snapshot = workflow_json(&input.snapshot_json)?;
            if let Some(need_id) = &input.maintenance_need_id {
                if input.owner != "memory" {
                    return Err(Error::Invalid(
                        "maintenance workflow route requires Memory".into(),
                    ));
                }
                let expected = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, format!("{need_id}:{}:{}", input.maintenance_trigger_authority_seq, input.maintenance_trigger_revision).as_bytes());
                if input.operation_key != expected.to_string() { return Err(Error::Invalid("maintenance workflow identity does not match obligation".into())); }
                let token = required(
                    input.maintenance_lease_token.clone(),
                    "maintenance_lease_token",
                )?;
                snapshot
                    .as_object_mut()
                    .ok_or_else(|| Error::Invalid("workflow snapshot must be an object".into()))?
                    .insert(
                        "maintenance_claim".into(),
                        serde_json::json!({"need_id":need_id,"lease_token":token,"trigger":input.maintenance_trigger_authority_seq,"trigger_revision":input.maintenance_trigger_revision}),
                    );
            } else if snapshot.get("maintenance_claim").is_some() {
                return Err(Error::Invalid(
                    "maintenance binding requires typed claim".into(),
                ));
            }
            if input.owner == "memory" {
                let object = snapshot
                    .as_object_mut()
                    .ok_or_else(|| Error::Invalid("workflow snapshot must be an object".into()))?;
                object.insert(
                    "cognitive_formed_at".into(),
                    serde_json::json!(self.0.cognition.now(subject)),
                );
            }
            let reserved = self
                .0
                .store
                .reserve_model_workflow(
                    subject,
                    &input.owner,
                    &input.operation_key,
                    &input.semantic_digest,
                    &snapshot, self.0.configuration.snapshot_for_subject(subject)?.get(nous_runtime::MODEL_WORKFLOW_LEASE)?)
                .await?;
            Ok(k::WorkflowReservation {
                snapshot_json: reserved.snapshot.to_string(),
                proposal_json: reserved.proposal.map(|value| value.to_string()),
                outcome_json: reserved.outcome.map(|value| value.to_string()),
                lease_token: reserved.lease_token.map(|value| value.to_string()),
                busy: reserved.busy,
            })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn save_workflow(
        &self,
        request: Request<k::SaveWorkflowRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let input = request.into_inner();
        let result: Result<_> = async {
            private_workflow_owner(&input.owner)?;
            let proposal = input
                .proposal_json
                .as_deref()
                .map(workflow_json)
                .transpose()?;
            let outcome = input
                .outcome_json
                .as_deref()
                .map(workflow_json)
                .transpose()?;
            self.0
                .store
                .save_model_workflow(
                    SubjectId(id(&input.subject_id)?),
                    &input.owner,
                    &input.operation_key,
                    id(&input.lease_token)?,
                    proposal.as_ref(),
                    outcome.as_ref(),
                )
                .await
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn release_workflow(
        &self,
        request: Request<k::ReleaseWorkflowRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let input = request.into_inner();
        private_workflow_owner(&input.owner).map_err(status)?;
        self.0
            .store
            .release_model_workflow(
                SubjectId(id(&input.subject_id).map_err(status)?),
                &input.owner,
                &input.operation_key,
                id(&input.lease_token).map_err(status)?,
            )
            .await
            .map(Response::new)
            .map_err(status)
    }
}
