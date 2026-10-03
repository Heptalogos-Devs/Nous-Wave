CREATE INDEX memory_revisions_subject_time_idx ON memory_revisions(subject_id, recorded_at DESC);
CREATE INDEX memory_revision_evidence_occurrence_idx ON memory_revision_evidence(occurrence_id);
CREATE INDEX resident_refs_session_state_idx ON resident_refs(session_id, state, last_meaningful_use_at DESC);
-- Typed retrieval lanes use Authority-side joins instead of application-side
-- object sampling. These indexes keep bounded predicates on semantic keys.
CREATE INDEX memory_revision_aboutness_entity_idx
    ON memory_revision_aboutness(entity_ref, memory_revision_id);

CREATE INDEX memory_revisions_subject_valid_idx
    ON memory_revisions(subject_id, valid_time_start, valid_time_end, recorded_at DESC);

CREATE INDEX memory_revision_evidence_revision_idx
    ON memory_revision_evidence(memory_revision_id, occurrence_id);

CREATE INDEX observation_occurrences_subject_occurred_idx
    ON observation_occurrences(subject_id, occurred_time_start, occurred_time_end);

CREATE INDEX observation_occurrences_subject_observed_asc_idx
    ON observation_occurrences(subject_id, observed_at);
CREATE INDEX memory_dependency_source_idx
    ON memory_revision_dependencies(target_ref_kind,target_ref,memory_revision_id);
CREATE INDEX schema_dependency_source_idx
    ON cognitive_schema_evidence_links(support_kind,support_ref,schema_revision_id)
    WHERE revoked_at IS NULL;
CREATE INDEX journal_point_dependency_source_idx
    ON journal_point_supports((support#>>'{value,target_revision,kind}'),(support#>>'{value,target_revision,id}'),journal_revision_id)
    WHERE support->>'kind'='cognition_dependency';
