#[path = "test_support/mod.rs"]
mod test_support;

use chrono::Utc;
use nous_core::{
    CognitiveQuery, CognitiveRef, Cue, EntityRef, EpistemicClass, EvidenceLocator, EvidenceRef,
    OperationId, QueryTarget, RevisionSupport, SeedSupportRef, SocialRelationCue, SupportRole,
    TemporalExtent,
};
use nous_self_domain::{CreateSelfFacet, SelfFacetKind};
use nous_social_domain::SocialScope;
use nous_social_domain::{
    ConventionFormationEvidence, DegreeSemantics, FormationEvidenceKind, SocialEntityKind,
    SocialParty, TemporalSemantics, ViewSemantics,
};
use nous_social_service::{
    CreateLanguageConvention, CreateRelationship, MutateSocialLifecycle, RegisterRelationType,
    ReviseRelationship,
};
use nous_subject_core::{CognitiveSeedInput, CreateSubject, SubjectCapabilities};
use test_support::{
    database, external_actor_observation, form_input, initial_seed_version, observation,
    open_runtime_with_all_domains, open_runtime_with_social,
};

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Social focused scenario keeps Authority, revision, lifecycle, convention, and query evidence together"
)]
async fn directed_relationship_has_social_authority_and_query_projection() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_social(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: Some(SubjectCapabilities {
                memory: true,
                self_cognition: false,
                social: true,
            }),
        })
        .await
        .expect("subject")
        .subject_id;
    let seed_version_id = initial_seed_version(&runtime, subject).await;
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .register_relation_type(RegisterRelationType {
            operation_id: OperationId::new(),
            subject,
            key: "friend".into(),
            allowed_from_kinds: vec!["subject".into()],
            allowed_to_kinds: vec!["person".into()],
            view_semantics: ViewSemantics::Directed,
            degree_semantics: DegreeSemantics::None,
            temporal_semantics: TemporalSemantics::State,
            source_seed_version_id: None,
            source_seed_path: None,
        })
        .await
        .expect("relation type");
    let relationship = runtime
        .social
        .as_ref()
        .expect("social capability")
        .create_relationship(CreateRelationship {
            operation_id: OperationId::new(),
            subject,
            relation_type_key: "friend".into(),
            from: SocialParty::SubjectSelf,
            to: SocialParty::Entity {
                entity_kind: SocialEntityKind::Person,
                entity_ref: EntityRef::new("entity:person:alice").unwrap(),
            },
            degree: None,
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(
                SeedSupportRef::new(seed_version_id, "social.relationships/alice-friend").unwrap(),
            )],
            producer_signature_id: None,
        })
        .await
        .expect("relationship");
    let revised = runtime
        .social
        .as_ref()
        .expect("social capability")
        .revise_relationship(ReviseRelationship {
            operation_id: OperationId::new(),
            subject,
            relationship_id: relationship.assertion.relationship_id,
            expected_object_epoch: relationship.assertion.object_epoch,
            parent_revision_id: relationship.revision.relationship_revision_id,
            revision_intent: "refine".into(),
            degree: None,
            epistemic_class: EpistemicClass::Observed,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(
                SeedSupportRef::new(seed_version_id, "social.relationships/alice-friend").unwrap(),
            )],
            producer_signature_id: None,
        })
        .await
        .expect("relationship revision");
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .register_relation_type(RegisterRelationType {
            operation_id: OperationId::new(),
            subject,
            key: "knows".into(),
            allowed_from_kinds: vec!["person".into()],
            allowed_to_kinds: vec!["person".into()],
            view_semantics: ViewSemantics::SymmetricView,
            degree_semantics: DegreeSemantics::None,
            temporal_semantics: TemporalSemantics::State,
            source_seed_version_id: None,
            source_seed_path: None,
        })
        .await
        .expect("symmetric relation type");
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .create_relationship(CreateRelationship {
            operation_id: OperationId::new(),
            subject,
            relation_type_key: "knows".into(),
            from: SocialParty::Entity {
                entity_kind: SocialEntityKind::Person,
                entity_ref: EntityRef::new("entity:person:bob").unwrap(),
            },
            to: SocialParty::Entity {
                entity_kind: SocialEntityKind::Person,
                entity_ref: EntityRef::new("entity:person:alice").unwrap(),
            },
            degree: None,
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(
                SeedSupportRef::new(seed_version_id, "social.relationships/knows").unwrap(),
            )],
            producer_signature_id: None,
        })
        .await
        .expect("symmetric relationship");
    let symmetric = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::Social],
            cues: vec![Cue::SocialRelation(SocialRelationCue {
                relation_type_key: Some("knows".into()),
                from: Some(EntityRef::new("entity:person:alice").unwrap()),
                to: Some(EntityRef::new("entity:person:bob").unwrap()),
                include_views: true,
            })],
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("symmetric view query");
    assert!(symmetric.results.iter().any(|hit| {
        hit.match_evidence
            .variants
            .iter()
            .any(|variant| variant == "social:symmetric-view")
    }));
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .register_relation_type(RegisterRelationType {
            operation_id: OperationId::new(),
            subject,
            key: "follows".into(),
            allowed_from_kinds: vec!["person".into()],
            allowed_to_kinds: vec!["person".into()],
            view_semantics: ViewSemantics::InverseView {
                inverse_key: "followed_by".into(),
            },
            degree_semantics: DegreeSemantics::None,
            temporal_semantics: TemporalSemantics::State,
            source_seed_version_id: None,
            source_seed_path: None,
        })
        .await
        .expect("inverse relation type");
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .create_relationship(CreateRelationship {
            operation_id: OperationId::new(),
            subject,
            relation_type_key: "follows".into(),
            from: SocialParty::Entity {
                entity_kind: SocialEntityKind::Person,
                entity_ref: EntityRef::new("entity:person:alice").unwrap(),
            },
            to: SocialParty::Entity {
                entity_kind: SocialEntityKind::Person,
                entity_ref: EntityRef::new("entity:person:bob").unwrap(),
            },
            degree: None,
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(
                SeedSupportRef::new(seed_version_id, "social.relationships/follows").unwrap(),
            )],
            producer_signature_id: None,
        })
        .await
        .expect("inverse relationship");
    let inverse = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::Social],
            cues: vec![Cue::SocialRelation(SocialRelationCue {
                relation_type_key: Some("followed_by".into()),
                from: Some(EntityRef::new("entity:person:bob").unwrap()),
                to: Some(EntityRef::new("entity:person:alice").unwrap()),
                include_views: true,
            })],
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("inverse view query");
    assert!(inverse.results.iter().any(|hit| {
        hit.match_evidence
            .variants
            .iter()
            .any(|variant| variant == "social:inverse-view:followed_by")
    }));
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .mutate_lifecycle(MutateSocialLifecycle {
            operation_id: OperationId::new(),
            subject,
            reference: CognitiveRef::RelationshipAssertion(revised.assertion.relationship_id),
            expected_object_epoch: revised.assertion.object_epoch,
            operation: "suppress".into(),
        })
        .await
        .expect("relationship suppress");
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .mutate_lifecycle(MutateSocialLifecycle {
            operation_id: OperationId::new(),
            subject,
            reference: CognitiveRef::RelationshipAssertion(revised.assertion.relationship_id),
            expected_object_epoch: revised.assertion.object_epoch + 1,
            operation: "restore".into(),
        })
        .await
        .expect("relationship restore");
    runtime
        .social
        .as_ref()
        .expect("social capability")
        .create_language_convention(CreateLanguageConvention {
            operation_id: OperationId::new(),
            subject,
            key: "alice-project".into(),
            expression: "X".into(),
            scope: SocialScope::Person(EntityRef::new("entity:person:alice").unwrap()),
            context_scope: None,
            topic_scope: None,
            meaning: "the project codename".into(),
            pragmatic_role: Some("shorthand".into()),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(
                SeedSupportRef::new(seed_version_id, "social.conventions/alice-project").unwrap(),
            )],
            formation_evidence: vec![nous_social_domain::ConventionFormationEvidence {
                support_index: 0,
                kind: nous_social_domain::FormationEvidenceKind::SeedDirect,
                external_actor: None,
            }],
            producer_signature_id: None,
        })
        .await
        .expect("language convention");
    let result = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::Social],
            cues: vec![Cue::SocialRelation(SocialRelationCue {
                relation_type_key: Some("friend".into()),
                from: None,
                to: Some(EntityRef::new("entity:person:alice").unwrap()),
                include_views: false,
            })],
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("social query");
    assert!(
        result
            .results
            .iter()
            .any(|hit| matches!(hit.reference, CognitiveRef::RelationshipRevision(_)))
    );
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Mixed-owner fixture keeps composition, Authority objects, and one-fusion assertions together"
)]
async fn mixed_memory_self_social_text_query_uses_one_runtime_fusion() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_all_domains(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: Some(SubjectCapabilities {
                memory: true,
                self_cognition: true,
                social: true,
            }),
        })
        .await
        .expect("subject")
        .subject_id;
    let seed_version_id = initial_seed_version(&runtime, subject).await;
    let occurrence = observation(&runtime, subject, "shared memory phrase").await;
    let memory = runtime
        .require_memory()
        .expect("memory")
        .form_memory(form_input(
            subject,
            occurrence.occurrence.occurrence_id,
            OperationId::new(),
            "shared memory phrase",
        ))
        .await
        .expect("memory formation");
    runtime
        .require_self()
        .expect("Self")
        .create_facet(CreateSelfFacet {
            operation_id: OperationId::new(),
            subject,
            kind: SelfFacetKind::Identity,
            key: "shared-identity".into(),
            statement: "shared self phrase".into(),
            scope: "subject".into(),
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::CognitionDependency(
                nous_core::CognitionDependency {
                    target_revision: CognitiveRef::MemoryRevision(
                        memory.revision.memory_revision_id,
                    ),
                    support_role: SupportRole::Contextual,
                },
            )],
            producer_signature_id: None,
        })
        .await
        .expect("Self facet");
    runtime
        .social
        .as_ref()
        .expect("social")
        .register_relation_type(RegisterRelationType {
            operation_id: OperationId::new(),
            subject,
            key: "shared".into(),
            allowed_from_kinds: vec!["subject".into()],
            allowed_to_kinds: vec!["person".into()],
            view_semantics: ViewSemantics::Directed,
            degree_semantics: DegreeSemantics::None,
            temporal_semantics: TemporalSemantics::State,
            source_seed_version_id: None,
            source_seed_path: None,
        })
        .await
        .expect("relation type");
    runtime
        .social
        .as_ref()
        .expect("social")
        .create_relationship(CreateRelationship {
            operation_id: OperationId::new(),
            subject,
            relation_type_key: "shared".into(),
            from: SocialParty::SubjectSelf,
            to: SocialParty::Entity {
                entity_kind: SocialEntityKind::Person,
                entity_ref: EntityRef::new("entity:person:shared").unwrap(),
            },
            degree: None,
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: vec![RevisionSupport::Seed(
                SeedSupportRef::new(seed_version_id, "social.relationships/shared").unwrap(),
            )],
            producer_signature_id: None,
        })
        .await
        .expect("relationship");
    let result = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: Vec::new(),
            cues: vec![Cue::Text(nous_core::TextCue {
                text: "shared".into(),
            })],
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("mixed query");
    assert!(
        result
            .results
            .iter()
            .any(|hit| matches!(hit.reference, CognitiveRef::MemoryRevision(_)))
    );
    assert!(
        result
            .results
            .iter()
            .any(|hit| matches!(hit.reference, CognitiveRef::SelfFacetRevision(_)))
    );
    assert!(
        result
            .results
            .iter()
            .any(|hit| matches!(hit.reference, CognitiveRef::RelationshipRevision(_)))
    );
    assert!(result.results.iter().all(|hit| {
        !hit.match_evidence.families.is_empty() && hit.match_evidence.final_score > 0.0
    }));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "Formation acceptance fixture covers independent roots and exact external actor fencing"
)]
async fn language_convention_external_formation_requires_independent_owned_evidence() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_social(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: Some(SubjectCapabilities {
                memory: true,
                self_cognition: false,
                social: true,
            }),
        })
        .await
        .expect("subject")
        .subject_id;
    let actor = EntityRef::new("entity:person:alice").unwrap();
    let first = external_actor_observation(&runtime, subject, actor.as_str(), "object:one").await;
    let second = external_actor_observation(&runtime, subject, actor.as_str(), "object:two").await;
    let supports = vec![
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: first.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: second.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
    ];
    runtime
        .social
        .as_ref()
        .expect("social")
        .create_language_convention(CreateLanguageConvention {
            operation_id: OperationId::new(),
            subject,
            key: "external-code".into(),
            expression: "X".into(),
            scope: SocialScope::Person(actor.clone()),
            context_scope: None,
            topic_scope: None,
            meaning: "the external code".into(),
            pragmatic_role: None,
            epistemic_class: EpistemicClass::Reported,
            valid_time: TemporalExtent::Unknown,
            formed_at: Utc::now(),
            supports: supports.clone(),
            formation_evidence: vec![
                ConventionFormationEvidence {
                    support_index: 0,
                    kind: FormationEvidenceKind::ExternalConsistentUse,
                    external_actor: Some(actor.clone()),
                },
                ConventionFormationEvidence {
                    support_index: 1,
                    kind: FormationEvidenceKind::ExternalConsistentUse,
                    external_actor: Some(actor.clone()),
                },
            ],
            producer_signature_id: None,
        })
        .await
        .expect("independent external formation");

    let same_root_a =
        external_actor_observation(&runtime, subject, actor.as_str(), "object:same").await;
    let same_root_b =
        external_actor_observation(&runtime, subject, actor.as_str(), "object:same").await;
    let same_root_supports = vec![
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: same_root_a.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: same_root_b.occurrence.occurrence_id,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        }),
    ];
    assert!(
        runtime
            .social
            .as_ref()
            .expect("social")
            .create_language_convention(CreateLanguageConvention {
                operation_id: OperationId::new(),
                subject,
                key: "same-root-code".into(),
                expression: "Y".into(),
                scope: SocialScope::Person(actor.clone()),
                context_scope: None,
                topic_scope: None,
                meaning: "same root must not form".into(),
                pragmatic_role: None,
                epistemic_class: EpistemicClass::Reported,
                valid_time: TemporalExtent::Unknown,
                formed_at: Utc::now(),
                supports: same_root_supports,
                formation_evidence: vec![
                    ConventionFormationEvidence {
                        support_index: 0,
                        kind: FormationEvidenceKind::ExternalConsistentUse,
                        external_actor: Some(actor.clone()),
                    },
                    ConventionFormationEvidence {
                        support_index: 1,
                        kind: FormationEvidenceKind::ExternalConsistentUse,
                        external_actor: Some(actor.clone()),
                    },
                ],
                producer_signature_id: None,
            })
            .await
            .is_err()
    );

    let mismatch =
        external_actor_observation(&runtime, subject, actor.as_str(), "object:three").await;
    assert!(
        runtime
            .social
            .as_ref()
            .expect("social")
            .create_language_convention(CreateLanguageConvention {
                operation_id: OperationId::new(),
                subject,
                key: "actor-mismatch".into(),
                expression: "Z".into(),
                scope: SocialScope::Person(actor.clone()),
                context_scope: None,
                topic_scope: None,
                meaning: "actor must match occurrence".into(),
                pragmatic_role: None,
                epistemic_class: EpistemicClass::Reported,
                valid_time: TemporalExtent::Unknown,
                formed_at: Utc::now(),
                supports: vec![RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: mismatch.occurrence.occurrence_id,
                    locator: EvidenceLocator::WholeOccurrence,
                    support_role: SupportRole::Direct,
                })],
                formation_evidence: vec![ConventionFormationEvidence {
                    support_index: 0,
                    kind: FormationEvidenceKind::ExplicitExplanation,
                    external_actor: Some(EntityRef::new("entity:person:bob").unwrap()),
                }],
                producer_signature_id: None,
            })
            .await
            .is_err()
    );
}
#[tokio::test]
async fn social_seed_import_is_idempotent_and_definition_conflicts_are_reported() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_social(&url, &root).await;
    let subject = runtime
        .subjects
        .create_subject(CreateSubject {
            subject_id: None,
            operation_id: OperationId::new(),
            cognitive_seed: CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject_core::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({}),
            },
            metadata: serde_json::json!({}),
            capabilities: Some(SubjectCapabilities {
                memory: true,
                self_cognition: false,
                social: true,
            }),
        })
        .await
        .expect("subject")
        .subject_id;
    let seed_version_id = initial_seed_version(&runtime, subject).await;
    let seed = r#"
schema_version = 1

[[social.relation_types]]
key = "friend"
allowed_from = ["subject"]
allowed_to = ["person"]
view = "directed"
temporal = "state"

[[social.relationships]]
key = "alice-friend"
type = "friend"
epistemic = "reported"

[social.relationships.from]
kind = "subject"

[social.relationships.to]
kind = "person"
ref = "entity:person:alice"

[[social.conventions]]
key = "alice-project"
expression = "X"
meaning = "the project codename"
epistemic = "reported"

[social.conventions.scope]
kind = "person"
ref = "entity:person:alice"
"#;
    let first = runtime
        .social
        .as_ref()
        .expect("social")
        .import_seed(subject, seed_version_id, seed)
        .await
        .expect("first import");
    assert_eq!(first.created.len(), 3);
    let second = runtime
        .social
        .as_ref()
        .expect("social")
        .import_seed(subject, seed_version_id, seed)
        .await
        .expect("retry import");
    assert_eq!(second.unchanged.len(), 3);
    assert!(second.created.is_empty());

    let conflict = seed.replace("allowed_to = [\"person\"]", "allowed_to = [\"group\"]");
    let conflicted = runtime
        .social
        .as_ref()
        .expect("social")
        .import_seed(subject, seed_version_id, &conflict)
        .await
        .expect("conflicting import result");
    assert!(
        conflicted
            .conflicts
            .iter()
            .any(|path| path == "social.relation-types/friend")
    );
}
