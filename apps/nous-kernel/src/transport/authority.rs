use super::*;

#[tonic::async_trait]
impl k::authority_service_server::AuthorityService for KernelService {
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
    async fn get_derived_region(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::DerivedRegion>, Status> {
        let input = request.into_inner();
        let result: nous_core::Result<_> = async {
            use sqlx::Row;
            let r = sqlx::query(
                "SELECT * FROM derived_regions WHERE subject_id=$1 AND derived_region_id=$2",
            )
            .bind(id(&input.subject_id)?)
            .bind(id(&input.id)?)
            .fetch_optional(self.0.store.pool())
            .await
            .map_err(nous_persistence::database_error)?
            .ok_or_else(|| Error::NotFound("derived region not found".into()))?;
            Ok(p::DerivedRegion {
                derived_region_id: input.id,
                subject_id: input.subject_id,
                representation_id: r
                    .try_get::<uuid::Uuid, _>("derived_representation_id")
                    .map_err(nous_persistence::database_error)?
                    .to_string(),
                coordinate_kind: r
                    .try_get("coordinate_kind")
                    .map_err(nous_persistence::database_error)?,
                coordinate: to_object(
                    r.try_get("coordinate")
                        .map_err(nous_persistence::database_error)?,
                ),
                coordinate_hash: r
                    .try_get("coordinate_hash")
                    .map_err(nous_persistence::database_error)?,
                parent_derived_region_id: r
                    .try_get::<Option<uuid::Uuid>, _>("parent_derived_region_id")
                    .map_err(nous_persistence::database_error)?
                    .map(|id| id.to_string()),
            })
        }
        .await;
        result.map(Response::new).map_err(status)
    }
    async fn get_producer(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::ProducerSignature>, Status> {
        let input = request.into_inner();
        let result: nous_core::Result<_> = async {
            use sqlx::Row;
            let r = sqlx::query("SELECT p.* FROM producer_signatures p WHERE p.producer_signature_id=$2 AND (EXISTS(SELECT 1 FROM derived_representations d WHERE d.subject_id=$1 AND d.producer_signature_id=p.producer_signature_id) OR EXISTS(SELECT 1 FROM memory_revisions m WHERE m.subject_id=$1 AND m.producer_signature_id=p.producer_signature_id))").bind(id(&input.subject_id)?).bind(id(&input.id)?).fetch_optional(self.0.store.pool()).await.map_err(nous_persistence::database_error)?.ok_or_else(||Error::NotFound("producer is not referenced by Subject".into()))?;
            Ok(p::ProducerSignature { signature_hash: r.try_get("signature_hash").map_err(nous_persistence::database_error)?, provider_class: r.try_get("provider_class").map_err(nous_persistence::database_error)?, operation: r.try_get::<String,_>("operation").map_err(nous_persistence::database_error)?.replace('.',"_"), implementation: r.try_get("implementation").map_err(nous_persistence::database_error)?, model_identity: r.try_get("model_identity").map_err(nous_persistence::database_error)?, model_revision: r.try_get("model_revision").map_err(nous_persistence::database_error)?, output_schema_digest: r.try_get("output_schema_digest").map_err(nous_persistence::database_error)?, preprocessing_identity: r.try_get("preprocessing_identity").map_err(nous_persistence::database_error)?, preprocessing_revision: r.try_get("preprocessing_revision").map_err(nous_persistence::database_error)?, config_digest: r.try_get("config_digest").map_err(nous_persistence::database_error)? })
        }.await;
        result.map(Response::new).map_err(status)
    }
    async fn list_derived_representations(
        &self,
        request: Request<p::RepresentationListRequest>,
    ) -> std::result::Result<Response<p::RepresentationListResponse>, Status> {
        let input = request.into_inner();
        let result: nous_core::Result<_> = async {
            if input.limit == 0 || input.limit > 200 { return Err(Error::Invalid("representation limit must be 1..200".into())); }
            let subject = SubjectId(id(&input.subject_id)?);
            self.0.store.require_subject(subject).await?;
            let source = input.source_region_id.as_deref().map(id).transpose()?;
            let artifact = input.artifact_id.as_deref().map(id).transpose()?;
            let ids: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT d.derived_representation_id FROM derived_representations d WHERE d.subject_id=$1 AND ($2::uuid IS NULL OR EXISTS(SELECT 1 FROM representation_source_regions($1,d.derived_representation_id) roots WHERE roots.source_region_id=$2)) AND ($3::uuid IS NULL OR EXISTS(SELECT 1 FROM representation_source_regions($1,d.derived_representation_id) roots JOIN source_regions s USING(source_region_id) WHERE s.artifact_id=$3)) AND ($4::text IS NULL OR d.representation_kind=$4) AND NOT EXISTS(SELECT 1 FROM derived_representations newer WHERE newer.supersedes=d.derived_representation_id) ORDER BY d.derived_representation_id DESC LIMIT $5")
                .bind(subject.0).bind(source).bind(artifact).bind(input.kind).bind(i64::from(input.limit)+1).fetch_all(self.0.store.pool()).await.map_err(nous_persistence::database_error)?;
            let truncated = ids.len()>input.limit as usize;
            let mut items = Vec::new();
            for value in ids.into_iter().take(input.limit as usize) { items.push(self.get_derived_representation(p::ObjectRequest { subject_id: input.subject_id.clone(), id: value.to_string() }).await?); }
            Ok(p::RepresentationListResponse { items, truncated })
        }.await;
        result.map(Response::new).map_err(status)
    }
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
    async fn create_episode(
        &self,
        request: Request<p::CreateEpisodeRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::create_episode(self, request.into_inner())).await
    }
    async fn get_episode(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::get_episode(self, request.into_inner())).await
    }
    async fn get_episode_revision(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::EpisodeRevision>, Status> {
        rpc_reply(KernelService::get_episode_revision(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn list_episodes(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListEpisodesResponse>, Status> {
        rpc_reply(KernelService::list_episodes(self, request.into_inner())).await
    }
    async fn list_episode_revisions(
        &self,
        request: Request<p::ListEpisodeRevisionsRequest>,
    ) -> std::result::Result<Response<p::ListEpisodeRevisionsResponse>, Status> {
        rpc_reply(KernelService::list_episode_revisions(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn revise_episode(
        &self,
        request: Request<p::ReviseEpisodeRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::revise_episode(self, request.into_inner())).await
    }
    async fn link_episode_revisions(
        &self,
        request: Request<p::LinkEpisodeRevisionsRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::link_episode_revisions(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn suppress_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::suppress_episode(self, request.into_inner())).await
    }
    async fn restore_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::restore_episode(self, request.into_inner())).await
    }
    async fn withdraw_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::withdraw_episode(self, request.into_inner())).await
    }
    async fn reaccept_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<p::EpisodeResponse>, Status> {
        rpc_reply(KernelService::reaccept_episode(self, request.into_inner())).await
    }
    async fn purge_episode(
        &self,
        request: Request<p::EpisodeMutationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::purge_episode(self, request.into_inner())).await
    }
    async fn get_journal(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::JournalResponse>, Status> {
        rpc_reply(KernelService::get_journal(self, request.into_inner())).await
    }
    async fn get_journal_revision(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::JournalRevision>, Status> {
        rpc_reply(KernelService::get_journal_revision(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn list_journals(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListJournalsResponse>, Status> {
        rpc_reply(KernelService::list_journals(self, request.into_inner())).await
    }
    async fn list_journal_revisions(
        &self,
        request: Request<p::ListJournalRevisionsRequest>,
    ) -> std::result::Result<Response<p::ListJournalRevisionsResponse>, Status> {
        rpc_reply(KernelService::list_journal_revisions(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn suppress_journal(
        &self,
        request: Request<p::JournalMutationRequest>,
    ) -> std::result::Result<Response<p::JournalResponse>, Status> {
        rpc_reply(KernelService::suppress_journal(self, request.into_inner())).await
    }
    async fn restore_journal(
        &self,
        request: Request<p::JournalMutationRequest>,
    ) -> std::result::Result<Response<p::JournalResponse>, Status> {
        rpc_reply(KernelService::restore_journal(self, request.into_inner())).await
    }
    async fn withdraw_journal(
        &self,
        request: Request<p::JournalMutationRequest>,
    ) -> std::result::Result<Response<p::JournalResponse>, Status> {
        rpc_reply(KernelService::withdraw_journal(self, request.into_inner())).await
    }
    async fn reaccept_journal(
        &self,
        request: Request<p::JournalMutationRequest>,
    ) -> std::result::Result<Response<p::JournalResponse>, Status> {
        rpc_reply(KernelService::reaccept_journal(self, request.into_inner())).await
    }
    async fn purge_journal(
        &self,
        request: Request<p::JournalMutationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::purge_journal(self, request.into_inner())).await
    }
    async fn set_accessibility(
        &self,
        request: Request<p::SetAccessibilityRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::set_accessibility(self, request.into_inner())).await
    }
    async fn link_revisions(
        &self,
        request: Request<p::LinkRevisionsRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::link_revisions(self, request.into_inner())).await
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
    async fn consolidate_memory(
        &self,
        request: Request<p::ConsolidateMemoryRequest>,
    ) -> std::result::Result<Response<p::ConsolidationResponse>, Status> {
        rpc_reply(KernelService::consolidate_memory(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn get_occurrence(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Occurrence>, Status> {
        rpc_reply(KernelService::get_occurrence(self, request.into_inner())).await
    }
    async fn get_source_region(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::SourceRegion>, Status> {
        rpc_reply(KernelService::get_source_region(self, request.into_inner())).await
    }
    async fn get_derived_representation(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::DerivedRepresentation>, Status> {
        rpc_reply(KernelService::get_derived_representation(
            self,
            request.into_inner(),
        ))
        .await
    }
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
    async fn get_memory(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::get_memory(self, request.into_inner())).await
    }
    async fn list_memories(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListMemoriesResponse>, Status> {
        rpc_reply(KernelService::list_memories(self, request.into_inner())).await
    }
    async fn get_memory_revision(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::get_memory_revision(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn list_memory_revisions(
        &self,
        request: Request<p::MemoryHistoryRequest>,
    ) -> std::result::Result<Response<p::ListMemoriesResponse>, Status> {
        rpc_reply(KernelService::list_memory_revisions(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn form_memory(
        &self,
        request: Request<p::FormMemoryRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::form_memory(self, request.into_inner())).await
    }
    async fn revise_memory(
        &self,
        request: Request<p::ReviseMemoryRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::revise_memory(self, request.into_inner())).await
    }
    async fn suppress_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::suppress_memory(self, request.into_inner())).await
    }
    async fn restore_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::restore_memory(self, request.into_inner())).await
    }
    async fn withdraw_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::withdraw_memory(self, request.into_inner())).await
    }
    async fn reaccept_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<p::Memory>, Status> {
        rpc_reply(KernelService::reaccept_memory(self, request.into_inner())).await
    }
    async fn purge_memory(
        &self,
        request: Request<p::MemoryMutationRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::purge_memory(self, request.into_inner())).await
    }
    async fn get_artifact(
        &self,
        request: Request<p::ObjectRequest>,
    ) -> std::result::Result<Response<p::Artifact>, Status> {
        rpc_reply(KernelService::get_artifact(self, request.into_inner())).await
    }
    async fn list_artifacts(
        &self,
        request: Request<p::ListRequest>,
    ) -> std::result::Result<Response<p::ListArtifactsResponse>, Status> {
        rpc_reply(KernelService::list_artifacts(self, request.into_inner())).await
    }
    async fn materialize_evidence(
        &self,
        request: Request<p::MaterializeRequest>,
    ) -> std::result::Result<Response<p::MaterializedEvidence>, Status> {
        rpc_reply(KernelService::materialize_evidence(
            self,
            request.into_inner(),
        ))
        .await
    }
}
