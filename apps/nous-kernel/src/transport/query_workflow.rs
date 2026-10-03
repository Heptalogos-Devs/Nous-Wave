use super::*;

#[tonic::async_trait]
impl k::kernel_query_service_server::KernelQueryService for KernelService {
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
}
