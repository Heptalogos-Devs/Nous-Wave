//! Rebuildable historical read descriptors; canonical Authority remains with semantic owners.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0
use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalCognitionState {
    pub object: CognitiveRef,
    pub head: CognitiveRef,
    pub revisions: Vec<CognitiveRef>,
    pub recorded_revisions: Vec<CognitiveRef>,
    pub state: serde_json::Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalTagState {
    pub tag: TagId,
    pub revision_id: Uuid,
    pub canonical_tag: Option<TagId>,
    pub status: String,
    pub semantic: TagSemanticRepresentation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalLexicalVisibility {
    pub lexical_ref: String,
    pub object_kind: String,
    pub canonical_ref: String,
    pub display_name: String,
    pub aliases: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalAuthoritySnapshot {
    pub subject: SubjectId,
    pub as_of: chrono::DateTime<chrono::Utc>,
    pub revision_view: RevisionView,
    pub cognition: Vec<HistoricalCognitionState>,
    pub tags: Vec<HistoricalTagState>,
    pub associations: Vec<AssociationEvidenceId>,
    pub schema_evidence_links: Vec<Uuid>,
    pub entity_bindings: Vec<Uuid>,
    pub material_visibility: Vec<CognitiveRef>,
    pub material_documents: Vec<CognitiveRef>,
    pub lexical_visibility: Vec<HistoricalLexicalVisibility>,
    pub snapshot_digest: String,
}
impl HistoricalAuthoritySnapshot {
    pub fn refresh_digest(&mut self) -> Result<()> {
        let identities=self.cognition.iter().map(|state|serde_json::json!({"object":state.object,"head":state.head,"revisions":state.revisions,"acceptance":state.state["acceptance_state"],"integrity":state.state["integrity_state"],"suppression":state.state["suppression_state"],"purge":state.state["purge_state"],"accessibility":state.state["accessibility_mode"]})).collect::<Vec<_>>();
        let value = serde_json::json!({"subject":self.subject,"revision_view":self.revision_view,"cognition":identities,"tags":self.tags,"associations":self.associations,"schema_evidence_links":self.schema_evidence_links,"entity_bindings":self.entity_bindings,"material":self.material_visibility,"material_documents":self.material_documents,"lexical":self.lexical_visibility});
        self.snapshot_digest = blake3::hash(
            &serde_json::to_vec(&value)
                .map_err(|error| Error::Infrastructure(error.to_string()))?,
        )
        .to_hex()
        .to_string();
        Ok(())
    }
    pub fn canonical_tag(&self, requested: TagId) -> Option<TagId> {
        let mut current = requested;
        for _ in 0..128 {
            let tag = self.tags.iter().find(|tag| tag.tag == current)?;
            match tag.status.as_str() {
                "active" => return Some(current),
                "merged" => current = tag.canonical_tag?,
                _ => return None,
            }
        }
        None
    }
    pub fn cognition_for(&self, reference: &CognitiveRef) -> Option<&HistoricalCognitionState> {
        self.cognition.iter().find(|state| {
            &state.object == reference || state.recorded_revisions.contains(reference)
        })
    }
    pub fn contains(&self, reference: &CognitiveRef) -> bool {
        self.cognition_for(reference).is_some()
            || self.material_visibility.contains(reference)
            || matches!(reference,CognitiveRef::Tag(tag) if self.canonical_tag(*tag).is_some())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMaterialProjection {
    pub known_references: Vec<CognitiveRef>,
    pub document_references: Vec<CognitiveRef>,
    pub entity_bindings: Vec<Uuid>,
}
