//! Tag identity interpretation shared by binding and Authority projections.
use crate::*;
use nous_core::{CognitiveRef, TagId};

impl AuthorityStore {
    pub async fn canonical_tag_id(&self, subject: SubjectId, tag: TagId) -> Result<TagId> {
        sqlx::query_scalar::<_, Option<uuid::Uuid>>("SELECT canonical_tag($1,$2)")
            .bind(subject.0)
            .bind(tag.0)
            .fetch_one(self.pool())
            .await
            .map_err(database_error)?
            .map(TagId)
            .ok_or_else(|| Error::NotFound("active Tag identity not found".into()))
    }
}
pub(crate) async fn canonical_topology_ref_in(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    reference: CognitiveRef,
) -> Result<Option<CognitiveRef>> {
    let CognitiveRef::Tag(tag) = reference else {
        return Ok(Some(reference));
    };
    Ok(
        sqlx::query_scalar::<_, Option<uuid::Uuid>>("SELECT canonical_tag($1,$2)")
            .bind(subject.0)
            .bind(tag.0)
            .fetch_one(&mut **tx)
            .await
            .map_err(database_error)?
            .map(|id| CognitiveRef::Tag(TagId(id))),
    )
}
