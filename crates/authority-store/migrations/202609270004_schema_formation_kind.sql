ALTER TABLE cognitive_schema_revisions
    ADD COLUMN formation_kind text NOT NULL DEFAULT 'explicit_import'
        CHECK (formation_kind IN ('explicit_import','synthesized'));
