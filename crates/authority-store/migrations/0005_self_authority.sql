-- Cognitive Seed and Self Authority are part of the canonical fresh schema.

CREATE TABLE cognitive_seed_versions (
    seed_version_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    artifact_id uuid NOT NULL REFERENCES artifacts(artifact_id) ON DELETE RESTRICT,
    format text NOT NULL CHECK (format = 'application/vnd.nous-wave.cognitive-seed+toml;version=1'),
    provenance jsonb NOT NULL DEFAULT '{}',
    content_hash text NOT NULL CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL,
    UNIQUE(subject_id, content_hash, format)
);

CREATE TABLE subject_seed_adoptions (
    adoption_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    seed_version_id uuid NOT NULL REFERENCES cognitive_seed_versions(seed_version_id) ON DELETE RESTRICT,
    kind text NOT NULL CHECK (kind IN ('initial','import')),
    operation_id uuid NOT NULL,
    request_digest text NOT NULL CHECK (request_digest ~ '^[0-9a-f]{64}$'),
    adopted_at timestamptz NOT NULL,
    UNIQUE(subject_id, operation_id),
    UNIQUE(subject_id, seed_version_id, kind)
);

CREATE TABLE self_facets (
    self_facet_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('identity','role','capability','limitation','tendency','value','preference')),
    key text NOT NULL CHECK (length(trim(key)) > 0 AND octet_length(key) <= 256 AND key !~ '[[:space:]]'),
    current_revision_id uuid NOT NULL,
    object_epoch bigint NOT NULL DEFAULT 1 CHECK (object_epoch > 0),
    acceptance_state text NOT NULL CHECK (acceptance_state IN ('accepted','withdrawn')),
    integrity_state text NOT NULL CHECK (integrity_state IN ('valid','revalidation_required')),
    suppression_state text NOT NULL CHECK (suppression_state IN ('normal','suppressed')),
    purge_state text NOT NULL CHECK (purge_state IN ('normal','purging')),
    created_at timestamptz NOT NULL,
    UNIQUE(subject_id, kind, key)
);

CREATE TABLE self_facet_revisions (
    self_facet_revision_id uuid PRIMARY KEY,
    self_facet_id uuid NOT NULL REFERENCES self_facets(self_facet_id) ON DELETE CASCADE,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    parent_revision_id uuid NULL REFERENCES self_facet_revisions(self_facet_revision_id),
    revision_intent text NULL CHECK (revision_intent IN ('correct','refine','reinterpret','evolve')),
    statement text NOT NULL CHECK (length(trim(statement)) > 0 AND octet_length(statement) <= 65536),
    scope text NOT NULL CHECK (length(trim(scope)) > 0 AND octet_length(scope) <= 256),
    epistemic_class text NOT NULL,
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    formed_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    UNIQUE(self_facet_id, revision_no),
    CHECK (valid_time_kind <> 'interval' OR valid_time_start IS NULL OR valid_time_end IS NULL OR valid_time_start < valid_time_end)
);
ALTER TABLE self_facets
    ADD CONSTRAINT self_facets_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES self_facet_revisions(self_facet_revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE self_facet_revision_supports (
    self_facet_revision_id uuid NOT NULL REFERENCES self_facet_revisions(self_facet_revision_id) ON DELETE CASCADE,
    support_kind text NOT NULL CHECK (support_kind IN ('evidence','cognitive_seed_version','memory_revision','cognitive_schema_revision','self_facet_revision','narrative_identity_revision')),
    support_ref text NOT NULL,
    support_role text NOT NULL CHECK (support_role IN ('direct','corroborating','interpretation','contradiction','contextual')),
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    PRIMARY KEY(self_facet_revision_id, support_kind, support_ref, support_role),
    CHECK (num_nonnulls(source_region_id, derived_representation_id, derived_region_id) <= 1)
);

CREATE TABLE narrative_identities (
    narrative_identity_id uuid PRIMARY KEY,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    key text NOT NULL CHECK (length(trim(key)) > 0 AND octet_length(key) <= 256 AND key !~ '[[:space:]]'),
    current_revision_id uuid NOT NULL,
    object_epoch bigint NOT NULL DEFAULT 1 CHECK (object_epoch > 0),
    acceptance_state text NOT NULL CHECK (acceptance_state IN ('accepted','withdrawn')),
    integrity_state text NOT NULL CHECK (integrity_state IN ('valid','revalidation_required')),
    suppression_state text NOT NULL CHECK (suppression_state IN ('normal','suppressed')),
    purge_state text NOT NULL CHECK (purge_state IN ('normal','purging')),
    created_at timestamptz NOT NULL,
    UNIQUE(subject_id, key)
);

CREATE TABLE narrative_identity_revisions (
    narrative_identity_revision_id uuid PRIMARY KEY,
    narrative_identity_id uuid NOT NULL REFERENCES narrative_identities(narrative_identity_id) ON DELETE CASCADE,
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    revision_no integer NOT NULL CHECK (revision_no > 0),
    parent_revision_id uuid NULL REFERENCES narrative_identity_revisions(narrative_identity_revision_id),
    revision_intent text NULL CHECK (revision_intent IN ('correct','refine','reinterpret','evolve')),
    text text NOT NULL CHECK (length(trim(text)) > 0 AND octet_length(text) <= 65536),
    valid_time_kind text NOT NULL DEFAULT 'unknown' CHECK (valid_time_kind IN ('unknown','instant','interval')),
    valid_time_start timestamptz NULL,
    valid_time_end timestamptz NULL,
    formed_at timestamptz NOT NULL,
    recorded_at timestamptz NOT NULL,
    producer_signature_id uuid NULL REFERENCES producer_signatures(producer_signature_id),
    UNIQUE(narrative_identity_id, revision_no),
    CHECK (valid_time_kind <> 'interval' OR valid_time_start IS NULL OR valid_time_end IS NULL OR valid_time_start < valid_time_end)
);
ALTER TABLE narrative_identities
    ADD CONSTRAINT narrative_identities_current_revision_fk
    FOREIGN KEY (current_revision_id) REFERENCES narrative_identity_revisions(narrative_identity_revision_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE narrative_identity_revision_supports (
    narrative_identity_revision_id uuid NOT NULL REFERENCES narrative_identity_revisions(narrative_identity_revision_id) ON DELETE CASCADE,
    support_kind text NOT NULL CHECK (support_kind IN ('evidence','cognitive_seed_version','memory_revision','cognitive_schema_revision','self_facet_revision','narrative_identity_revision')),
    support_ref text NOT NULL,
    support_role text NOT NULL CHECK (support_role IN ('direct','corroborating','interpretation','contradiction','contextual')),
    occurrence_id uuid NULL REFERENCES observation_occurrences(occurrence_id) ON DELETE RESTRICT,
    source_region_id uuid NULL REFERENCES source_regions(source_region_id) ON DELETE RESTRICT,
    derived_representation_id uuid NULL REFERENCES derived_representations(derived_representation_id) ON DELETE RESTRICT,
    derived_region_id uuid NULL REFERENCES derived_regions(derived_region_id) ON DELETE RESTRICT,
    PRIMARY KEY(narrative_identity_revision_id, support_kind, support_ref, support_role),
    CHECK (num_nonnulls(source_region_id, derived_representation_id, derived_region_id) <= 1)
);

CREATE TABLE narrative_references (
    narrative_identity_revision_id uuid NOT NULL REFERENCES narrative_identity_revisions(narrative_identity_revision_id) ON DELETE CASCADE,
    position integer NOT NULL CHECK (position >= 0),
    target_ref_kind text NOT NULL CHECK (target_ref_kind IN ('memory_revision','self_facet_revision')),
    target_ref text NOT NULL,
    role text NOT NULL CHECK (length(trim(role)) > 0 AND octet_length(role) <= 256),
    PRIMARY KEY(narrative_identity_revision_id, position)
);

CREATE TABLE self_purge_receipts (
    subject_id uuid NOT NULL REFERENCES subjects(subject_id) ON DELETE CASCADE,
    operation_id uuid NOT NULL,
    ref_kind text NOT NULL,
    ref_value text NOT NULL,
    request_digest text NOT NULL CHECK (request_digest ~ '^[0-9a-f]{64}$'),
    purged_at timestamptz NOT NULL,
    PRIMARY KEY(subject_id, operation_id),
    UNIQUE(subject_id, ref_kind, ref_value)
);

CREATE INDEX self_facets_subject_lifecycle_idx ON self_facets(subject_id, acceptance_state, integrity_state, suppression_state, purge_state, kind, key);
CREATE INDEX self_facet_revisions_subject_recorded_idx ON self_facet_revisions(subject_id, recorded_at DESC);
CREATE INDEX narrative_identities_subject_lifecycle_idx ON narrative_identities(subject_id, acceptance_state, integrity_state, suppression_state, purge_state, key);
CREATE INDEX narrative_identity_revisions_subject_recorded_idx ON narrative_identity_revisions(subject_id, recorded_at DESC);
