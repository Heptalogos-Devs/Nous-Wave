use super::*;

#[tonic::async_trait]
impl k::kernel_maintenance_service_server::KernelMaintenanceService for KernelService {
    async fn refresh_maintenance(
        &self,
        request: Request<k::RefreshMaintenanceRequest>,
    ) -> std::result::Result<Response<()>, Status> {
        rpc_reply(KernelService::refresh_maintenance(
            self,
            request.into_inner(),
        ))
        .await
    }
    async fn commit_topology(
        &self,
        request: Request<k::CommitTopologyRequest>,
    ) -> std::result::Result<Response<k::CommitTopologyResponse>, Status> {
        rpc_reply(KernelService::commit_topology(self, request.into_inner())).await
    }
    async fn plan_maintenance(
        &self,
        request: Request<k::PlanMaintenanceRequest>,
    ) -> std::result::Result<Response<k::MaintenancePlan>, Status> {
        rpc_reply(KernelService::plan_maintenance(self, request.into_inner())).await
    }
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
}
