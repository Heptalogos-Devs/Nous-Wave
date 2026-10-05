use super::*;

#[tonic::async_trait]
impl p::topology_service_server::TopologyService for KernelService {
    async fn create_tag(
        &self,
        request: Request<p::CreateTagRequest>,
    ) -> std::result::Result<Response<p::Tag>, Status> {
        rpc_reply(KernelService::create_tag(self, request.into_inner())).await
    }
    async fn get_tag(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Tag>, Status> {
        rpc_reply(KernelService::get_tag(self, request.into_inner())).await
    }
    async fn list_tags(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListTagsResponse>, Status> {
        rpc_reply(KernelService::list_tags(self, request.into_inner())).await
    }
    async fn search_tags(
        &self,
        request: Request<p::SearchTagsRequest>,
    ) -> std::result::Result<Response<p::ListTagsResponse>, Status> {
        rpc_reply(KernelService::search_tags(self, request.into_inner())).await
    }
    async fn revise_tag(
        &self,
        request: Request<p::ReviseTagRequest>,
    ) -> std::result::Result<Response<p::Tag>, Status> {
        rpc_reply(KernelService::revise_tag(self, request.into_inner())).await
    }
    async fn merge_tags(
        &self,
        request: Request<p::MergeTagsRequest>,
    ) -> std::result::Result<Response<p::Tag>, Status> {
        rpc_reply(KernelService::merge_tags(self, request.into_inner())).await
    }
    async fn split_tag(
        &self,
        request: Request<p::SplitTagRequest>,
    ) -> std::result::Result<Response<p::SplitTagResponse>, Status> {
        rpc_reply(KernelService::split_tag(self, request.into_inner())).await
    }
    async fn create_association(
        &self,
        request: Request<p::CreateAssociationRequest>,
    ) -> std::result::Result<Response<p::Association>, Status> {
        rpc_reply(KernelService::create_association(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn revoke_association(
        &self,
        request: Request<p::RevokeAssociationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::revoke_association(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn get_neighborhood(
        &self,
        request: Request<p::NeighborhoodRequest>,
    ) -> std::result::Result<Response<p::NeighborhoodResponse>, Status> {
        rpc_reply(KernelService::get_neighborhood(self, request.into_inner())).await
    }
    async fn rebind_entity(
        &self,
        request: Request<p::RebindEntityRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::rebind_entity(self, request.into_inner())).await
    }
    async fn create_cognitive_schema(
        &self,
        request: Request<p::CreateCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        rpc_reply(KernelService::create_cognitive_schema(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn get_cognitive_schema(
        &self,
        request: Request<p::GetCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        rpc_reply(KernelService::get_cognitive_schema(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn add_schema_evidence(
        &self,
        request: Request<p::AddSchemaEvidenceRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        rpc_reply(KernelService::add_schema_evidence(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn revise_cognitive_schema(
        &self,
        request: Request<p::ReviseCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        rpc_reply(KernelService::revise_cognitive_schema(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn split_cognitive_schema(
        &self,
        request: Request<p::SplitCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::SplitCognitiveSchemaResponse>, Status> {
        rpc_reply(KernelService::split_cognitive_schema(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn merge_cognitive_schemas(
        &self,
        request: Request<p::MergeCognitiveSchemasRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        rpc_reply(KernelService::merge_cognitive_schemas(
            self,
            request.into_inner(),
        ))
        .await
    }
}
