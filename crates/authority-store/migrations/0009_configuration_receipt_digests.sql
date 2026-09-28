-- Configuration receipts are pre-production replay records. Existing rows do
-- not contain the historical scoped digests required by the new contract, so
-- clear them instead of fabricating values.

TRUNCATE configuration_mutation_receipts;

ALTER TABLE configuration_mutation_receipts
    ADD COLUMN active_digest text NOT NULL CHECK (active_digest ~ '^[0-9a-f]{64}$'),
    ADD COLUMN desired_digest text NOT NULL CHECK (desired_digest ~ '^[0-9a-f]{64}$');
