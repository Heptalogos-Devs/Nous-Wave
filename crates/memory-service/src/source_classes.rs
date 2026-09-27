use crate::*;
use nous_authority_store::database_error as db;

impl MemoryService {
    pub(crate) async fn source_classes(
        &self,
        revision: MemoryRevisionId,
    ) -> Result<Vec<SourceClass>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT o.source_class FROM memory_revision_evidence e JOIN observation_occurrences o USING(occurrence_id) WHERE e.memory_revision_id=$1 ORDER BY o.source_class",
        )
        .bind(revision.0)
        .fetch_all(self.store.pool())
        .await
        .map_err(db)?
        .into_iter()
        .map(SourceClass::from)
        .collect())
    }
}
