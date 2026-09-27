use crate::*;

impl MemoryService {
    pub async fn ready_status(&self) -> RuntimeStatus {
        self.status().await
    }
}
