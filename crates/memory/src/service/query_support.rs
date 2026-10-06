// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::*;
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) async fn resolve_memory_references(
    service: &MemoryService,
    subject: SubjectId,
    references: &[CognitiveRef],
) -> Result<HashMap<CognitiveRef, (MemoryId, MemoryRevisionId)>> {
    let memory_ids = references
        .iter()
        .filter_map(|reference| match reference {
            CognitiveRef::Memory(value) => Some(value.0),
            _ => None,
        })
        .collect::<Vec<_>>();
    let revision_ids = references
        .iter()
        .filter_map(|reference| match reference {
            CognitiveRef::MemoryRevision(value) => Some(value.0),
            _ => None,
        })
        .collect::<Vec<_>>();
    if memory_ids.is_empty() && revision_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query("SELECT memory_id,current_revision_id,memory_revision_id,source_kind,source_id FROM (SELECT memory_id,current_revision_id AS current_revision_id,current_revision_id AS memory_revision_id,'memory'::text AS source_kind,memory_id AS source_id FROM memory_objects WHERE subject_id=$1 AND memory_id=ANY($2::uuid[]) UNION ALL SELECT memory_id,memory_revision_id,memory_revision_id,'memory_revision'::text,memory_revision_id FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=ANY($3::uuid[])) resolved")
        .bind(subject.0)
        .bind(&memory_ids)
        .bind(&revision_ids)
        .fetch_all(service.store.pool())
        .await
        .map_err(nous_persistence::database_error)?;
    let mut result = HashMap::new();
    for row in rows {
        let memory = MemoryId(
            row.try_get("memory_id")
                .map_err(nous_persistence::database_error)?,
        );
        let revision = MemoryRevisionId(
            row.try_get("memory_revision_id")
                .map_err(nous_persistence::database_error)?,
        );
        let source_kind: String = row
            .try_get("source_kind")
            .map_err(nous_persistence::database_error)?;
        let source_id: Uuid = row
            .try_get("source_id")
            .map_err(nous_persistence::database_error)?;
        let reference = if source_kind == "memory" {
            CognitiveRef::Memory(MemoryId(source_id))
        } else {
            CognitiveRef::MemoryRevision(MemoryRevisionId(source_id))
        };
        result.insert(reference, (memory, revision));
    }
    Ok(result)
}
