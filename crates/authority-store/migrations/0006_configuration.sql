CREATE TABLE configuration_state (
    singleton boolean PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    revision bigint NOT NULL CHECK (revision > 0)
);
INSERT INTO configuration_state(singleton, revision) VALUES(TRUE, 1);

CREATE TABLE system_configuration_overrides (
    key text PRIMARY KEY,
    value jsonb NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    updated_at timestamptz NOT NULL
);

CREATE TABLE subject_configuration_overrides (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    key text NOT NULL,
    value jsonb NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    updated_at timestamptz NOT NULL,
    PRIMARY KEY(subject_id, key)
);

CREATE TABLE configuration_mutation_receipts (
    operation_id uuid PRIMARY KEY,
    request_digest text NOT NULL CHECK (request_digest ~ '^[0-9a-f]{64}$'),
    subject_id uuid NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    key text NOT NULL,
    action text NOT NULL CHECK (action IN ('set','clear')),
    revision bigint NOT NULL CHECK (revision > 0),
    apply_mode text NOT NULL CHECK (apply_mode IN ('live','serving_rebuild','new_subjects_only','restart_process')),
    pending_restart boolean NOT NULL,
    created_at timestamptz NOT NULL
);

CREATE TABLE subject_capabilities (
    subject_id uuid PRIMARY KEY REFERENCES subjects(subject_id) ON DELETE CASCADE,
    memory boolean NOT NULL,
    self_cognition boolean NOT NULL,
    social boolean NOT NULL
);

ALTER TABLE memory_revision_evidence ADD COLUMN seed_path text NULL;
ALTER TABLE memory_revision_dependencies ADD COLUMN seed_path text NULL;
ALTER TABLE self_facet_revision_supports ADD COLUMN seed_path text NULL;
ALTER TABLE narrative_identity_revision_supports ADD COLUMN seed_path text NULL;
