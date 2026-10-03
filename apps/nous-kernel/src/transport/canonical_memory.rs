use super::*;

#[tonic::async_trait]
impl p::memory_service_server::MemoryService for KernelService {
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
}
