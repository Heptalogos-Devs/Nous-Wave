use super::*;

#[tonic::async_trait]
impl k::kernel_projection_service_server::KernelProjectionService for KernelService {
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
}
