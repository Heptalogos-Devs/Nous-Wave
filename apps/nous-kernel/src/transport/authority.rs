use super::*;

#[tonic::async_trait]
impl k::authority_service_server::AuthorityService for KernelService {
    async fn get_cognitive_time(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<prost_types::Timestamp>, Status> {
        let result: nous_core::Result<_> = async {
            let subject = SubjectId(id(&request.into_inner().subject_id)?);
            self.0.store.require_subject(subject).await?;
            Ok(timestamp(self.0.cognition.now(subject)))
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn commit_longitudinal_consolidation(
        &self,
        request: Request<k::CommitLongitudinalConsolidationRequest>,
    ) -> std::result::Result<Response<k::CommitLongitudinalConsolidationResponse>, Status> {
        rpc_reply(KernelService::commit_longitudinal_consolidation(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn refresh_maintenance(
        &self,
        request: Request<k::RefreshMaintenanceRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::refresh_maintenance(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn plan_maintenance(
        &self,
        request: Request<k::PlanMaintenanceRequest>,
    ) -> std::result::Result<Response<k::MaintenancePlan>, Status> {
        rpc_reply(KernelService::plan_maintenance(self, request.into_inner())).await
    }
    async fn get_maintenance_policy(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<k::MaintenancePolicy>, Status> {
        rpc_reply(KernelService::get_maintenance_policy(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn claim_maintenance(
        &self,
        request: Request<k::ClaimMaintenanceRequest>,
    ) -> std::result::Result<Response<k::ClaimMaintenanceResponse>, Status> {
        rpc_reply(KernelService::claim_maintenance(self, request.into_inner())).await
    }
    async fn finish_maintenance(
        &self,
        request: Request<k::FinishMaintenanceRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::finish_maintenance(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn organize_experience(
        &self,
        request: Request<k::OrganizeExperienceRequest>,
    ) -> std::result::Result<Response<k::OrganizeExperienceResponse>, Status> {
        rpc_reply(KernelService::organize_experience(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn apply_episode_partition(
        &self,
        request: Request<k::ApplyEpisodePartitionRequest>,
    ) -> std::result::Result<Response<k::ApplyEpisodePartitionResponse>, Status> {
        rpc_reply(KernelService::apply_episode_partition(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn commit_journal(
        &self,
        request: Request<k::CommitJournalRequest>,
    ) -> std::result::Result<Response<p::JournalResponse>, Status> {
        rpc_reply(KernelService::commit_journal(self, request.into_inner())).await
    }
    async fn create_work_context(
        &self,
        request: Request<p::CreateWorkContextRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        rpc_reply(KernelService::create_work_context(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn get_work_context(
        &self,
        request: Request<p::GetWorkContextRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        rpc_reply(KernelService::get_work_context(self, request.into_inner())).await
    }
    async fn list_work_contexts(
        &self,
        request: Request<p::ListWorkContextsRequest>,
    ) -> std::result::Result<Response<p::ListWorkContextsResponse>, Status> {
        rpc_reply(KernelService::list_work_contexts(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn update_work_context(
        &self,
        request: Request<p::UpdateWorkContextRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        rpc_reply(KernelService::update_work_context(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn pause_work_context(
        &self,
        request: Request<p::WorkContextMutationRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        rpc_reply(KernelService::pause_work_context(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn resume_work_context(
        &self,
        request: Request<p::WorkContextMutationRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        rpc_reply(KernelService::resume_work_context(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn end_work_context(
        &self,
        request: Request<p::WorkContextMutationRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        rpc_reply(KernelService::end_work_context(self, request.into_inner())).await
    }
    async fn set_active_work_context(
        &self,
        request: Request<p::SetActiveWorkContextRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        rpc_reply(KernelService::set_active_work_context(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn put_resource(
        &self,
        request: Request<p::PutResourceRequest>,
    ) -> std::result::Result<Response<p::ResourceDescriptor>, Status> {
        rpc_reply(KernelService::put_resource(self, request.into_inner())).await
    }
    async fn get_resource(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::ResourceDescriptor>, Status> {
        rpc_reply(KernelService::get_resource(self, request.into_inner())).await
    }
    async fn list_resources(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListResourcesResponse>, Status> {
        rpc_reply(KernelService::list_resources(self, request.into_inner())).await
    }
    async fn remove_resource(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::remove_resource(self, request.into_inner())).await
    }
    async fn get_status(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<p::SystemStatus>, Status> {
        request.into_inner();
        KernelService::get_status(self, ())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_projection_status(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::ProjectionStatus>, Status> {
        rpc_reply(KernelService::get_projection_status(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn build_contribution_batch(
        &self,
        request: Request<p::ProjectionRequest>,
    ) -> std::result::Result<Response<p::Projection>, Status> {
        rpc_reply(KernelService::build_contribution_batch(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn open_session(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        rpc_reply(KernelService::open_session(self, request.into_inner())).await
    }
    async fn get_session(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        rpc_reply(KernelService::get_session(self, request.into_inner())).await
    }
    async fn list_sessions(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListSessionsResponse>, Status> {
        rpc_reply(KernelService::list_sessions(self, request.into_inner())).await
    }
    async fn close_session(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        rpc_reply(KernelService::close_session(self, request.into_inner())).await
    }
    async fn record_observation(
        &self,
        request: Request<p::ObservationInput>,
    ) -> std::result::Result<Response<p::AcceptedObservation>, Status> {
        rpc_reply(KernelService::record_observation(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn query(
        &self,
        request: Request<k::KernelQueryRequest>,
    ) -> std::result::Result<Response<k::KernelQueryResponse>, Status> {
        Box::pin(KernelService::query_with_material(
            self,
            request.into_inner(),
        ))
        .await
        .map(Response::new)
        .map_err(status)
    }
    async fn finalize_query(
        &self,
        request: Request<k::FinalizeQueryRequest>,
    ) -> std::result::Result<Response<p::QueryResponse>, Status> {
        rpc_reply(KernelService::finalize_query(self, request.into_inner())).await
    }
    async fn release_query(
        &self,
        request: Request<k::ReleaseQueryRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        let input = request.into_inner();
        let result: nous_core::Result<_> = self.0.cognition.release_query(
            SubjectId(id(&input.subject_id).map_err(status)?),
            id(&input.validation_ticket).map_err(status)?,
        );
        result.map(Response::new).map_err(status)
    }
    async fn report_use(
        &self,
        request: Request<p::ReportUseRequest>,
    ) -> std::result::Result<Response<p::ReportUseResponse>, Status> {
        rpc_reply(KernelService::report_use(self, request.into_inner())).await
    }
}
