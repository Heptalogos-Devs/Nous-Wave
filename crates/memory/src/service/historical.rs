//! Memory owns historical cognition and concept semantics over canonical chronology.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
use super::*;
impl MemoryService {
    pub async fn project_as_of(
        &self,
        subject: SubjectId,
        as_of: DateTime<Utc>,
        revision_view: RevisionView,
    ) -> Result<HistoricalAuthoritySnapshot> {
        self.store.require_subject(subject).await?;
        let rows = self.store.historical_owner_rows(subject, as_of).await?;
        let mut cognition = Vec::new();
        for mut row in rows.cognition {
            let object = parse_reference(&row.object_kind, &row.object_id.to_string())?;
            let kind = format!("{}_revision", row.object_kind);
            let mut revisions = row
                .revisions
                .into_iter()
                .map(|id| parse_reference(&kind, &id.to_string()))
                .collect::<Result<Vec<_>>>()?;
            let head = revisions.first().cloned().ok_or_else(|| {
                Error::Infrastructure("historical cognition missing effective revision".into())
            })?;
            row.state["current_revision_id"] = serde_json::json!(reference_parts(&head).1);
            let recorded_revisions = revisions.clone();
            if revision_view == RevisionView::Current {
                revisions.truncate(1);
            }
            cognition.push(HistoricalCognitionState {
                object,
                head,
                revisions,
                recorded_revisions,
                state: row.state,
            });
        }
        let tags = rows
            .tags
            .into_iter()
            .map(|row| {
                let canonical_tag = row.state["canonical_tag_id"]
                    .as_str()
                    .map(str::parse::<Uuid>)
                    .transpose()
                    .map_err(|_| Error::Infrastructure("invalid canonical Tag state".into()))?
                    .map(TagId);
                Ok(HistoricalTagState {
                    tag: row.tag_id,
                    revision_id: row.revision_id,
                    canonical_tag,
                    status: row.state["status"]
                        .as_str()
                        .ok_or_else(|| Error::Infrastructure("Tag state missing status".into()))?
                        .into(),
                    semantic: tag_semantic_representation(
                        &row.label,
                        row.description.as_deref(),
                        row.kind_hint.as_deref(),
                    )?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut snapshot = HistoricalAuthoritySnapshot {
            subject,
            as_of,
            revision_view,
            cognition,
            tags,
            associations: rows.associations,
            schema_evidence_links: rows.schema_evidence_links,
            entity_bindings: rows.entity_bindings,
            material_visibility: rows.material,
            lexical_visibility: rows.lexical,
            snapshot_digest: String::new(),
        };
        let identities=snapshot.cognition.iter().map(|state|serde_json::json!({"object":state.object,"head":state.head,"revisions":state.revisions,"acceptance":state.state["acceptance_state"],"integrity":state.state["integrity_state"],"suppression":state.state["suppression_state"],"purge":state.state["purge_state"],"accessibility":state.state["accessibility_mode"]})).collect::<Vec<_>>();
        let canonical = serde_json::json!({"subject":subject,"revision_view":revision_view,"cognition":identities,"tags":snapshot.tags,"associations":snapshot.associations,"schema_evidence_links":snapshot.schema_evidence_links,"entity_bindings":snapshot.entity_bindings,"material":snapshot.material_visibility,"lexical":snapshot.lexical_visibility});
        snapshot.snapshot_digest =
            operation_digest("historical_authority_snapshot_v1", subject, &canonical)?;
        Ok(snapshot)
    }
}
