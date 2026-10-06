// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use chrono::{Duration, Utc};
use nous_core::{
    EmbeddingSpaceSignature, EntityRef, ObjectRef, TemporalExtent, TimeInterval,
    tag_semantic_representation,
};

#[test]
fn opaque_refs_require_their_owner_namespace() {
    assert!(EntityRef::new("entity:host-person:alice").is_ok());
    assert!(EntityRef::new("name:alice").is_err());
    assert!(ObjectRef::new("object:messaging:message:1").is_ok());
}

#[test]
fn embedding_spaces_are_exactly_compatible() {
    let base = EmbeddingSpaceSignature {
        space_hash: "a".into(),
        model_identity: "model".into(),
        weights_revision: "1".into(),
        task: "retrieval".into(),
        input_representation: "text".into(),
        preprocessing_identity: "l2".into(),
        preprocessing_revision: "1".into(),
        dimension: 3,
        normalization: "l2".into(),
        output_semantics: "dense_similarity".into(),
    };
    assert!(base.compatible_with(&base));
    let mut changed = base.clone();
    changed.dimension = 4;
    assert!(!base.compatible_with(&changed));
}

#[test]
fn temporal_intervals_are_half_open_and_unknown_stays_unknown() {
    let start = Utc::now();
    let end = start + Duration::hours(1);
    let interval = TimeInterval {
        start: Some(start),
        end: Some(end),
    };
    assert!(interval.contains(start));
    assert!(!interval.contains(end));
    assert!(!TemporalExtent::Unknown.overlaps_interval(&interval));
    assert!(
        !TemporalExtent::Interval {
            start: Some(start),
            end: Some(end),
        }
        .overlaps_interval(&TimeInterval {
            start: Some(end),
            end: Some(end + Duration::hours(1)),
        })
    );
}

#[test]
fn tag_semantic_text_is_canonical_bounded_and_revision_sensitive() {
    let label = tag_semantic_representation("reader reclamation", None, None).unwrap();
    assert_eq!(label.text, "Concept:\nreader reclamation");
    let described = tag_semantic_representation(
        "reader reclamation",
        Some("Wait until all\r\nreaders leave"),
        Some("procedure"),
    )
    .unwrap();
    assert_eq!(
        described.text,
        "Concept:\nreader reclamation\n\nDescription:\nWait until all\nreaders leave\n\nKind:\nprocedure"
    );
    assert_ne!(label.digest, described.digest);
    assert_eq!(
        described.digest,
        tag_semantic_representation(
            "reader reclamation",
            Some("Wait until all\nreaders leave"),
            Some("procedure")
        )
        .unwrap()
        .digest
    );
    assert!(tag_semantic_representation("", None, None).is_err());
    assert!(tag_semantic_representation("concept", Some(&"x".repeat(4097)), None).is_err());
    let long = tag_semantic_representation("规则", Some(&"经验".repeat(600)), None).unwrap();
    assert!(long.text.ends_with("经验"));
}
