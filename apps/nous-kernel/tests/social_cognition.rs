#[path = "test_support/mod.rs"]
mod test_support;

use chrono::Utc;
use nous_core::{
    CognitiveQuery, CognitiveRef, Cue, EntityRef, EpistemicClass, OperationId, QueryTarget,
    RevisionSupport, SeedSupportRef, SocialRelationCue, TemporalExtent,
};
use nous_social_domain::SocialScope;
use nous_social_domain::{
    DegreeSemantics, SocialEntityKind, SocialParty, TemporalSemantics, ViewSemantics,
};
use nous_social_service::{
    CreateLanguageConvention, CreateRelationship, MutateSocialLifecycle, RegisterRelationType,
    ReviseRelationship,
};
use nous_subject_core::{CognitiveSeedInput, CreateSubject, SubjectCapabilities};
use test_support::{database, open_runtime_with_social};

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
                SeedSupportRef::new(
                    nous_core::CognitiveSeedVersionId::new(),
                    "social.relationships/alice-friend",
                )
                .unwrap(),
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
                SeedSupportRef::new(
                    nous_core::CognitiveSeedVersionId::new(),
                    "social.relationships/alice-friend",
                )
                .unwrap(),
            )],
            producer_signature_id: None,
        })
        .await
        .expect("relationship revision");
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
                SeedSupportRef::new(
                    nous_core::CognitiveSeedVersionId::new(),
                    "social.conventions/alice-project",
                )
                .unwrap(),
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
