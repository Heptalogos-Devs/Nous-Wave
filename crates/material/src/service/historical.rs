//! Historical Material visibility and effective derivation selection.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
use super::*;
use nous_persistence::database_error as db;
use sqlx::Row;
impl MaterialService {
    pub async fn project_as_of(
        &self,
        subject: SubjectId,
        as_of: DateTime<Utc>,
    ) -> Result<HistoricalMaterialProjection> {
        self.store.require_subject(subject).await?;
        let rows = self.store.historical_owner_rows(subject, as_of).await?;
        let active:Vec<Uuid>=sqlx::query_scalar("SELECT d.derived_representation_id FROM derived_representations d WHERE d.subject_id=$1 AND d.created_at<=$2 AND NOT EXISTS(SELECT 1 FROM derived_representations newer WHERE newer.subject_id=d.subject_id AND newer.supersedes=d.derived_representation_id AND newer.created_at<=$2) ORDER BY d.derived_representation_id").bind(subject.0).bind(as_of).fetch_all(self.store.pool()).await.map_err(db)?;
        let regions=sqlx::query("SELECT derived_region_id,derived_representation_id FROM derived_regions WHERE subject_id=$1 AND created_at<=$2").bind(subject.0).bind(as_of).fetch_all(self.store.pool()).await.map_err(db)?;
        let active_regions = regions
            .into_iter()
            .map(|row| {
                Ok((
                    row.try_get::<Uuid, _>("derived_region_id").map_err(db)?,
                    row.try_get::<Uuid, _>("derived_representation_id")
                        .map_err(db)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|(region, parent)| active.contains(&parent).then_some(region))
            .collect::<std::collections::HashSet<_>>();
        let document_references = rows
            .material
            .iter()
            .filter(|reference| match reference {
                CognitiveRef::DerivedRepresentation(id) => active.contains(&id.0),
                CognitiveRef::DerivedRegion(id) => active_regions.contains(&id.0),
                _ => true,
            })
            .cloned()
            .collect();
        Ok(HistoricalMaterialProjection {
            known_references: rows.material,
            document_references,
            entity_bindings: rows.entity_bindings,
        })
    }
}
