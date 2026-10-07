-- Copyright 2026 Aravine Zhu
-- SPDX-License-Identifier: Apache-2.0
-- Formation lineage records the input use; epistemic judgments are separate.
ALTER TABLE association_evidence_supports RENAME TO association_evidence_basis;
ALTER TABLE episode_revision_supports RENAME TO episode_revision_basis;
ALTER TABLE journal_point_supports RENAME TO journal_point_basis;
ALTER TABLE journal_point_basis RENAME COLUMN support_no TO basis_no;
ALTER TABLE journal_point_basis RENAME COLUMN support TO basis;
ALTER TABLE association_evidence RENAME COLUMN support_class TO basis_class;
ALTER TABLE tag_lineage RENAME COLUMN supports TO basis;
DO $$
DECLARE owner text; item record;
BEGIN
  FOREACH owner IN ARRAY ARRAY['memory_revision_evidence','memory_revision_dependencies','cognitive_schema_evidence_links','association_evidence_basis','episode_revision_basis'] LOOP
    FOR item IN SELECT conname FROM pg_constraint WHERE conrelid=owner::regclass AND contype='c' AND pg_get_constraintdef(oid) LIKE '%support_role%' LOOP
      EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',owner,item.conname);
    END LOOP;
    EXECUTE format('ALTER TABLE %I RENAME COLUMN support_role TO basis_role',owner);
    EXECUTE format('ALTER TABLE %I ADD COLUMN epistemic_relation text NOT NULL DEFAULT '''' CHECK (epistemic_relation IN ('''',''supports'',''contradicts'',''corroborates'',''weakens'',''corrects'',''counterexample'',''inferred_from''))',owner);
    EXECUTE format('UPDATE %I SET epistemic_relation=CASE basis_role WHEN ''corroborating'' THEN ''corroborates'' WHEN ''contradiction'' THEN ''contradicts'' ELSE '''' END, basis_role=CASE basis_role WHEN ''corroborating'' THEN ''direct'' WHEN ''contradiction'' THEN ''contextual'' ELSE basis_role END',owner);
    EXECUTE format('ALTER TABLE %I ADD CHECK (basis_role IN (''direct'',''interpretation'',''contextual''))',owner);
  END LOOP;
  FOREACH owner IN ARRAY ARRAY['cognitive_schema_evidence_links','association_evidence_basis','episode_revision_basis'] LOOP
    EXECUTE format('ALTER TABLE %I RENAME COLUMN support_kind TO basis_kind',owner);
    EXECUTE format('ALTER TABLE %I RENAME COLUMN support_ref TO basis_ref',owner);
  END LOOP;
END $$;
ALTER TABLE episode_revision_basis RENAME COLUMN support_no TO basis_no;
ALTER TABLE memory_revision_dependencies DROP CONSTRAINT memory_revision_dependencies_pkey;
ALTER TABLE memory_revision_dependencies ADD PRIMARY KEY(memory_revision_id,target_ref_kind,target_ref,basis_role,epistemic_relation);
ALTER TABLE association_evidence_basis DROP CONSTRAINT association_evidence_supports_pkey;
ALTER TABLE association_evidence_basis ADD PRIMARY KEY(association_evidence_id,basis_kind,basis_ref,basis_role,epistemic_relation);

-- Upgrade the known owner payloads once. Ordinary claim text and judgment values remain intact.
CREATE FUNCTION rebase_formation_payload(input jsonb, material boolean DEFAULT false) RETURNS jsonb LANGUAGE plpgsql AS $$
DECLARE result jsonb; entry record; key text; role text;
BEGIN
  IF input IS NULL THEN RETURN NULL; END IF;
  IF jsonb_typeof(input)='array' THEN
    SELECT COALESCE(jsonb_agg(rebase_formation_payload(value,material)),'[]'::jsonb) INTO result FROM jsonb_array_elements(input);
    RETURN result;
  ELSIF jsonb_typeof(input)<>'object' THEN RETURN input;
  END IF;
  result := '{}'::jsonb;
  FOR entry IN SELECT * FROM jsonb_each(input) LOOP
    key := CASE entry.key WHEN 'supports' THEN CASE WHEN material OR input->>'basis' IN ('direct','inferred') THEN 'basis_refs' ELSE 'basis' END
      WHEN 'support' THEN 'basis' WHEN 'support_role' THEN 'basis_role' WHEN 'supportRole' THEN 'basisRole'
      WHEN 'support_class' THEN 'basis_class' WHEN 'supportClass' THEN 'basisClass'
      WHEN 'supportKey' THEN 'basisKey' WHEN 'support_key' THEN 'basis_key'
      WHEN 'support_keys' THEN 'basis_keys' WHEN 'supportKeys' THEN 'basisKeys' ELSE entry.key END;
    result := result || jsonb_build_object(key,rebase_formation_payload(entry.value,material));
  END LOOP;
  role := COALESCE(result->>'basis_role',result->>'basisRole');
  IF role IN ('contradiction','corroborating') THEN
    key := CASE WHEN result ? 'basis_role' THEN 'basis_role' ELSE 'basisRole' END;
    result := result || jsonb_build_object(key,CASE role WHEN 'contradiction' THEN 'contextual' ELSE 'direct' END,
      CASE WHEN key='basis_role' THEN 'epistemic_relation' ELSE 'epistemicRelation' END,
      CASE role WHEN 'contradiction' THEN 'contradicts' ELSE 'corroborates' END);
  END IF;
  RETURN result;
END $$;
UPDATE journal_point_basis SET basis=rebase_formation_payload(basis);
UPDATE tag_lineage SET basis=rebase_formation_payload(basis);
UPDATE derived_representations SET payload_json=rebase_formation_payload(payload_json,true) WHERE payload_json IS NOT NULL;
UPDATE authority_object_states SET state=rebase_formation_payload(state);
UPDATE model_workflow_operations SET snapshot=rebase_formation_payload(snapshot),proposal=rebase_formation_payload(proposal),outcome=rebase_formation_payload(outcome);
DROP FUNCTION rebase_formation_payload(jsonb,boolean);
