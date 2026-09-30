use chrono::Utc;
use nous_core::{EpistemicClass, OccurrenceId, OperationId, SubjectId, TemporalExtent};
use nous_memory::{
    CognitiveRole, EvidenceLocator, EvidenceRef, ExplicitMemoryInput, FormationMode,
    RevisionSupport, SupportRole,
};

fn grounded_input() -> ExplicitMemoryInput {
    let occurrence = OccurrenceId::new();
    ExplicitMemoryInput {
        producer: None,
        operation_id: OperationId::new(),
        subject: SubjectId::new(),
        cognitive_role: CognitiveRole::Declarative,
        formation_mode: FormationMode::Grounded,
        grounding_occurrence_id: Some(occurrence),
        semantic_role: "fact".into(),
        representation_text: "a bounded fact".into(),
        title: None,
        supports: vec![RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: occurrence,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        })],
        aboutness: Vec::new(),
        tags: Vec::new(),
        valid_time: TemporalExtent::Unknown,
        formed_at: Utc::now(),
        epistemic_class: EpistemicClass::Observed,
    }
}

#[test]
fn role_and_formation_are_independent_domain_dimensions() {
    let value = grounded_input();
    assert_eq!(value.cognitive_role, CognitiveRole::Declarative);
    assert_eq!(value.formation_mode, FormationMode::Grounded);
    assert!(value.validate().is_ok());
}

#[test]
fn grounded_support_requires_the_declared_occurrence() {
    let mut value = grounded_input();
    value.grounding_occurrence_id = Some(OccurrenceId::new());
    assert!(value.validate().is_err());
}
