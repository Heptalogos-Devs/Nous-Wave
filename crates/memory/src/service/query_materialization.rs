use super::*;
use super::query::{Candidate, FinalMemoryState};
use std::collections::BTreeMap;

pub(super) fn final_state_filter(
    query: &CognitiveQuery,
    candidate: &Candidate,
    state: &FinalMemoryState,
    exact: bool,
    historical: bool,
) -> bool {
    if !matches!(state.purge_state, PurgeState::Normal) {
        return false;
    }
    if !historical
        && (!matches!(state.acceptance_state, AcceptanceState::Accepted)
            || !matches!(state.integrity_state, IntegrityState::Valid)
            || !matches!(state.suppression_state, SuppressionState::Normal))
    {
        return false;
    }
    if !hard_filter(query, &candidate.view, exact, historical) {
        return false;
    }
    let level = match state.accessibility_mode {
        AccessibilityMode::Normal => AccessibilityLevel::Normal,
        AccessibilityMode::Deep => AccessibilityLevel::Deep,
        AccessibilityMode::Explicit => AccessibilityLevel::Explicit,
        AccessibilityMode::Auto => candidate.view.accessibility_level,
    };
    accessibility_eligible(level, query.effort, exact || historical)
}

pub(super) fn to_hit(
    query: &CognitiveQuery,
    candidate: &Candidate,
    reference: CognitiveRef,
) -> CognitiveHit {
    CognitiveHit {
        reference,
        revision: Some(CognitiveRef::MemoryRevision(
            candidate.view.revision.memory_revision_id,
        )),
        semantic_role: Some(candidate.view.revision.semantic_role.clone()),
        cognitive_role: Some(candidate.view.object.cognitive_role.as_str().into()),
        formation_mode: Some(candidate.view.revision.formation_mode.as_str().into()),
        representation: Some(candidate.view.revision.representation_text.clone()),
        authority: AuthorityClass::SubjectCognition,
        freshness: FreshnessDescriptor {
            observed_at: candidate.view.temporal_evidence.observed_at,
            valid_time: candidate.view.revision.valid_time.clone(),
            formed_at: Some(candidate.view.revision.formed_at),
            recorded_at: Some(candidate.view.revision.recorded_at),
        },
        entity_refs: candidate.view.aboutness.clone(),
        evidence: if query.result_need.need_evidence {
            candidate
                .view
                .supports
                .iter()
                .map(|support| match support {
                    RevisionSupport::Evidence(value) => EvidenceHandle {
                        reference: value.cognitive_ref(),
                        support_role: value.support_role.as_str().into(),
                    },
                    RevisionSupport::CognitionDependency(value) => EvidenceHandle {
                        reference: value.target_revision.clone(),
                        support_role: value.support_role.as_str().into(),
                    },
                    RevisionSupport::Seed(value) => EvidenceHandle {
                        reference: CognitiveRef::CognitiveSeedVersion(value.seed_version_id),
                        support_role: "seed".into(),
                    },
                })
                .collect()
        } else {
            Vec::new()
        },
        match_evidence: MatchEvidence::default(),
        materialization: Vec::new(),
    }
}

pub(super) fn hard_filter(
    query: &CognitiveQuery,
    view: &MemoryView,
    exact: bool,
    historical: bool,
) -> bool {
    (historical
        || (matches!(view.object.acceptance_state, AcceptanceState::Accepted)
            && matches!(view.object.integrity_state, IntegrityState::Valid)
            && matches!(view.object.suppression_state, SuppressionState::Normal)))
        && matches!(view.object.purge_state, PurgeState::Normal)
        && accessibility_eligible(view.accessibility_level, query.effort, exact || historical)
        && (query.constraints.cognitive_roles_include.is_empty()
            || query
                .constraints
                .cognitive_roles_include
                .iter()
                .any(|role| role == view.object.cognitive_role.as_str()))
        && (query.constraints.formation_modes_include.is_empty()
            || query
                .constraints
                .formation_modes_include
                .iter()
                .any(|mode| mode == view.revision.formation_mode.as_str()))
        && query
            .constraints
            .entity_requirements
            .iter()
            .all(|entity| view.aboutness.contains(entity))
        && query
            .constraints
            .authority
            .is_none_or(|value| value == AuthorityClass::SubjectCognition)
        && (query.constraints.source_classes_include.is_empty()
            || query
                .constraints
                .source_classes_include
                .iter()
                .any(|value| view.source_classes.contains(value)))
        && !query
            .constraints
            .source_classes_exclude
            .iter()
            .any(|value| view.source_classes.contains(value))
        && (query.constraints.modalities.is_empty()
            || query.constraints.modalities.contains(&Modality::Text))
        && (query.constraints.evidence_classes.is_empty()
            || query.constraints.evidence_classes.iter().any(|value| {
                format!("{:?}", view.revision.epistemic_class).to_lowercase() == *value
            }))
        && temporal_match(query, view)
}

pub(super) fn temporal_match(query: &CognitiveQuery, view: &MemoryView) -> bool {
    query
        .constraints
        .valid
        .is_none_or(|interval| view.revision.valid_time.overlaps_interval(&interval))
        && query.constraints.occurred.is_none_or(|interval| {
            view.temporal_evidence
                .occurred
                .iter()
                .any(|value| value.overlaps_interval(&interval))
        })
        && query.constraints.observed.is_none_or(|interval| {
            view.temporal_evidence
                .observed_at
                .is_some_and(|value| interval.contains(value))
        })
}

pub(super) fn increment_drop(counts: &mut BTreeMap<String, usize>, reason: &str) {
    *counts.entry(reason.into()).or_default() += 1;
}

pub(super) fn lifecycle_drop_reason(state: &FinalMemoryState) -> String {
    if !matches!(state.acceptance_state, AcceptanceState::Accepted) {
        "withdrawn".into()
    } else if !matches!(state.integrity_state, IntegrityState::Valid) {
        "revalidation_required".into()
    } else if !matches!(state.suppression_state, SuppressionState::Normal) {
        "suppressed".into()
    } else if !matches!(state.purge_state, PurgeState::Normal) {
        "purging".into()
    } else {
        "accessibility".into()
    }
}
