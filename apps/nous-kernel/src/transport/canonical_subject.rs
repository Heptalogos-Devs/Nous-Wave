use super::*;

#[tonic::async_trait]
impl p::subject_service_server::SubjectService for KernelService {
    async fn create_subject(
        &self,
        request: Request<p::CreateSubjectRequest>,
    ) -> std::result::Result<Response<p::Subject>, Status> {
        rpc_reply(KernelService::create_subject(self, request.into_inner())).await
    }
    async fn get_subject(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::Subject>, Status> {
        rpc_reply(KernelService::get_subject(self, request.into_inner())).await
    }
    async fn list_subjects(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListSubjectsResponse>, Status> {
        rpc_reply(KernelService::list_subjects(self, request.into_inner())).await
    }
    async fn get_cognitive_seed(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::CognitiveSeedVersion>, Status> {
        rpc_reply(KernelService::get_cognitive_seed(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn adopt_cognitive_seed(
        &self,
        request: Request<p::AdoptCognitiveSeedRequest>,
    ) -> std::result::Result<Response<p::CognitiveSeedVersion>, Status> {
        rpc_reply(KernelService::adopt_cognitive_seed(
            self,
            request.into_inner(),
        ))
        .await
    }
}
