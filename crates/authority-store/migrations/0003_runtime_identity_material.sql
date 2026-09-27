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
