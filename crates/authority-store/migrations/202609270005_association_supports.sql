ALTER TABLE association_evidence_supports
    DROP CONSTRAINT IF EXISTS association_evidence_supports_support_kind_check;

ALTER TABLE association_evidence_supports
    ADD CONSTRAINT association_evidence_supports_support_kind_check
    CHECK (support_kind IN ('evidence','memory_revision','cognitive_schema_revision','use_event'));
