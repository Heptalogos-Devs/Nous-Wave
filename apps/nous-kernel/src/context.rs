use crate::NousRuntime;
use nous_cognitive_runtime::{ContextResolver, ContextSource, bounded_text, modality};
use nous_core::*;
use nous_material_service::MaterializeRequest;

#[async_trait::async_trait]
impl ContextResolver for NousRuntime {
    #[expect(
        clippy::unnecessary_filter_map,
        reason = "context evidence selection preserves owner-specific support variants"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "context resolution keeps Memory, Resource, Self, and material ownership branches explicit"
    )]
    async fn context_source(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
        max_bytes: usize,
        explicit: bool,
    ) -> Result<ContextSource> {
        let memory = match reference {
            CognitiveRef::Memory(id) => {
                Some(self.require_memory()?.memory(subject, *id, None).await?)
            }
            CognitiveRef::MemoryRevision(id) => {
                Some(self.require_memory()?.revision(subject, *id).await?)
            }
            _ => None,
        };
        if let Some(memory) = memory {
            if !matches!(
                memory.object.acceptance_state,
                nous_memory_domain::AcceptanceState::Accepted
            ) || !matches!(
                memory.object.integrity_state,
                nous_memory_domain::IntegrityState::Valid
            ) || !matches!(
                memory.object.suppression_state,
                nous_memory_domain::SuppressionState::Normal
            ) || !matches!(
                memory.object.purge_state,
                nous_memory_domain::PurgeState::Normal
            ) {
                return Err(Error::Unavailable("Memory is suppressed".into()));
            }
            let level = self
                .require_memory()?
                .accessibility_level(subject, memory.object.memory_id, chrono::Utc::now())
                .await?;
            if !nous_memory_service::accessibility_eligible(
                level,
                CognitiveEffort::Normal,
                explicit,
            ) {
                return Err(Error::Unavailable(
                    "Memory accessibility requires deeper or explicit recall".into(),
                ));
            }

            let evidence = memory
                .supports
                .iter()
                .filter_map(|item| match item {
                    nous_memory_domain::RevisionSupport::Evidence(value) => Some(EvidenceHandle {
                        reference: value.cognitive_ref(),
                        support_role: value.support_role.as_str().into(),
                    }),
                    nous_memory_domain::RevisionSupport::CognitionDependency(value) => {
                        Some(EvidenceHandle {
                            reference: value.target_revision.clone(),
                            support_role: value.support_role.as_str().into(),
                        })
                    }
                    nous_memory_domain::RevisionSupport::Seed(value) => Some(EvidenceHandle {
                        reference: CognitiveRef::CognitiveSeedVersion(value.seed_version_id),
                        support_role: "seed".into(),
                    }),
                })
                .collect::<Vec<_>>();
            return Ok(ContextSource {
                source_revision: Some(memory.revision.memory_revision_id),
                media_type: "text/plain".into(),
                text: (max_bytes > 0)
                    .then(|| bounded_text(memory.revision.representation_text, max_bytes)),
                authority: AuthorityClass::SubjectCognition,
                provenance: ProvenanceSummary {
                    source_count: evidence.len(),
                    producer_signatures: Vec::new(),
                    note: None,
                },
                evidence,
            });
        }
        if let CognitiveRef::Resource(resource) = reference {
            let descriptor = self
                .cognition
                .list_resources(subject)
                .await?
                .into_iter()
                .find(|view| view.descriptor.resource_ref == *resource)
                .ok_or_else(|| Error::NotFound("Resource not found".into()))?
                .descriptor;
            return Ok(ContextSource {
                source_revision: None,
                media_type: "text/plain".into(),
                text: (max_bytes > 0).then(|| {
                    bounded_text(
                        descriptor
                            .display_label
                            .unwrap_or_else(|| resource.as_str().to_owned()),
                        max_bytes,
                    )
                }),
                authority: AuthorityClass::ResourceDescriptor,
                evidence: Vec::new(),
                provenance: ProvenanceSummary {
                    source_count: 0,
                    producer_signatures: Vec::new(),
                    note: Some("resource awareness".into()),
                },
            });
        }
        let material = self
            .material
            .materialize(
                subject,
                MaterializeRequest {
                    reference: reference.clone(),
                    byte_range: None,
                    max_bytes: (max_bytes.max(1) as u64).min(self.material.max_upload_bytes),
                    resource_handle: None,
                    resource: None,
                },
            )
            .await?;
        material_context(material, max_bytes)
    }
}

fn material_context(
    material: nous_material_service::MaterializedEvidence,
    max_bytes: usize,
) -> Result<ContextSource> {
    let text = if max_bytes > 0
        && matches!(
            modality(&material.media_type),
            Modality::Text | Modality::Structured
        ) {
        let end = match std::str::from_utf8(&material.bytes) {
            Ok(_) => material.bytes.len(),
            Err(error) => error.valid_up_to(),
        };
        Some(
            String::from_utf8(material.bytes[..end].to_vec())
                .map_err(|e| Error::Infrastructure(e.to_string()))?,
        )
    } else {
        None
    };
    let evidence = material
        .provenance
        .into_iter()
        .map(|reference| EvidenceHandle {
            reference,
            support_role: "source".into(),
        })
        .collect::<Vec<_>>();
    Ok(ContextSource {
        source_revision: None,
        media_type: material.media_type,
        text,
        authority: if material.producer.is_some() {
            AuthorityClass::Interpretation
        } else {
            AuthorityClass::Evidence
        },
        provenance: ProvenanceSummary {
            source_count: evidence.len(),
            producer_signatures: material
                .producer
                .and_then(|p| {
                    p.get("signature_hash")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned)
                })
                .into_iter()
                .collect(),
            note: None,
        },
        evidence,
    })
}
