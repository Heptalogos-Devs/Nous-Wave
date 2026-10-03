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

        epistemic_class: EpistemicClass::Observed,
    }
}

#[test]
fn grounded_support_requires_the_declared_occurrence() {
    let mut value = grounded_input();
    value.grounding_occurrence_id = Some(OccurrenceId::new());
    assert!(value.validate().is_err());
}
