-- Copyright 2026 Aravine Zhu
-- SPDX-License-Identifier: Apache-2.0
ALTER TABLE producer_signatures
 ADD COLUMN model_role text,
 ADD COLUMN model_profile text,
 ADD COLUMN execution_profile text,
 ADD COLUMN inference_controls_digest text,
 ADD COLUMN role_policy_digest text,
 ADD COLUMN prompt_id text,
 ADD COLUMN prompt_digest text;
ALTER TABLE model_workflow_operations ADD COLUMN execution_telemetry jsonb;
