//! Generation graph assembly from coherent Nous projection material.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use crate::reference::{
    ReferenceFileTags, ReferenceGraphConfig, ReferenceGraphInput, ReferenceGraphOutput,
    ReferenceProvenanceEdge, reference_graph_file_facts, reference_graph_from_facts,
};
use crate::{
    VcpCurveOrder, VcpEvidenceContribution, VcpIdentityMap, VcpProjectedDocument,
    VcpProjectionMaterial, vcp_evidence_contributions,
};
use nous_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct VcpGraphAssets {
    pub graph: ReferenceGraphOutput,
    pub membership: Vec<ReferenceFileTags>,
    /// Root IDs have their own namespace. They are evidence identities, not
    /// candidate IDs or an Authority visibility filter.
    pub provenance_roots: Vec<String>,
    pub evidence: Vec<VcpEvidenceContribution>,
}

pub fn vcp_graph_assets(
    material: &VcpProjectionMaterial,
    pairwise: &[(i64, i64, f64)],
    anchor_gain: &[(i64, f64)],
    config: &ReferenceGraphConfig,
) -> Result<VcpGraphAssets> {
    let ids = &material.identities;
    let evidence = vcp_evidence_contributions(ids, &material.edges)?;
    let mut facts = BTreeMap::<(i64, i64), f64>::new();
    let mut roots = BTreeMap::<(i64, i64), BTreeMap<String, f64>>::new();
    let membership = vcp_membership(ids, &material.documents)?;
    let output = reference_graph_file_facts(&ReferenceGraphInput {
        files: membership.clone(),
        pairwise: pairwise.to_vec(),
        anchor_gain: anchor_gain.to_vec(),
        config: config.clone(),
    })?;
    for file in output {
        let root = ids.reference(file.file_id)?.to_string();
        for (from, to, mass) in file.facts {
            *facts.entry((from, to)).or_default() += mass;
            *roots
                .entry((from, to))
                .or_default()
                .entry(root.clone())
                .or_default() += mass;
        }
    }
    for item in &evidence {
        let key = (item.source_id, item.target_id);
        *facts.entry(key).or_default() += item.support_mass;
        *roots
            .entry(key)
            .or_default()
            .entry(item.provenance_root.clone())
            .or_default() += item.support_mass;
    }
    let provenance_roots = roots
        .values()
        .flat_map(|r| r.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let root_ids = provenance_roots
        .iter()
        .enumerate()
        .map(|(i, root)| (root.clone(), i as i64 + 1))
        .collect::<BTreeMap<_, _>>();
    let provenance = roots
        .into_iter()
        .map(|((source_id, target_id), values)| ReferenceProvenanceEdge {
            source_id,
            target_id,
            file_contributions: values
                .into_iter()
                .map(|(root, mass)| (root_ids[&root], mass))
                .collect(),
        })
        .collect();
    let facts = facts
        .into_iter()
        .map(|((from, to), mass)| (from, to, mass))
        .collect::<Vec<_>>();
    let graph = reference_graph_from_facts(&facts, anchor_gain, provenance, config)?;
    Ok(VcpGraphAssets {
        graph,
        membership,
        provenance_roots,
        evidence,
    })
}

pub(crate) fn vcp_membership(
    ids: &VcpIdentityMap,
    documents: &[VcpProjectedDocument],
) -> Result<Vec<ReferenceFileTags>> {
    let mut membership = Vec::new();
    for document in documents {
        let file_id = ids.id(&document.reference)?;
        let mut seen = BTreeSet::new();
        let mut concepts = document
            .concept_refs
            .iter()
            .map(|r| ids.id(r))
            .collect::<Result<Vec<_>>>()?;
        if matches!(document.curve_order, VcpCurveOrder::StableIdentity) {
            concepts.sort_unstable();
        }
        concepts.retain(|id| seen.insert(*id));
        let file = ReferenceFileTags {
            file_id,
            tags: concepts
                .into_iter()
                .enumerate()
                .map(|(i, id)| {
                    (
                        id,
                        match document.curve_order {
                            VcpCurveOrder::SourceSequence => i as i64 + 1,
                            VcpCurveOrder::StableIdentity => 0,
                        },
                    )
                })
                .collect(),
        };
        membership.push(file);
    }
    Ok(membership)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VcpIdentityMap, VcpProjectedDocument};
    use nous_core::{CognitiveRef, MemoryRevisionId, TagId};
    use nous_persistence::TopologyEdgeSource;

    #[test]
    fn unordered_membership_and_independent_evidence_preserve_mass_and_roots() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/vcp-graph.json")).unwrap();
        let config: ReferenceGraphConfig =
            serde_json::from_value(fixture["input"]["config"].clone()).unwrap();
        let body = CognitiveRef::MemoryRevision(MemoryRevisionId::new());
        let a = CognitiveRef::Tag(TagId::new());
        let b = CognitiveRef::Tag(TagId::new());
        let ids = VcpIdentityMap::new([body.clone(), a.clone(), b.clone()]);
        let ai = ids.id(&a).unwrap();
        let bi = ids.id(&b).unwrap();
        let edge = TopologyEdgeSource {
            from: a.clone(),
            to: b.clone(),
            support_class: "source_evidence".into(),
            association_kind: "assoc.related".into(),
            polarity: "positive".into(),
            support_mass: 0.6,
            provenance_root: Some("occurrence:one".into()),
        };
        let mut stronger = edge.clone();
        stronger.support_mass = 0.9;
        let mut independent = edge.clone();
        independent.provenance_root = Some("occurrence:two".into());
        let mut negative = edge.clone();
        negative.polarity = "negative".into();
        let mut material = VcpProjectionMaterial {
            authority_watermark: 9,
            space: serde_json::from_value(serde_json::json!({
                "space_hash":"fixture", "model_identity":"fixture", "weights_revision":"1",
                "task":"retrieval", "input_representation":"text", "preprocessing_identity":"fixture",
                "preprocessing_revision":"1", "dimension":2, "normalization":"l2", "output_semantics":"dense"
            })).unwrap(),
            producer: serde_json::from_value(serde_json::json!({
                "signature_hash":"fixture", "provider_class":"fixture", "operation":"text_embedding",
                "implementation":"fixture", "preprocessing_identity":"fixture", "preprocessing_revision":"1",
                "config_digest":"fixture"
            })).unwrap(),
            identities: ids,
            edges: vec![edge, stronger, independent, negative],
            documents: vec![VcpProjectedDocument {
                reference: body.clone(), representation_text: "fixture".into(), vector: vec![1.0, 0.0],
                concept_refs: vec![b.clone(), a.clone(), b.clone()], curve_order: VcpCurveOrder::StableIdentity,
                evidence_roots: Default::default(),
            }],
        };
        let assets = vcp_graph_assets(&material, &[(ai, bi, 0.5)], &[], &config).unwrap();
        assert_eq!(assets.membership[0].tags.len(), 2);
        assert!(
            assets.membership[0]
                .tags
                .iter()
                .all(|(_, position)| *position == 0)
        );
        assert_eq!(assets.evidence.len(), 2);
        let fact = |from, to| {
            assets
                .graph
                .fact_matrix
                .iter()
                .find(|(a, b, _)| *a == from && *b == to)
                .unwrap()
                .2
        };
        assert!((fact(ai, bi) - fact(bi, ai) - 1.5).abs() < 1e-12);
        let provenance = assets
            .graph
            .provenance
            .iter()
            .find(|e| e.source_id == ai && e.target_id == bi)
            .unwrap();
        assert!(
            (provenance
                .file_contributions
                .iter()
                .map(|(_, m)| m)
                .sum::<f64>()
                - fact(ai, bi))
            .abs()
                < 1e-12
        );
        let roots = provenance
            .file_contributions
            .iter()
            .map(|(id, _)| assets.provenance_roots[*id as usize - 1].as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            roots,
            BTreeSet::from([
                body.to_string().as_str(),
                "occurrence:one",
                "occurrence:two"
            ])
        );
        material.documents[0].concept_refs = vec![a, b];
        material.edges.reverse();
        let reordered = vcp_graph_assets(&material, &[(ai, bi, 0.5)], &[], &config).unwrap();
        assert_eq!(
            serde_json::to_value(&assets).unwrap(),
            serde_json::to_value(reordered).unwrap()
        );
        material.edges[0].polarity = "positive".into();
        material.edges[0].provenance_root = None;
        assert!(vcp_graph_assets(&material, &[], &[], &config).is_err());
        material.edges[0].provenance_root = Some("occurrence:restored".into());
        check_index_roundtrip(material);
    }

    fn check_index_roundtrip(mut material: VcpProjectionMaterial) {
        let tag_refs = material.documents[0].concept_refs.clone();
        for (i, reference) in tag_refs.into_iter().enumerate() {
            material.documents.push(VcpProjectedDocument {
                reference,
                representation_text: format!("Tag{i}"),
                vector: if i == 0 {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                },
                concept_refs: Vec::new(),
                curve_order: VcpCurveOrder::StableIdentity,
                evidence_roots: Default::default(),
            });
        }
        let assets = crate::VcpGeneration::build(
            nous_core::ServingGenerationId::new(),
            nous_runtime::CognitiveProfile::VcpDtsc,
            material,
            crate::VcpAssetPolicy::default(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let index = crate::VcpServingGeneration::create(assets, directory.path()).unwrap();
        let hits = index.search_residual_tags(&[1.0, 0.0], 2).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "Tag0");
        assert!((hits[0].similarity - 1.0).abs() < 1e-6);
        assert_eq!(hits[0].vector, vec![1.0, 0.0]);
        let decoded = serde_json::from_value(serde_json::to_value(&index).unwrap()).unwrap();
        let reopened = crate::VcpServingGeneration::open(decoded, directory.path()).unwrap();
        assert_eq!(
            serde_json::to_value(reopened.search_residual_tags(&[1.0, 0.0], 2).unwrap()).unwrap(),
            serde_json::to_value(hits).unwrap()
        );
    }
}
