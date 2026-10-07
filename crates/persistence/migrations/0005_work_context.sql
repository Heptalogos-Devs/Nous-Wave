-- Copyright 2026 Aravine Zhu
-- SPDX-License-Identifier: Apache-2.0
-- Durable payload is larger than any query projection; projection is separately bounded.
ALTER TABLE work_contexts ADD COLUMN context_text text NOT NULL DEFAULT '' CHECK (octet_length(context_text) <= 65536);
CREATE TABLE work_context_anchors (
    work_context_id uuid NOT NULL REFERENCES work_contexts(work_context_id) ON DELETE CASCADE,
    ordinal integer NOT NULL CHECK (ordinal >= 0),
    ref_kind text NOT NULL CHECK (ref_kind IN ('entity','tag')),
    ref_value text NOT NULL,
    PRIMARY KEY(work_context_id, ordinal),
    UNIQUE(work_context_id, ref_kind, ref_value)
);
