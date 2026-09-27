use super::*;
use nous_authority_store::database_error as db;
use nous_core::{OperationId, Result, SubjectId};
use nous_memory_domain::{AssociationSupport, UseEventRef};
use nous_memory_service::{CreateAssociationRequest, CreateTagRequest};
use sqlx::Row;

impl KernelService {
    pub(super) async fn create_tag(&self, input: p::CreateTagRequest) -> Result<p::Tag> {
        let subject = SubjectId(id(&input.subject_id)?);
        let tag = required(input.tag, "tag")?;
        let created = self
            .require_memory()?
            .create_tag(
                subject,
                CreateTagRequest {
                    operation_id: OperationId(id(&input.operation_id)?),
                    label: tag.label,
                    description: tag.description,
                    kind_hint: tag.kind_hint,
                    origin: tag.origin,
                },
            )
            .await?;
        let revision = sqlx::query(
            "SELECT label,description,kind_hint,origin FROM tag_revisions WHERE tag_revision_id=$1",
        )
        .bind(created.current_revision_id)
        .fetch_one(self.0.store.pool())
        .await
        .map_err(db)?;
        Ok(p::Tag {
            tag_id: created.tag_id.0.to_string(),
            label: revision.try_get("label").map_err(db)?,
            description: revision.try_get("description").map_err(db)?,
            kind_hint: revision.try_get("kind_hint").map_err(db)?,
            origin: revision.try_get("origin").map_err(db)?,
        })
    }
    pub(super) async fn get_tag(&self, input: p::ObjectRequest) -> Result<p::Tag> {
        let row=sqlx::query("SELECT t.tag_id,r.label,r.description,r.kind_hint,r.origin FROM tags t JOIN tag_revisions r ON r.tag_revision_id=t.current_revision_id WHERE t.subject_id=$1 AND t.tag_id=$2").bind(id(&input.subject_id)?).bind(id(&input.id)?).fetch_optional(self.0.store.pool()).await.map_err(db)?.ok_or_else(||Error::NotFound("tag not found".into()))?;
        Ok(p::Tag {
            tag_id: row
                .try_get::<uuid::Uuid, _>("tag_id")
                .map_err(db)?
                .to_string(),
            label: row.try_get("label").map_err(db)?,
            description: row.try_get("description").map_err(db)?,
            kind_hint: row.try_get("kind_hint").map_err(db)?,
            origin: row.try_get("origin").map_err(db)?,
        })
    }
    pub(super) async fn list_tags(&self, input: p::ListRequest) -> Result<p::ListTagsResponse> {
        let rows=sqlx::query("SELECT t.tag_id,r.label,r.description,r.kind_hint,r.origin FROM tags t JOIN tag_revisions r ON r.tag_revision_id=t.current_revision_id WHERE t.subject_id=$1 ORDER BY t.tag_id LIMIT 257").bind(id(&input.subject_id)?).fetch_all(self.0.store.pool()).await.map_err(db)?;
        let mut items = Vec::new();
        for row in rows.into_iter().take(256) {
            items.push(p::Tag {
                tag_id: row
                    .try_get::<uuid::Uuid, _>("tag_id")
                    .map_err(db)?
                    .to_string(),
                label: row.try_get("label").map_err(db)?,
                description: row.try_get("description").map_err(db)?,
                kind_hint: row.try_get("kind_hint").map_err(db)?,
                origin: row.try_get("origin").map_err(db)?,
            });
        }
        Ok(p::ListTagsResponse {
            items,
            next_page_token: String::new(),
        })
    }
    pub(super) async fn create_association(
        &self,
        input: p::CreateAssociationRequest,
    ) -> Result<p::Association> {
        let subject = SubjectId(id(&input.subject_id)?);
        let association = required(input.association, "association")?;
        let from = from_ref(required(association.from, "association.from")?)?;
        let to = from_ref(required(association.to, "association.to")?)?;
        let supports = association
            .supports
            .into_iter()
            .map(association_support)
            .collect::<Result<Vec<AssociationSupport>>>()?;
        let result = self
            .require_memory()?
            .create_association(
                CreateAssociationRequest {
                    operation_id: OperationId(id(&input.operation_id)?),
                    from: from.clone(),
                    to: to.clone(),
                    relation_kind: association.relation_kind.clone(),
                    polarity: enum_value(&association.polarity)?,
                    support_class: enum_value(&association.support_class)?,
                    supports,
                    producer_signature_id: association
                        .producer_signature_id
                        .as_deref()
                        .map(id)
                        .transpose()?,
                    valid_time: nous_core::TemporalExtent::Unknown,
                },
                subject,
            )
            .await?;
        Ok(p::Association {
            association_id: result.association_evidence_id.0.to_string(),
            from: Some(to_ref(result.from)),
            to: Some(to_ref(result.to)),
            relation_kind: result.relation_kind,
            polarity: enum_name(result.polarity),
            support_class: enum_name(result.support_class),
            supports: result
                .supports
                .into_iter()
                .map(association_support_proto)
                .collect(),
            producer_signature_id: result.producer_signature_id.map(|value| value.to_string()),
        })
    }

    pub(super) async fn revoke_association(
        &self,
        input: p::RevokeAssociationRequest,
    ) -> Result<()> {
        self.require_memory()?
            .revoke_association(
                SubjectId(id(&input.subject_id)?),
                nous_core::AssociationEvidenceId(id(&input.association_id)?),
                OperationId(id(&input.operation_id)?),
            )
            .await
    }
    pub(super) async fn get_neighborhood(
        &self,
        _input: p::NeighborhoodRequest,
    ) -> Result<p::NeighborhoodResponse> {
        Ok(p::NeighborhoodResponse {
            nodes: Vec::new(),
            associations: Vec::new(),
            truncated: false,
        })
    }
    pub(super) async fn rebind_entity(&self, input: p::RebindEntityRequest) -> Result<()> {
        let subject = id(&input.subject_id)?;
        let mention = id(&input.mention_id)?;
        let revision:i32=sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM entity_binding_revisions WHERE mention_id=$1").bind(mention).fetch_one(self.0.store.pool()).await.map_err(db)?;
        sqlx::query("INSERT INTO entity_binding_revisions(binding_revision_id,mention_id,revision_no,entity_ref,binding_state,host_resolution_ref,reason,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,now())").bind(uuid::Uuid::now_v7()).bind(mention).bind(revision).bind(input.entity_ref).bind(input.binding_state).bind(input.host_resolution_ref).bind(input.reason).execute(self.0.store.pool()).await.map_err(db)?;
        let _ = subject;
        Ok(())
    }
}

fn association_support(value: p::AssociationSupport) -> Result<AssociationSupport> {
    match value
        .support
        .ok_or_else(|| Error::Invalid("empty association support".into()))?
    {
        p::association_support::Support::Revision(value) => {
            Ok(AssociationSupport::Revision(super::support(value)?))
        }
        p::association_support::Support::UseEvent(value) => {
            Ok(AssociationSupport::UseEvent(UseEventRef {
                subject_id: SubjectId(id(&value.subject_id)?),
                consumer_ref: value.consumer_ref,
                event_id: nous_core::UseEventId(id(&value.event_id)?),
            }))
        }
    }
}

fn association_support_proto(value: AssociationSupport) -> p::AssociationSupport {
    let support = match value {
        AssociationSupport::Revision(value) => {
            p::association_support::Support::Revision(super::support_proto(value))
        }
        AssociationSupport::UseEvent(value) => {
            p::association_support::Support::UseEvent(p::UseEventRef {
                subject_id: value.subject_id.0.to_string(),
                consumer_ref: value.consumer_ref,
                event_id: value.event_id.0.to_string(),
            })
        }
    };
    p::AssociationSupport {
        support: Some(support),
    }
}
