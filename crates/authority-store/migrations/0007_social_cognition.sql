CREATE TABLE relation_type_definitions (
    relation_type_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    key text NOT NULL CHECK (key ~ '^[a-z0-9][a-z0-9._:-]{0,127}$'),
    allowed_from_kinds text[] NOT NULL,
    allowed_to_kinds text[] NOT NULL,
    view_kind text NOT NULL CHECK (view_kind IN ('directed','symmetric','inverse')),
    inverse_key text NULL,
    degree_kind text NOT NULL,
    degree_config jsonb NOT NULL DEFAULT '{}',
    temporal_kind text NOT NULL CHECK (temporal_kind IN ('state','interval','instant')),
    created_at timestamptz NOT NULL,
    UNIQUE(subject_id, key)
);

CREATE TABLE relationship_assertions (
    relationship_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    relation_type_id uuid NOT NULL REFERENCES relation_type_definitions(relation_type_id) ON DELETE RESTRICT,
    from_kind text NOT NULL,
    from_entity_ref text NULL,
    to_kind text NOT NULL,
    to_entity_ref text NULL,
    current_revision_id uuid NOT NULL,
    object_epoch bigint NOT NULL DEFAULT 1 CHECK (object_epoch > 0),
    acceptance_state text NOT NULL CHECK (acceptance_state IN ('accepted','withdrawn')),
    integrity_state text NOT NULL CHECK (integrity_state IN ('valid','revalidation_required')),
    suppression_state text NOT NULL CHECK (suppression_state IN ('normal','suppressed')),
    purge_state text NOT NULL CHECK (purge_state IN ('normal','purging')),
    created_at timestamptz NOT NULL,
    CHECK ((from_kind = 'subject' AND from_entity_ref IS NULL) OR (from_kind <> 'subject' AND from_entity_ref IS NOT NULL)),
    CHECK ((to_kind = 'subject' AND to_entity_ref IS NULL) OR (to_kind <> 'subject' AND to_entity_ref IS NOT NULL))
);
CREATE UNIQUE INDEX relationship_assertions_identity_idx ON relationship_assertions(
    subject_id, relation_type_id, from_kind, COALESCE(from_entity_ref,''), to_kind, COALESCE(to_entity_ref,'')
);
CREATE TABLE relationship_revisions (
    relationship_revision_id uuid PRIMARY KEY,
    relationship_id uuid NOT NULL REFERENCES relationship_assertions(relationship_id) ON DELETE CASCADE,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    parent_revision_id uuid NULL REFERENCES relationship_revisions(relationship_revision_id),
    revision_intent text NULL CHECK (revision_intent IN ('correct','refine','reinterpret','evolve')),
    degree_kind text NULL,
    degree_value jsonb NULL,
    epistemic_class text NOT NULL,
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    formed_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    UNIQUE(relationship_id, revision_no),
    CHECK (valid_time_kind <> 'interval' OR valid_time_start IS NULL OR valid_time_end IS NULL OR valid_time_start < valid_time_end)
);

ALTER TABLE relationship_assertions
    ADD CONSTRAINT relationship_assertions_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES relationship_revisions(relationship_revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE relationship_revision_supports (
    relationship_revision_id uuid NOT NULL REFERENCES relationship_revisions(relationship_revision_id) ON DELETE CASCADE,
    support_kind text NOT NULL,
    support_ref text NOT NULL,
    support_role text NOT NULL,
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    seed_path text NULL,
    PRIMARY KEY(relationship_revision_id, support_kind, support_ref, support_role),
    CHECK (num_nonnulls(source_region_id, derived_representation_id, derived_region_id) <= 1)
);

CREATE TABLE language_conventions (
    convention_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    key text NOT NULL CHECK (key ~ '^[a-z0-9][a-z0-9._-]{0,127}$'),
    expression text NOT NULL CHECK (length(trim(expression)) > 0 AND octet_length(expression) <= 65536),
    scope_kind text NOT NULL CHECK (scope_kind IN ('person','dyad','group','community','channel')),
    scope_refs text[] NOT NULL DEFAULT '{}',
    context_scope text NULL,
    topic_scope text NULL,
    current_revision_id uuid NOT NULL,
    object_epoch bigint NOT NULL DEFAULT 1 CHECK (object_epoch > 0),
    acceptance_state text NOT NULL CHECK (acceptance_state IN ('accepted','withdrawn')),
    integrity_state text NOT NULL CHECK (integrity_state IN ('valid','revalidation_required')),
    suppression_state text NOT NULL CHECK (suppression_state IN ('normal','suppressed')),
    purge_state text NOT NULL CHECK (purge_state IN ('normal','purging')),
    created_at timestamptz NOT NULL,
    UNIQUE(subject_id, key)
);

CREATE TABLE language_convention_revisions (
    convention_revision_id uuid PRIMARY KEY,
    convention_id uuid NOT NULL REFERENCES language_conventions(convention_id) ON DELETE CASCADE,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    parent_revision_id uuid NULL REFERENCES language_convention_revisions(convention_revision_id),
    revision_intent text NULL CHECK (revision_intent IN ('correct','refine','reinterpret','evolve')),
    meaning text NOT NULL CHECK (length(trim(meaning)) > 0 AND octet_length(meaning) <= 65536),
    pragmatic_role text NULL CHECK (pragmatic_role IS NULL OR octet_length(pragmatic_role) <= 256),
    epistemic_class text NOT NULL,
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    formed_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    formation_policy_digest text NOT NULL CHECK (formation_policy_digest ~ '^[0-9a-f]{64}$'),
    UNIQUE(convention_id, revision_no),
    CHECK (valid_time_kind <> 'interval' OR valid_time_start IS NULL OR valid_time_end IS NULL OR valid_time_start < valid_time_end)
);
ALTER TABLE language_conventions
    ADD CONSTRAINT language_conventions_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES language_convention_revisions(convention_revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE language_convention_revision_supports (
    convention_revision_id uuid NOT NULL REFERENCES language_convention_revisions(convention_revision_id) ON DELETE CASCADE,
    support_kind text NOT NULL,
    support_ref text NOT NULL,
    support_role text NOT NULL,
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    seed_path text NULL,
    PRIMARY KEY(convention_revision_id, support_kind, support_ref, support_role),
    CHECK (num_nonnulls(source_region_id, derived_representation_id, derived_region_id) <= 1)
);

CREATE TABLE language_convention_formation_evidence (
    convention_revision_id uuid NOT NULL REFERENCES language_convention_revisions(convention_revision_id) ON DELETE CASCADE,
    support_ordinal integer NOT NULL CHECK (support_ordinal >= 0),
    evidence_kind text NOT NULL CHECK (evidence_kind IN ('seed_direct','explicit_explanation','explicit_confirmation','external_consistent_use','successful_understanding','repair_sequence','contextual')),
    external_actor_ref text NULL,
    PRIMARY KEY(convention_revision_id, support_ordinal)
);

CREATE INDEX relationship_assertions_subject_current_idx ON relationship_assertions(subject_id, acceptance_state, integrity_state, suppression_state, purge_state);
CREATE INDEX relationship_revisions_subject_recorded_idx ON relationship_revisions(subject_id, recorded_at DESC);
CREATE INDEX relationship_assertions_from_entity_idx ON relationship_assertions(subject_id, from_entity_ref) WHERE from_entity_ref IS NOT NULL;
CREATE INDEX relationship_assertions_to_entity_idx ON relationship_assertions(subject_id, to_entity_ref) WHERE to_entity_ref IS NOT NULL;
CREATE INDEX language_conventions_expression_idx ON language_conventions(subject_id, expression);
CREATE INDEX language_conventions_scope_idx ON language_conventions USING GIN(scope_refs);
CREATE INDEX language_convention_revisions_subject_recorded_idx ON language_convention_revisions(subject_id, recorded_at DESC);
