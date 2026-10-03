use super::*;

#[tonic::async_trait]
impl p::runtime_service_server::RuntimeService for KernelService {
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
    async fn report_use(
        &self,
        request: Request<p::ReportUseRequest>,
    ) -> std::result::Result<Response<p::ReportUseResponse>, Status> {
        rpc_reply(KernelService::report_use(self, request.into_inner())).await
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
}
