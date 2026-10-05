use super::*;
use nous_core::{OperationId, Result, SubjectId};
use nous_memory::{AssociationSupport, UseEventRef};
use nous_memory::{CreateAssociationRequest, CreateTagRequest};
use nous_persistence::database_error as db;
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
        self.read_tags(&input.subject_id, "", input.page, &input.status)
            .await
    }
    pub(super) async fn search_tags(
        &self,
        input: p::SearchTagsRequest,
    ) -> Result<p::ListTagsResponse> {
        if input.text.trim().is_empty() || input.text.chars().count() > 256 {
            return Err(Error::Invalid(
                "tag search requires 1..256 characters".into(),
            ));
        }
        self.read_tags(&input.subject_id, &input.text, input.page, "active")
            .await
    }
    async fn read_tags(
        &self,
        subject: &str,
        text: &str,
        request_page: Option<p::Page>,
        status: &str,
    ) -> Result<p::ListTagsResponse> {
        let subject = SubjectId(id(subject)?);
        self.0.store.require_subject(subject).await?;
        let status = if status.is_empty() { "active" } else { status };
        if !matches!(status, "active" | "withdrawn") {
            return Err(Error::Invalid("invalid tag status".into()));
        }
        let scope = format!("tags:{}:{status}:{text}", subject.0);
        let (limit, last) = page(request_page, &scope)?;
        let rows = sqlx::query("SELECT t.tag_id,r.label,r.description,r.kind_hint,r.origin FROM tags t JOIN tag_revisions r ON r.tag_revision_id=t.current_revision_id WHERE t.subject_id=$1 AND t.status=$2 AND ($3::uuid IS NULL OR t.tag_id>$3) AND ($4='' OR strpos(lower(r.label || ' ' || COALESCE(r.description,'')),lower($4))>0 OR EXISTS(SELECT 1 FROM lexical_bindings b JOIN lexical_visibility v ON v.lexical_ref=b.lexical_ref WHERE v.subject_id=t.subject_id AND b.object_kind='tag' AND b.canonical_ref=t.tag_id::text AND b.tombstoned_at IS NULL AND (strpos(lower(v.display_name),lower($4))>0 OR EXISTS(SELECT 1 FROM unnest(v.aliases) alias WHERE strpos(lower(alias),lower($4))>0)))) ORDER BY t.tag_id LIMIT $5")
            .bind(subject.0).bind(status).bind(last).bind(text).bind(limit+1).fetch_all(self.0.store.pool()).await.map_err(db)?;
        let truncated = rows.len() > usize::try_from(limit).unwrap_or(200);
        let mut items = Vec::new();
        for row in rows.into_iter().take(usize::try_from(limit).unwrap_or(200)) {
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
        let next_page_token = if truncated {
            next_token(
                &scope,
                id(&items
                    .last()
                    .ok_or_else(|| Error::Infrastructure("empty tag page".into()))?
                    .tag_id)?,
            )
        } else {
            String::new()
        };
        Ok(p::ListTagsResponse {
            items,
            next_page_token,
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
        input: p::NeighborhoodRequest,
    ) -> Result<p::NeighborhoodResponse> {
        let result = self
            .require_memory()?
            .association_neighborhood(
                SubjectId(id(&input.subject_id)?),
                from_ref(required(input.root, "root")?)?,
                if input.max_nodes == 0 {
                    64
                } else {
                    input.max_nodes as usize
                },
                if input.max_depth == 0 {
                    1
                } else {
                    input.max_depth as usize
                },
            )
            .await?;
        Ok(p::NeighborhoodResponse {
            nodes: result.nodes.into_iter().map(to_ref).collect(),
            associations: result
                .associations
                .into_iter()
                .map(|a| p::Association {
                    association_id: a.association_evidence_id.0.to_string(),
                    from: Some(to_ref(a.from)),
                    to: Some(to_ref(a.to)),
                    relation_kind: a.relation_kind,
                    polarity: enum_name(a.polarity),
                    support_class: enum_name(a.support_class),
                    supports: a
                        .supports
                        .into_iter()
                        .map(association_support_proto)
                        .collect(),
                    producer_signature_id: a.producer_signature_id.map(|id| id.to_string()),
                })
                .collect(),
            truncated: result.truncated,
        })
    }
    pub(super) async fn rebind_entity(&self, input: p::RebindEntityRequest) -> Result<()> {
        let subject = id(&input.subject_id)?;
        let mention = id(&input.mention_id)?;
        let revision:i32=sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM entity_binding_revisions WHERE mention_id=$1").bind(mention).fetch_one(self.0.store.pool()).await.map_err(db)?;
        sqlx::query("INSERT INTO entity_binding_revisions(binding_revision_id,mention_id,revision_no,entity_ref,binding_state,host_resolution_ref,reason,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(uuid::Uuid::now_v7()).bind(mention).bind(revision).bind(input.entity_ref).bind(input.binding_state).bind(input.host_resolution_ref).bind(input.reason).bind(self.0.cognition.now(SubjectId(subject))).execute(self.0.store.pool()).await.map_err(db)?;
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
