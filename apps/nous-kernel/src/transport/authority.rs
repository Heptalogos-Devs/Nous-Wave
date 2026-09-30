use super::*;

#[tonic::async_trait]
impl k::authority_service_server::AuthorityService for KernelService {
    async fn get_material_limits(
        &self,
        _: Request<()>,
    ) -> std::result::Result<Response<p::MaterialLimits>, Status> {
        Ok(Response::new(p::MaterialLimits {
            max_upload_bytes: self.0.material.max_upload_bytes,
        }))
    }
    async fn create_work_context(
        &self,
        request: Request<p::CreateWorkContextRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        KernelService::create_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_work_context(
        &self,
        request: Request<p::GetWorkContextRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        KernelService::get_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_work_contexts(
        &self,
        request: Request<p::ListWorkContextsRequest>,
    ) -> std::result::Result<Response<p::ListWorkContextsResponse>, Status> {
        KernelService::list_work_contexts(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn update_work_context(
        &self,
        request: Request<p::UpdateWorkContextRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        KernelService::update_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn pause_work_context(
        &self,
        request: Request<p::WorkContextMutationRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        KernelService::pause_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn resume_work_context(
        &self,
        request: Request<p::WorkContextMutationRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        KernelService::resume_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn end_work_context(
        &self,
        request: Request<p::WorkContextMutationRequest>,
    ) -> std::result::Result<Response<p::WorkContextResponse>, Status> {
        KernelService::end_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn set_active_work_context(
        &self,
        request: Request<p::SetActiveWorkContextRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        KernelService::set_active_work_context(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn create_episode(
        &self,
        request: Request<p::CreateEpisodeRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::create_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_episode(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::get_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_episode_revision(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::EpisodeRevision>, Status> {
        KernelService::get_episode_revision(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_episodes(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListEpisodesResponse>, Status> {
        KernelService::list_episodes(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_episode_revisions(
        &self,
        request: Request<p::ListEpisodeRevisionsRequest>,
    ) -> std::result::Result<Response<p::ListEpisodeRevisionsResponse>, Status> {
        KernelService::list_episode_revisions(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn revise_episode(
        &self,
        request: Request<p::ReviseEpisodeRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::revise_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn link_episode_revisions(
        &self,
        request: Request<p::LinkEpisodeRevisionsRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::link_episode_revisions(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn suppress_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::suppress_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn restore_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::restore_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn withdraw_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::withdraw_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn reaccept_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        KernelService::reaccept_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn purge_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::purge_episode(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn set_accessibility(
        &self,
        request: Request<p::SetAccessibilityRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::set_accessibility(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn link_revisions(
        &self,
        request: Request<p::LinkRevisionsRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::link_revisions(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }

    async fn put_resource(
        &self,
        request: Request<p::PutResourceRequest>,
    ) -> std::result::Result<Response<p::ResourceDescriptor>, Status> {
        KernelService::put_resource(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_resource(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::ResourceDescriptor>, Status> {
        KernelService::get_resource(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_resources(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListResourcesResponse>, Status> {
        KernelService::list_resources(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn remove_resource(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::remove_resource(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn create_tag(
        &self,
        request: Request<p::CreateTagRequest>,
    ) -> std::result::Result<Response<p::Tag>, Status> {
        KernelService::create_tag(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_tag(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Tag>, Status> {
        KernelService::get_tag(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_tags(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListTagsResponse>, Status> {
        KernelService::list_tags(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn create_association(
        &self,
        request: Request<p::CreateAssociationRequest>,
    ) -> std::result::Result<Response<p::Association>, Status> {
        KernelService::create_association(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn revoke_association(
        &self,
        request: Request<p::RevokeAssociationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::revoke_association(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn create_cognitive_schema(
        &self,
        request: Request<p::CreateCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        KernelService::create_cognitive_schema(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_cognitive_schema(
        &self,
        request: Request<p::GetCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        KernelService::get_cognitive_schema(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn add_schema_evidence(
        &self,
        request: Request<p::AddSchemaEvidenceRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        KernelService::add_schema_evidence(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn revise_cognitive_schema(
        &self,
        request: Request<p::ReviseCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        KernelService::revise_cognitive_schema(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn split_cognitive_schema(
        &self,
        request: Request<p::SplitCognitiveSchemaRequest>,
    ) -> std::result::Result<Response<p::SplitCognitiveSchemaResponse>, Status> {
        KernelService::split_cognitive_schema(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn merge_cognitive_schemas(
        &self,
        request: Request<p::MergeCognitiveSchemasRequest>,
    ) -> std::result::Result<Response<p::CognitiveSchema>, Status> {
        KernelService::merge_cognitive_schemas(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_neighborhood(
        &self,
        request: Request<p::NeighborhoodRequest>,
    ) -> std::result::Result<Response<p::NeighborhoodResponse>, Status> {
        KernelService::get_neighborhood(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn rebind_entity(
        &self,
        request: Request<p::RebindEntityRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::rebind_entity(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
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
        KernelService::get_projection_status(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn consolidate_memory(
        &self,
        request: Request<p::ConsolidateMemoryRequest>,
    ) -> std::result::Result<Response<p::ConsolidationResponse>, Status> {
        KernelService::consolidate_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_occurrence(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Occurrence>, Status> {
        KernelService::get_occurrence(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_source_region(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::SourceRegion>, Status> {
        KernelService::get_source_region(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_derived_representation(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::DerivedRepresentation>, Status> {
        KernelService::get_derived_representation(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn bind_identity(
        &self,
        request: Request<p::BindIdentityRequest>,
    ) -> std::result::Result<Response<p::IdentityBinding>, Status> {
        KernelService::bind_identity(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn resolve_identity(
        &self,
        request: Request<p::ResolveIdentityRequest>,
    ) -> std::result::Result<Response<p::ResolveIdentityResponse>, Status> {
        KernelService::resolve_identity(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn build_contribution_batch(
        &self,
        request: Request<p::ProjectionRequest>,
    ) -> std::result::Result<Response<p::Projection>, Status> {
        KernelService::build_contribution_batch(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn create_subject(
        &self,
        request: Request<p::CreateSubjectRequest>,
    ) -> std::result::Result<Response<p::Subject>, Status> {
        KernelService::create_subject(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_subject(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::Subject>, Status> {
        KernelService::get_subject(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_subjects(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListSubjectsResponse>, Status> {
        KernelService::list_subjects(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_cognitive_seed(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::CognitiveSeedVersion>, Status> {
        KernelService::get_cognitive_seed(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn adopt_cognitive_seed(
        &self,
        request: Request<p::AdoptCognitiveSeedRequest>,
    ) -> std::result::Result<Response<p::CognitiveSeedVersion>, Status> {
        KernelService::adopt_cognitive_seed(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn open_session(
        &self,
        request: Request<p::SubjectRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        KernelService::open_session(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_session(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        KernelService::get_session(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_sessions(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListSessionsResponse>, Status> {
        KernelService::list_sessions(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn close_session(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Session>, Status> {
        KernelService::close_session(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn record_observation(
        &self,
        request: Request<p::ObservationInput>,
    ) -> std::result::Result<Response<p::AcceptedObservation>, Status> {
        KernelService::record_observation(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn query(
        &self,
        request: Request<k::KernelQueryRequest>,
    ) -> std::result::Result<Response<p::QueryResponse>, Status> {
        KernelService::query_with_material(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn report_use(
        &self,
        request: Request<p::ReportUseRequest>,
    ) -> std::result::Result<Response<p::ReportUseResponse>, Status> {
        KernelService::report_use(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_memory(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::get_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_memories(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListMemoriesResponse>, Status> {
        KernelService::list_memories(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_memory_revision(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::get_memory_revision(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_memory_revisions(
        &self,
        request: Request<p::MemoryHistoryRequest>,
    ) -> std::result::Result<Response<p::ListMemoriesResponse>, Status> {
        KernelService::list_memory_revisions(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn form_memory(
        &self,
        request: Request<p::FormMemoryRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::form_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn revise_memory(
        &self,
        request: Request<p::ReviseMemoryRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::revise_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn suppress_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::suppress_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn restore_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::restore_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn withdraw_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::withdraw_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn reaccept_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        KernelService::reaccept_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn purge_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        KernelService::purge_memory(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn get_artifact(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Artifact>, Status> {
        KernelService::get_artifact(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn list_artifacts(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListArtifactsResponse>, Status> {
        KernelService::list_artifacts(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
    async fn materialize_evidence(
        &self,
        request: Request<p::MaterializeRequest>,
    ) -> std::result::Result<Response<p::MaterializedEvidence>, Status> {
        KernelService::materialize_evidence(self, request.into_inner())
            .await
            .map(Response::new)
            .map_err(status)
    }
}
