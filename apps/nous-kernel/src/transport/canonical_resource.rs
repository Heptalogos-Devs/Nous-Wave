use super::*;

#[tonic::async_trait]
impl p::resource_registry_service_server::ResourceRegistryService for KernelService {
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
}
