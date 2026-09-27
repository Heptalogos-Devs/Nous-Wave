-- R2 typed lanes use Authority-side joins instead of application-side object
-- sampling. These indexes keep the bounded predicates on their semantic keys.
CREATE INDEX memory_revision_aboutness_entity_idx
    ON memory_revision_aboutness(entity_ref, memory_revision_id);

CREATE INDEX memory_revisions_subject_valid_idx
    ON memory_revisions(subject_id, valid_time_start, valid_time_end, recorded_at DESC);

CREATE INDEX memory_revision_evidence_revision_idx
    ON memory_revision_evidence(memory_revision_id, occurrence_id);

CREATE INDEX observation_occurrences_subject_occurred_idx
    ON observation_occurrences(subject_id, occurred_time_start, occurred_time_end);

CREATE INDEX observation_occurrences_subject_observed_r2_idx
    ON observation_occurrences(subject_id, observed_at);
