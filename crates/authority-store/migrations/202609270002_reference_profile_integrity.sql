CREATE TABLE IF NOT EXISTS cognition_dependency_invalidations (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    dependent_kind text NOT NULL,
    dependent_ref text NOT NULL,
    invalidated_by_kind text NOT NULL,
    invalidated_by_ref text NOT NULL,
    reason text NOT NULL,
    created_at timestamptz NOT NULL,
    PRIMARY KEY(dependent_kind, dependent_ref, invalidated_by_kind, invalidated_by_ref)
);
