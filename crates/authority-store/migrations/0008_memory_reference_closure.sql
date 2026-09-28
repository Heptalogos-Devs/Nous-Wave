-- R2 Memory Reference Profile closure: runtime reference vocabulary.

ALTER TABLE resident_refs
    DROP CONSTRAINT resident_refs_ref_kind_check;

ALTER TABLE resident_refs
    ADD CONSTRAINT resident_refs_ref_kind_format_check
    CHECK (ref_kind ~ '^[a-z][a-z0-9_]{0,63}$');
