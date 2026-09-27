CREATE TABLE subjects (
    subject_id uuid PRIMARY KEY,
    created_at timestamptz NOT NULL,
    authority_seq bigint NOT NULL DEFAULT 0,
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active','suspended','purging')),
    metadata jsonb NOT NULL DEFAULT '{}'
);

CREATE TABLE artifacts (
    artifact_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    content_hash text NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    media_type text NOT NULL,
    storage_key text NOT NULL,
    created_at timestamptz NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}',
    UNIQUE(subject_id, content_hash)
);

CREATE TABLE observation_occurrences (
    occurrence_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    artifact_id uuid NULL REFERENCES artifacts(artifact_id) ON DELETE RESTRICT,
    source_class text NOT NULL,
    external_object_ref text NULL,
    occurred_time_kind text NOT NULL DEFAULT 'unknown'
        CHECK (occurred_time_kind IN ('unknown','instant','interval')),
    occurred_time_start timestamptz NULL,
    occurred_time_end timestamptz NULL,
    observed_at timestamptz NOT NULL,
    conversation_ref text NULL,
    actor_entity_ref text NULL,
    context jsonb NOT NULL DEFAULT '{}',
    created_at timestamptz NOT NULL,
    CHECK (occurred_time_kind <> 'interval'
        OR occurred_time_start IS NULL
        OR occurred_time_end IS NULL
        OR occurred_time_start < occurred_time_end)
);
CREATE INDEX observation_occurrences_subject_observed_idx
    ON observation_occurrences(subject_id, observed_at DESC);

CREATE TABLE source_regions (
    source_region_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    artifact_id uuid NOT NULL REFERENCES artifacts(artifact_id) ON DELETE CASCADE,
    coordinate_kind text NOT NULL,
    coordinate jsonb NOT NULL,
    coordinate_hash text NOT NULL,
    parent_source_region_id uuid NULL REFERENCES source_regions(source_region_id),
    created_at timestamptz NOT NULL,
    UNIQUE(subject_id, artifact_id, coordinate_kind, coordinate_hash)
);

CREATE TABLE producer_signatures (
    producer_signature_id uuid PRIMARY KEY,
    signature_hash text UNIQUE NOT NULL,
    provider_class text NOT NULL,
    operation text NOT NULL,
    implementation text NOT NULL,
    model_identity text NULL,
    model_revision text NULL,
    preprocessing_identity text NOT NULL,
    preprocessing_revision text NOT NULL,
    config_digest text NOT NULL,
    created_at timestamptz NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'
);

CREATE TABLE embedding_spaces (
    embedding_space_id uuid PRIMARY KEY,
    space_hash text UNIQUE NOT NULL,
    model_identity text NOT NULL,
    weights_revision text NOT NULL,
    task text NOT NULL,
    input_representation text NOT NULL,
    preprocessing_identity text NOT NULL,
    preprocessing_revision text NOT NULL,
    dimension integer NOT NULL CHECK (dimension > 0),
    normalization text NOT NULL,
    output_semantics text NOT NULL,
    created_at timestamptz NOT NULL
);

CREATE TABLE derived_representations (
    derived_representation_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    source_region_id uuid NOT NULL REFERENCES source_regions(source_region_id) ON DELETE CASCADE,
    representation_kind text NOT NULL,
    producer_signature_id uuid NOT NULL REFERENCES producer_signatures(producer_signature_id),
    revision integer NOT NULL CHECK (revision > 0),
    payload_text text NULL,
    payload_artifact_id uuid NULL REFERENCES artifacts(artifact_id),
    quality jsonb NOT NULL DEFAULT '{}',
    created_at timestamptz NOT NULL,
    supersedes uuid NULL REFERENCES derived_representations(derived_representation_id),
    UNIQUE(subject_id, source_region_id, representation_kind, producer_signature_id, revision),
    CHECK (payload_text IS NOT NULL OR payload_artifact_id IS NOT NULL)
);

CREATE TABLE derived_regions (
    derived_region_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    derived_representation_id uuid NOT NULL REFERENCES derived_representations(derived_representation_id) ON DELETE CASCADE,
    coordinate_kind text NOT NULL,
    coordinate jsonb NOT NULL,
    coordinate_hash text NOT NULL,
    parent_derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id),
    created_at timestamptz NOT NULL,
    UNIQUE(derived_representation_id, coordinate_kind, coordinate_hash)
);

CREATE TABLE coverage_needs (
    coverage_need_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    source_region_id uuid NOT NULL REFERENCES source_regions(source_region_id) ON DELETE CASCADE,
    representation_kind text NOT NULL,
    capability_operation text NOT NULL,
    requirement text NOT NULL CHECK (requirement IN ('required','preferred','opportunistic')),
    state text NOT NULL CHECK (state IN ('missing','scheduled','ready','unavailable','failed')),
    current_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id),
    updated_at timestamptz NOT NULL,
    UNIQUE(subject_id, source_region_id, representation_kind, capability_operation)
);

CREATE TABLE derivations (
    derivation_id uuid PRIMARY KEY,
    derivation_key text UNIQUE NOT NULL,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    source_region_id uuid NOT NULL REFERENCES source_regions(source_region_id) ON DELETE CASCADE,
    representation_kind text NOT NULL,
    producer_signature_id uuid NOT NULL REFERENCES producer_signatures(producer_signature_id),
    state text NOT NULL CHECK (state IN ('pending','running','succeeded','failed','unavailable')),
    successful_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL
);

CREATE TABLE derivation_attempts (
    attempt_id uuid PRIMARY KEY,
    derivation_id uuid NOT NULL REFERENCES derivations(derivation_id) ON DELETE CASCADE,
    attempt_no integer NOT NULL CHECK (attempt_no > 0),
    state text NOT NULL CHECK (state IN ('running','succeeded','transient_failed','permanent_failed')),
    lease_owner text NULL,
    lease_until timestamptz NULL,
    started_at timestamptz NOT NULL,
    finished_at timestamptz NULL,
    problem_code text NULL,
    problem_detail jsonb NOT NULL DEFAULT '{}',
    UNIQUE(derivation_id, attempt_no)
);

CREATE TABLE entity_mentions (
    mention_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE CASCADE,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE CASCADE,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE CASCADE,
    surface text NOT NULL,
    semantic_role text NULL,
    created_at timestamptz NOT NULL,
    CHECK (num_nonnulls(occurrence_id, source_region_id, derived_region_id) >= 1)
);

CREATE TABLE entity_binding_revisions (
    binding_revision_id uuid PRIMARY KEY,
    mention_id uuid NOT NULL REFERENCES entity_mentions(mention_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    entity_ref text NULL,
    binding_state text NOT NULL CHECK (binding_state IN ('bound','unbound','disputed')),
    host_resolution_ref text NULL,
    reason text NULL,
    created_at timestamptz NOT NULL,
    UNIQUE(mention_id, revision_no)
);

CREATE TABLE resources (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    resource_ref text NOT NULL,
    display_label text NULL,
    authority_class text NOT NULL,
    coverage jsonb NOT NULL DEFAULT '{}',
    query_dimensions jsonb NOT NULL DEFAULT '{}',
    modalities jsonb NOT NULL DEFAULT '[]',
    freshness_policy jsonb NOT NULL DEFAULT '{}',
    access_cost_class text NOT NULL,
    readiness text NOT NULL CHECK (readiness IN ('ready','degraded','unavailable')),
    updated_at timestamptz NOT NULL,
    PRIMARY KEY(subject_id, resource_ref)
);

CREATE INDEX entity_binding_current_lookup_idx
    ON entity_binding_revisions(mention_id, revision_no DESC);
