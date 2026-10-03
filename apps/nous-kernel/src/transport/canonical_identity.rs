use super::*;

#[tonic::async_trait]
impl p::identity_service_server::IdentityService for KernelService {
    async fn bind_identity(
        &self,
        request: Request<p::BindIdentityRequest>,
    ) -> std::result::Result<Response<p::IdentityBinding>, Status> {
        rpc_reply(KernelService::bind_identity(self, request.into_inner())).await
    }
    async fn resolve_identity(
        &self,
        request: Request<p::ResolveIdentityRequest>,
    ) -> std::result::Result<Response<p::ResolveIdentityResponse>, Status> {
        rpc_reply(KernelService::resolve_identity(self, request.into_inner())).await
    }
}
