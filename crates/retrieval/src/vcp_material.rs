use crate::*;
use nous_persistence::TopologyEdgeSource;

pub struct VcpProjectionMaterial {
    pub authority_watermark: i64,
    pub space: EmbeddingSpaceSignature,
    pub producer: ProducerSignature,
    pub identities: VcpIdentityMap,
    pub edges: Vec<TopologyEdgeSource>,
    pub documents: Vec<VcpProjectedDocument>,
}
pub struct VcpProjectedDocument {
    pub reference: CognitiveRef,
    pub representation_text: String,
    pub vector: Vec<f32>,
    pub concept_refs: Vec<CognitiveRef>,
    pub curve_order: VcpCurveOrder,
}

impl ServingService {
    /// Resolve model material for a disposable VCP serving generation using
    /// the same host-supplied provider contract as dense construction.
    pub async fn vcp_projection_material(
        &self,
        subject: SubjectId,
        snapshot: &nous_configuration::ConfigSnapshot,
    ) -> Result<VcpProjectionMaterial> {
        let provider = self.embedding().ok_or_else(|| {
            Error::Unavailable("VCP serving build requires configured embedding material".into())
        })?;
        let space = provider.space();
        let producer = provider.producer();
        let budget = snapshot.get(crate::EPISODE_SYNOPSIS)?;
        let capabilities = self.projection_capabilities(subject).await?;
        let input = self
            .store
            .cognitive_projection_input(subject, capabilities.memory, budget)
            .await?;
        let mut documents = Vec::new();
        for document in self.documents(input.sources, budget).await? {
            let concept_refs = document
                .tag_ids
                .iter()
                .map(|id| {
                    id.parse()
                        .map(|id| CognitiveRef::Tag(TagId(id)))
                        .map_err(|_| {
                            Error::Infrastructure(
                                "cognitive projection contains an invalid Tag identity".into(),
                            )
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            let output = provider
                .embed(TextEmbeddingRequest {
                    subject,
                    text: document.representation_text.clone(),
                    query: false,
                })
                .await?;
            if !output.space.compatible_with(&space)
                || output.producer.signature_hash != producer.signature_hash
            {
                return Err(Error::Conflict(
                    "VCP embedding material disagrees with generation space/producer".into(),
                ));
            }
            if output.vector.len() != space.dimension as usize
                || output.vector.iter().any(|v| !v.is_finite())
            {
                return Err(Error::Invalid(
                    "VCP embedding material has invalid dimension or values".into(),
                ));
            }
            documents.push(VcpProjectedDocument {
                reference: document.reference,
                representation_text: document.representation_text,
                vector: output.vector,
                concept_refs,
                curve_order: VcpCurveOrder::StableIdentity,
            });
        }
        let identities = VcpIdentityMap::new(
            input
                .topology
                .nodes
                .into_iter()
                .chain(documents.iter().map(|d| d.reference.clone()))
                .chain(
                    documents
                        .iter()
                        .flat_map(|d| d.concept_refs.iter().cloned()),
                ),
        );
        Ok(VcpProjectionMaterial {
            authority_watermark: input.authority_watermark,
            space,
            producer,
            identities,
            edges: input.topology.edges,
            documents,
        })
    }
}
