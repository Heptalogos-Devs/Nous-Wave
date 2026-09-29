CREATE TABLE mutation_receipts (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    operation_id uuid NOT NULL,
    operation_kind text NOT NULL,
    request_digest text NOT NULL CHECK (request_digest ~ '^[0-9a-f]{64}$'),
    state text NOT NULL CHECK (state IN ('in_progress','committed','failed_terminal')),
    result_kind text NULL,
    result_ref_kind text NULL,
    result_ref text NULL,
    result_revision uuid NULL,
    result_epoch bigint NULL,
    created_at timestamptz NOT NULL,
    committed_at timestamptz NULL,
    terminal_problem_code text NULL,
    PRIMARY KEY(subject_id, operation_id)
);
CREATE TABLE cognition_dependency_invalidations (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    dependent_kind text NOT NULL,
    dependent_ref text NOT NULL,
    invalidated_by_kind text NOT NULL,
    invalidated_by_ref text NOT NULL,
    reason text NOT NULL,
    created_at timestamptz NOT NULL,
    PRIMARY KEY(dependent_kind, dependent_ref, invalidated_by_kind, invalidated_by_ref)
);

CREATE TABLE cognitive_sessions (
    session_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    opened_at timestamptz NOT NULL,
    last_activity_at timestamptz NOT NULL,
    last_meaningful_use_at timestamptz NULL,
    closed_at timestamptz NULL,
    runtime_revision bigint NOT NULL DEFAULT 0,
    active_focus_key text NULL,
    metadata jsonb NOT NULL DEFAULT '{}'
);

CREATE TABLE resident_refs (
    session_id uuid NOT NULL REFERENCES cognitive_sessions(session_id) ON DELETE CASCADE,
    ref_kind text NOT NULL CHECK (ref_kind IN ('memory_revision','cognitive_schema_revision')),
    ref_value text NOT NULL,
    entered_at timestamptz NOT NULL,
    entry_reason text NOT NULL,
    last_meaningful_use_at timestamptz NULL,
    hold_until timestamptz NULL,
    state text NOT NULL CHECK (state IN ('resident','provisional','evicted')),
    metadata jsonb NOT NULL DEFAULT '{}',
    PRIMARY KEY(session_id, ref_kind, ref_value)
);
CREATE TABLE cognitive_use_events (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    consumer_ref text NOT NULL CHECK (octet_length(consumer_ref) BETWEEN 1 AND 512 AND consumer_ref !~ '[[:space:]]' AND consumer_ref LIKE '%:%:%'),
    event_id uuid NOT NULL,
    ref_kind text NOT NULL CHECK (ref_kind IN ('memory_revision','cognitive_schema_revision')),
    ref_value text NOT NULL,
    use_kind text NOT NULL CHECK (use_kind IN ('presented','referenced','acted_on','result_supported','result_refuted','corrected','pinned')),
    session_id uuid NULL REFERENCES cognitive_sessions(session_id) ON DELETE SET NULL,
    occurred_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    context jsonb NOT NULL DEFAULT '{}',
    request_digest text NOT NULL CHECK (request_digest ~ '^[0-9a-f]{64}$'),
    PRIMARY KEY(subject_id, consumer_ref, event_id)
);
CREATE TABLE purged_use_receipts (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    consumer_ref text NOT NULL,
    event_id uuid NOT NULL,
    request_digest text NOT NULL CHECK (request_digest ~ '^[0-9a-f]{64}$'),
    purged_at timestamptz NOT NULL,
    PRIMARY KEY(subject_id, consumer_ref, event_id)
);

CREATE TABLE serving_generations (
    generation_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    family text NOT NULL CHECK (family IN ('exact','lexical','dense','topology')),
    space_signature text NULL,
    authority_watermark bigint NOT NULL,
    implementation_id text NOT NULL,
    implementation_revision text NOT NULL,
    config_digest text NOT NULL,
    artifact_location text NOT NULL,
    artifact_hash text NOT NULL,
    state text NOT NULL CHECK (state IN ('building','ready','retired','failed')),
    built_at timestamptz NOT NULL,
    published_at timestamptz NULL,
    metadata jsonb NOT NULL DEFAULT '{}'
);
CREATE TABLE serving_current (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    family text NOT NULL,
    space_signature text NOT NULL DEFAULT '',
    generation_id uuid NOT NULL REFERENCES serving_generations(generation_id) ON DELETE CASCADE,
    PRIMARY KEY(subject_id, family, space_signature)
);
CREATE TABLE projection_watermarks (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    family text NOT NULL,
    space_signature text NOT NULL DEFAULT '',
    desired_authority_seq bigint NOT NULL,
    PRIMARY KEY(subject_id, family, space_signature)
);

CREATE TABLE runtime_checkpoints (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id),
    session_id uuid NOT NULL REFERENCES cognitive_sessions(session_id),
    owner_kind text NOT NULL CHECK (owner_kind IN ('focus', 'context', 'steward')),
    owner_key text NOT NULL CHECK (length(owner_key) BETWEEN 1 AND 256),
    schema_version integer NOT NULL CHECK (schema_version > 0),
    revision bigint NOT NULL CHECK (revision > 0),
    payload bytea NOT NULL CHECK (octet_length(payload) <= 1048576),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, owner_kind, owner_key)
);

CREATE TABLE observation_request_bindings (
    request_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id),
    request_digest text NOT NULL,
    occurrence_id uuid NOT NULL REFERENCES observation_occurrences(occurrence_id),
    accepted jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE embedding_materials (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id),
    content_digest text NOT NULL,
    space_hash text NOT NULL,
    producer_hash text NOT NULL,
    vector real[] NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (subject_id, content_digest, space_hash, producer_hash)
);

CREATE TABLE lexical_bindings (
    lexical_ref text PRIMARY KEY,
    object_kind text NOT NULL,
    canonical_ref text NOT NULL,
    wordlist_version integer NOT NULL CHECK (wordlist_version = 1),
    tombstoned_at timestamptz NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (object_kind, canonical_ref)
);
CREATE TABLE lexical_visibility (
    lexical_ref text NOT NULL REFERENCES lexical_bindings(lexical_ref),
    subject_id uuid NOT NULL REFERENCES subjects(subject_id),
    display_name text NOT NULL,
    aliases text[] NOT NULL,
    PRIMARY KEY(subject_id, lexical_ref)
);
CREATE INDEX lexical_aliases ON lexical_visibility USING gin(aliases);
