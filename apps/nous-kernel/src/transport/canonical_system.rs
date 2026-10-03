use super::*;

#[tonic::async_trait]
impl p::system_service_server::SystemService for KernelService {
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
    async fn get_capabilities(
        &self,
        _: Request<()>,
    ) -> std::result::Result<Response<p::SystemStatus>, Status> {
        rpc_reply(KernelService::get_status(self, ())).await
    }
}
