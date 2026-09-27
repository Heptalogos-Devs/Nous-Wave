-- Memory Reference Profile R1 is a PRE_PRODUCTION clean replacement.
-- The earlier migrations remain historical; this migration defines the active
-- development schema and deliberately removes the superseded ontology.

DROP VIEW IF EXISTS memory_evidence_occurrences CASCADE;

DROP TABLE IF EXISTS
    serving_current,
    serving_generations,
    projection_watermarks,
    purged_use_receipts,
    cognitive_use_events,
    resident_refs,
    cognitive_sessions,
    mutation_receipts,
    cognition_dependency_invalidations,
    cognitive_schema_lineage,
    cognitive_schema_evidence_links,
    cognitive_schema_revisions,
    cognitive_schemas,
    association_evidence_supports,
    association_evidence,
    memory_revision_tags,
    memory_revision_aboutness,
    memory_revision_entities,
    memory_revision_dependencies,
    memory_revision_evidence,
    memory_revision_relations,
    memory_revisions,
    memory_objects,
    anchor_support,
    anchor_revisions,
    anchors,
    tag_revisions,
    tags
    CASCADE;

ALTER TABLE subjects RENAME COLUMN state_revision TO authority_seq;

ALTER TABLE observation_occurrences
    ADD COLUMN occurred_time_kind text NOT NULL DEFAULT 'unknown'
        CHECK (occurred_time_kind IN ('unknown','instant','interval')),
    ADD COLUMN occurred_time_start timestamptz NULL,
    ADD COLUMN occurred_time_end timestamptz NULL;
UPDATE observation_occurrences
SET occurred_time_kind = 'instant', occurred_time_start = occurred_at
WHERE occurred_at IS NOT NULL;
ALTER TABLE observation_occurrences DROP COLUMN occurred_at;
ALTER TABLE observation_occurrences
    ADD CONSTRAINT observation_occurrences_interval_check
    CHECK (occurred_time_kind <> 'interval'
        OR occurred_time_start IS NULL
        OR occurred_time_end IS NULL
        OR occurred_time_start < occurred_time_end);

CREATE TABLE memory_objects (
    memory_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    cognitive_role text NOT NULL CHECK (cognitive_role IN ('experiential','declarative','procedural_experience')),
    current_revision_id uuid NOT NULL,
    object_epoch bigint NOT NULL DEFAULT 1 CHECK (object_epoch > 0),
    acceptance_state text NOT NULL CHECK (acceptance_state IN ('accepted','withdrawn')),
    integrity_state text NOT NULL CHECK (integrity_state IN ('valid','revalidation_required')),
    suppression_state text NOT NULL CHECK (suppression_state IN ('normal','suppressed')),
    purge_state text NOT NULL CHECK (purge_state IN ('normal','purging')),
    accessibility_mode text NOT NULL DEFAULT 'auto' CHECK (accessibility_mode IN ('auto','normal','deep','explicit')),
    created_at timestamptz NOT NULL
);

CREATE TABLE memory_revisions (
    memory_revision_id uuid PRIMARY KEY,
    memory_id uuid NOT NULL REFERENCES memory_objects(memory_id) ON DELETE CASCADE,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    parent_revision_id uuid NULL REFERENCES memory_revisions(memory_revision_id),
    revision_intent text NULL CHECK (revision_intent IN ('correct','rephrase','reinterpret')),
    formation_mode text NOT NULL CHECK (formation_mode IN ('grounded','synthesized')),
    grounding_occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    semantic_role text NOT NULL,
    title text NULL,
    representation_text text NOT NULL CHECK (length(trim(representation_text)) > 0 AND octet_length(representation_text) <= 1048576),
    epistemic_class text NOT NULL,
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    formed_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    UNIQUE(memory_id, revision_no),
    CHECK (valid_time_kind <> 'interval'
        OR valid_time_start IS NULL
        OR valid_time_end IS NULL
        OR valid_time_start < valid_time_end),
    CHECK ((formation_mode = 'grounded') = (grounding_occurrence_id IS NOT NULL))
);
ALTER TABLE memory_objects
    ADD CONSTRAINT memory_objects_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES memory_revisions(memory_revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE memory_revision_evidence (
    memory_revision_id uuid NOT NULL REFERENCES memory_revisions(memory_revision_id) ON DELETE CASCADE,
    occurrence_id uuid NOT NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    evidence_no integer NOT NULL CHECK (evidence_no >= 0),
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    support_role text NOT NULL CHECK (support_role IN ('direct','corroborating','interpretation','contradiction','contextual')),
    PRIMARY KEY(memory_revision_id, evidence_no),
    CHECK (num_nonnulls(source_region_id, derived_representation_id, derived_region_id) <= 1)
);

CREATE TABLE memory_revision_dependencies (
    memory_revision_id uuid NOT NULL REFERENCES memory_revisions(memory_revision_id) ON DELETE CASCADE,
    target_ref_kind text NOT NULL CHECK (target_ref_kind IN ('memory_revision','cognitive_schema_revision')),
    target_ref text NOT NULL,
    support_role text NOT NULL CHECK (support_role IN ('direct','corroborating','interpretation','contradiction','contextual')),
    PRIMARY KEY(memory_revision_id, target_ref_kind, target_ref, support_role)
);

CREATE TABLE memory_revision_aboutness (
    memory_revision_id uuid NOT NULL REFERENCES memory_revisions(memory_revision_id) ON DELETE CASCADE,
    entity_ref text NOT NULL,
    PRIMARY KEY(memory_revision_id, entity_ref)
);

CREATE TABLE memory_revision_relations (
    from_revision_id uuid NOT NULL REFERENCES memory_revisions(memory_revision_id) ON DELETE CASCADE,
    to_revision_id uuid NOT NULL REFERENCES memory_revisions(memory_revision_id) ON DELETE CASCADE,
    relation text NOT NULL CHECK (relation IN ('derived_from','contradicts','temporal_successor','replaces_basis','elaborates')),
    created_at timestamptz NOT NULL,
    PRIMARY KEY(from_revision_id, to_revision_id, relation)
);

CREATE TABLE tags (
    tag_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    current_revision_id uuid NOT NULL,
    created_at timestamptz NOT NULL,
    status text NOT NULL CHECK (status IN ('active','withdrawn'))
);
CREATE TABLE tag_revisions (
    tag_revision_id uuid PRIMARY KEY,
    tag_id uuid NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    label text NOT NULL,
    description text NULL,
    kind_hint text NULL,
    origin text NOT NULL,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    created_at timestamptz NOT NULL,
    UNIQUE(tag_id, revision_no)
);
ALTER TABLE tags ADD CONSTRAINT tags_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES tag_revisions(tag_revision_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE memory_revision_tags (
    memory_revision_id uuid NOT NULL REFERENCES memory_revisions(memory_revision_id) ON DELETE CASCADE,
    tag_id uuid NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    PRIMARY KEY(memory_revision_id, tag_id)
);

CREATE TABLE cognitive_schemas (
    schema_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    current_revision_id uuid NOT NULL,
    object_epoch bigint NOT NULL DEFAULT 1 CHECK (object_epoch > 0),
    acceptance_state text NOT NULL CHECK (acceptance_state IN ('accepted','withdrawn')),
    integrity_state text NOT NULL CHECK (integrity_state IN ('valid','revalidation_required')),
    suppression_state text NOT NULL CHECK (suppression_state IN ('normal','suppressed')),
    purge_state text NOT NULL CHECK (purge_state IN ('normal','purging')),
    created_at timestamptz NOT NULL
);
CREATE TABLE cognitive_schema_revisions (
    schema_revision_id uuid PRIMARY KEY,
    schema_id uuid NOT NULL REFERENCES cognitive_schemas(schema_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    parent_revision_id uuid NULL REFERENCES cognitive_schema_revisions(schema_revision_id),
    revision_intent text NULL CHECK (revision_intent IN ('correct','rephrase','reinterpret')),
    title text NULL,
    structural_claim text NOT NULL CHECK (octet_length(structural_claim) <= 16384),
    applicability_description text NOT NULL CHECK (octet_length(applicability_description) <= 16384),
    aboutness text[] NOT NULL DEFAULT '{}',
    tags uuid[] NOT NULL DEFAULT '{}',
    boundary_definition text NOT NULL CHECK (octet_length(boundary_definition) <= 16384),
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    formed_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    UNIQUE(schema_id, revision_no),
    CHECK (valid_time_kind <> 'interval'
        OR valid_time_start IS NULL
        OR valid_time_end IS NULL
        OR valid_time_start < valid_time_end)
);
ALTER TABLE cognitive_schemas
    ADD CONSTRAINT cognitive_schemas_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES cognitive_schema_revisions(schema_revision_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE cognitive_schema_evidence_links (
    link_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    schema_revision_id uuid NOT NULL REFERENCES cognitive_schema_revisions(schema_revision_id) ON DELETE CASCADE,
    role text NOT NULL CHECK (role IN ('support','counterexample','boundary_case')),
    support_kind text NOT NULL CHECK (support_kind IN ('evidence','memory_revision','cognitive_schema_revision')),
    support_ref text NOT NULL,
    support_role text NOT NULL CHECK (support_role IN ('direct','corroborating','interpretation','contradiction','contextual')),
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    created_at timestamptz NOT NULL,
    revoked_at timestamptz NULL
);
CREATE TABLE cognitive_schema_lineage (
    from_revision_id uuid NOT NULL REFERENCES cognitive_schema_revisions(schema_revision_id) ON DELETE CASCADE,
    to_revision_id uuid NOT NULL REFERENCES cognitive_schema_revisions(schema_revision_id) ON DELETE CASCADE,
    relation text NOT NULL CHECK (relation IN ('schema_split_from','schema_merged_from')),
    PRIMARY KEY(from_revision_id, to_revision_id, relation)
);

CREATE TABLE association_evidence (
    association_evidence_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    from_ref_kind text NOT NULL,
    from_ref text NOT NULL,
    to_ref_kind text NOT NULL,
    to_ref text NOT NULL,
    relation_kind text NOT NULL CHECK (length(relation_kind) BETWEEN 1 AND 128 AND relation_kind ~ '^[a-z0-9_.:-]+$'),
    polarity text NOT NULL CHECK (polarity IN ('positive','negative')),
    support_class text NOT NULL CHECK (support_class IN ('host_explicit','source_evidence','cognitive_derivation','meaningful_use','derived_structure')),
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    created_at timestamptz NOT NULL,
    revoked_at timestamptz NULL
);
CREATE TABLE association_evidence_supports (
    association_evidence_id uuid NOT NULL REFERENCES association_evidence(association_evidence_id) ON DELETE CASCADE,
    support_kind text NOT NULL CHECK (support_kind IN ('evidence','memory_revision','cognitive_schema_revision')),
    support_ref text NOT NULL,
    support_role text NOT NULL CHECK (support_role IN ('direct','corroborating','interpretation','contradiction','contextual')),
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    PRIMARY KEY(association_evidence_id, support_kind, support_ref, support_role),
    CHECK (num_nonnulls(source_region_id, derived_representation_id, derived_region_id) <= 1)
);

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

DROP TABLE IF EXISTS serving_current, serving_generations, projection_watermarks CASCADE;
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

CREATE INDEX memory_revisions_subject_time_idx ON memory_revisions(subject_id, recorded_at DESC);
CREATE INDEX memory_revision_evidence_occurrence_idx ON memory_revision_evidence(occurrence_id);
CREATE INDEX resident_refs_session_state_idx ON resident_refs(session_id, state, last_meaningful_use_at DESC);
