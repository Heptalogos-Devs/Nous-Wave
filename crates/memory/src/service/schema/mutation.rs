// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl MemoryService {
    pub async fn create_schema(&self, mut input: CreateSchemaInput) -> Result<SchemaView> {
        input.producer = canonical_schema_producer(input.producer.as_ref())?;
        self.store.require_subject(input.subject).await?;
        let digest = operation_digest(
            "create_cognitive_schema",
            input.subject,
            &serde_json::json!({"title":input.title,"structural_claim":input.structural_claim,"scope":input.applicability_scope,"boundary_definition":input.boundary_definition,"formation_kind":input.formation_kind,"evidence_links":input.evidence_links,"producer":input.producer}),
        )?;
        let mut mutation = match self
            .start_mutation(
                input.subject,
                input.operation_id,
                "create_cognitive_schema",
                &digest,
            )
            .await?
        {
            MutationStart::Replay(receipt) => {
                let id = receipt.result_ref;
                if receipt.state == "committed" {
                    return self
                        .schema_at(
                            input.subject,
                            CognitiveSchemaId(
                                id.ok_or_else(|| {
                                    Error::Infrastructure("schema receipt missing result".into())
                                })?
                                .parse()
                                .map_err(|_| {
                                    Error::Infrastructure("invalid schema receipt".into())
                                })?,
                            ),
                            receipt.result_revision.map(CognitiveSchemaRevisionId),
                        )
                        .await;
                }
                return Err(Error::Unavailable(
                    "schema operation is already in progress".into(),
                ));
            }
            MutationStart::Active(mutation) => mutation,
        };
        self.validate_schema_formation(&input).await?;
        let producer = if let Some(value) = &input.producer {
            Some(AuthorityStore::register_producer_in(mutation.tx(), value).await?)
        } else {
            None
        };
        let (schema_id, revision_id) = self
            .create_schema_in(
                mutation.tx(),
                &input,
                producer,
                self.cognition.now(input.subject),
            )
            .await?;
        let sequence = mutation.invalidate(ProjectionInvalidation::all()).await?;
        self.enqueue_concept_in(
            mutation.tx(),
            input.subject,
            CognitiveRef::CognitiveSchemaRevision(revision_id),
            sequence,
        )
        .await?;
        mutation
            .commit(
                "schema",
                Some(&schema_id.0.to_string()),
                Some(revision_id.0),
                Some(1),
            )
            .await?;
        self.schema(input.subject, schema_id).await
    }

    pub(in crate::service) async fn insert_schema_link(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        revision: CognitiveSchemaRevisionId,
        input: SchemaEvidenceLinkInput,
    ) -> Result<()> {
        let id = SchemaEvidenceLinkId::new();
        let epistemic_relation = input.basis.epistemic_relation();
        let (
            kind,
            value,
            basis_role,
            occurrence,
            source_region,
            derived_representation,
            derived_region,
        ) = match input.basis {
            RevisionBasis::Evidence(e) => {
                let (sr, dr, drr) = match e.locator {
                    EvidenceLocator::WholeOccurrence => (None, None, None),
                    EvidenceLocator::SourceRegion(id) => (Some(id.0), None, None),
                    EvidenceLocator::DerivedRepresentation(id) => (None, Some(id.0), None),
                    EvidenceLocator::DerivedRegion(id) => (None, None, Some(id.0)),
                };
                (
                    "evidence".to_owned(),
                    e.canonical_key(),
                    e.basis_role.as_str().to_owned(),
                    Some(e.occurrence_id.0),
                    sr,
                    dr,
                    drr,
                )
            }
            RevisionBasis::CognitionDependency(d) => {
                let (k, v) = reference_parts(&d.target_revision);
                (
                    k,
                    v,
                    d.basis_role.as_str().to_owned(),
                    None,
                    None,
                    None,
                    None,
                )
            }
            RevisionBasis::Seed(_) => {
                return Err(Error::Invalid(
                    "CognitiveSchema cannot use Cognitive Seed support".into(),
                ));
            }
        };
        sqlx::query("INSERT INTO cognitive_schema_evidence_links(link_id,subject_id,schema_revision_id,role,basis_kind,basis_ref,basis_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,created_at,epistemic_relation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)").bind(id.0).bind(subject.0).bind(revision.0).bind(input.role.as_str()).bind(kind).bind(value).bind(basis_role).bind(occurrence).bind(source_region).bind(derived_representation).bind(derived_region).bind(self.cognition.now(subject)).bind(epistemic_relation_text(epistemic_relation)).execute(&mut **tx).await.map_err(db)?;
        Ok(())
    }

    pub async fn add_schema_evidence(
        &self,
        subject: SubjectId,
        schema_id: CognitiveSchemaId,
        operation_id: OperationId,
        expected_object_epoch: i64,
        link: SchemaEvidenceLinkInput,
    ) -> Result<SchemaView> {
        self.store.require_subject(subject).await?;
        self.validate_basis_for_subject(subject, std::slice::from_ref(&link.basis))
            .await?;
        let digest = operation_digest(
            "add_schema_evidence",
            subject,
            &serde_json::json!({"schema_id":schema_id,"expected_object_epoch":expected_object_epoch,"link":link}),
        )?;
        let mut mutation = match self
            .start_mutation(subject, operation_id, "add_schema_evidence", &digest)
            .await?
        {
            MutationStart::Replay(receipt) => {
                if receipt.state == "committed" {
                    return self.schema(subject, schema_id).await;
                }
                return Err(Error::Unavailable(
                    "schema evidence operation is already in progress".into(),
                ));
            }
            MutationStart::Active(mutation) => mutation,
        };
        let row=sqlx::query("SELECT current_revision_id,object_epoch FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE").bind(subject.0).bind(schema_id.0).fetch_optional(&mut **mutation.tx()).await.map_err(db)?.ok_or_else(||Error::NotFound("CognitiveSchema not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != expected_object_epoch {
            return Err(Error::Conflict(
                "expected schema object epoch is stale".into(),
            ));
        }
        self.validate_basis_in_tx(mutation.tx(), subject, std::slice::from_ref(&link.basis))
            .await?;
        self.insert_schema_link(
            mutation.tx(),
            subject,
            CognitiveSchemaRevisionId(row.try_get("current_revision_id").map_err(db)?),
            link,
        )
        .await?;
        sqlx::query("UPDATE cognitive_schemas SET object_epoch=object_epoch+1 WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(schema_id.0).execute(&mut **mutation.tx()).await.map_err(db)?;
        let sequence = mutation
            .invalidate(ProjectionInvalidation::topology())
            .await?;
        self.invalidate_object_dependents_in(
            mutation.tx(),
            subject,
            "schema",
            &[schema_id.0],
            sequence,
            "source_basis_changed",
        )
        .await?;
        mutation
            .commit(
                "schema",
                Some(&schema_id.0.to_string()),
                None,
                Some(epoch + 1),
            )
            .await?;
        self.schema(subject, schema_id).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "schema content revision owns one atomic Authority transaction"
    )]
    pub async fn revise_schema(&self, mut input: ReviseSchemaInput) -> Result<SchemaView> {
        self.store.require_subject(input.subject).await?;
        input.producer = canonical_schema_producer(input.producer.as_ref())?;
        validate_schema_revision_content(&input)?;
        let digest = operation_digest(
            "revise_cognitive_schema",
            input.subject,
            &serde_json::json!({
                "schema_id": input.schema_id,
                "expected_object_epoch": input.expected_object_epoch,
                "intent": input.intent,
                "title": input.title,
                "structural_claim": input.structural_claim,
                "applicability_scope": input.applicability_scope,
                "boundary_definition": input.boundary_definition,
                "copy_link_ids": input.copy_link_ids,
                "evidence_links": input.evidence_links,
                "producer": input.producer,
                "formation_kind": input.formation_kind,
            }),
        )?;
        let mut mutation = match self
            .start_mutation(
                input.subject,
                input.operation_id,
                "revise_cognitive_schema",
                &digest,
            )
            .await?
        {
            MutationStart::Replay(receipt) => {
                if receipt.state == "committed" {
                    return self
                        .schema_at(
                            input.subject,
                            input.schema_id,
                            receipt.result_revision.map(CognitiveSchemaRevisionId),
                        )
                        .await;
                }
                return Err(Error::Unavailable(
                    "schema revision operation is already in progress".into(),
                ));
            }
            MutationStart::Active(mutation) => mutation,
        };
        let row = sqlx::query(
            "SELECT current_revision_id,object_epoch FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE",
        )
        .bind(input.subject.0)
        .bind(input.schema_id.0)
        .fetch_optional(&mut **mutation.tx())
        .await
        .map_err(db)?
        .ok_or_else(|| Error::NotFound("CognitiveSchema not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(Error::Conflict(
                "expected schema object epoch is stale".into(),
            ));
        }
        let parent = CognitiveSchemaRevisionId(row.try_get("current_revision_id").map_err(db)?);
        let revision_no: i32 = sqlx::query_scalar(
            "SELECT revision_no+1 FROM cognitive_schema_revisions WHERE schema_revision_id=$1",
        )
        .bind(parent.0)
        .fetch_one(&mut **mutation.tx())
        .await
        .map_err(db)?;
        let mut copied_links = input.evidence_links.clone();
        for link_id in &input.copy_link_ids {
            let link = sqlx::query("SELECT schema_revision_id,role,basis_kind,basis_ref,basis_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,epistemic_relation FROM cognitive_schema_evidence_links WHERE link_id=$1 AND subject_id=$2 AND revoked_at IS NULL")
                .bind(link_id.0)
                .bind(input.subject.0)
                .fetch_optional(&mut **mutation.tx())
                .await
                .map_err(db)?
                .ok_or_else(|| Error::Invalid("schema copy link is not active in Subject".into()))?;
            let source_revision =
                CognitiveSchemaRevisionId(link.try_get("schema_revision_id").map_err(db)?);
            if source_revision != parent {
                return Err(Error::Invalid(
                    "schema copy link must belong to the current parent revision".into(),
                ));
            }
            copied_links.push(decode_schema_link_input(link)?);
        }
        let copied_basis = copied_links
            .iter()
            .map(|link| link.basis.clone())
            .collect::<Vec<_>>();
        self.validate_object_dependency_cycle(
            input.subject,
            &format!("schema:{}", input.schema_id.0),
            &copied_basis,
        )
        .await?;
        self.validate_basis_in_tx(mutation.tx(), input.subject, &copied_basis)
            .await?;
        let revision_id = CognitiveSchemaRevisionId::new();
        let parent_scope = sqlx::query(
            "SELECT formation_kind,aboutness FROM cognitive_schema_revisions WHERE schema_revision_id=$1",
        )
        .bind(parent.0)
        .fetch_one(&mut **mutation.tx())
        .await
        .map_err(db)?;
        let formation_kind: String = parent_scope.try_get("formation_kind").map_err(db)?;
        let parent_aboutness: Vec<String> = parent_scope.try_get("aboutness").map_err(db)?;
        if input.formation_kind.as_str() != formation_kind
            || (!parent_aboutness.is_empty()
                && !input.applicability_scope.aboutness.iter().any(|entity| {
                    parent_aboutness
                        .iter()
                        .any(|value| value == entity.as_str())
                }))
        {
            return Err(Error::Invalid(
                "Schema revision must preserve formation kind and aboutness continuity".into(),
            ));
        }
        let content = CreateSchemaInput {
            producer: input.producer.clone(),
            operation_id: input.operation_id,
            subject: input.subject,
            title: input.title.clone(),
            structural_claim: input.structural_claim.clone(),
            applicability_scope: input.applicability_scope.clone(),
            boundary_definition: input.boundary_definition.clone(),
            formation_kind: parse_enum(formation_kind, "Schema formation kind")?,
            evidence_links: copied_links,
        };
        self.validate_schema_formation(&content).await?;
        let producer = if let Some(value) = &input.producer {
            Some(AuthorityStore::register_producer_in(mutation.tx(), value).await?)
        } else {
            None
        };
        self.write_schema_revision_in(
            mutation.tx(),
            &content,
            SchemaRevisionWrite {
                schema_id: input.schema_id,
                revision_id,
                parent: Some(parent),
                intent: Some(input.intent),
                number: revision_no,
                formed_at: self.cognition.now(input.subject),
                recorded_at: self.cognition.now(input.subject),
                producer,
            },
        )
        .await?;
        sqlx::query("UPDATE cognitive_schemas SET current_revision_id=$3,object_epoch=object_epoch+1,integrity_state='valid' WHERE subject_id=$1 AND schema_id=$2")
            .bind(input.subject.0)
            .bind(input.schema_id.0)
            .bind(revision_id.0)
            .execute(&mut **mutation.tx())
            .await
            .map_err(db)?;
        let sequence = mutation.invalidate(ProjectionInvalidation::all()).await?;
        self.enqueue_concept_in(
            mutation.tx(),
            input.subject,
            CognitiveRef::CognitiveSchemaRevision(revision_id),
            sequence,
        )
        .await?;
        self.invalidate_object_dependents_in(
            mutation.tx(),
            input.subject,
            "schema",
            &[input.schema_id.0],
            sequence,
            "source_revised",
        )
        .await?;
        mutation
            .commit(
                "schema_revision",
                Some(&input.schema_id.0.to_string()),
                Some(revision_id.0),
                Some(epoch + 1),
            )
            .await?;
        self.schema(input.subject, input.schema_id).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "schema split commits children and lineage in one Authority transaction"
    )]
    pub async fn split_schemas(
        &self,
        subject: SubjectId,
        schema_id: CognitiveSchemaId,
        operation_id: OperationId,
        expected_object_epoch: i64,
        children: Vec<CreateSchemaInput>,
    ) -> Result<Vec<SchemaView>> {
        if children.len() < 2 || children.len() > 16 {
            return Err(Error::Invalid("schema split needs 2..16 children".into()));
        }
        for child in &children {
            validate_schema_content(child)?;
            child.applicability_scope.valid_time.validate()?;
            for link in &child.evidence_links {
                self.validate_basis_for_subject(subject, std::slice::from_ref(&link.basis))
                    .await?;
            }
        }
        let digest = operation_digest(
            "split_cognitive_schema",
            subject,
            &serde_json::json!({"schema_id":schema_id,"expected_object_epoch":expected_object_epoch,"children":children}),
        )?;
        let mut mutation = match self
            .start_mutation(subject, operation_id, "split_cognitive_schema", &digest)
            .await?
        {
            MutationStart::Replay(receipt) => {
                if receipt.state == "committed" {
                    return Err(Error::Unavailable(
                        "split retry requires child lookup".into(),
                    ));
                }
                return Err(Error::Unavailable(
                    "schema split operation is already in progress".into(),
                ));
            }
            MutationStart::Active(mutation) => mutation,
        };
        let source=sqlx::query("SELECT current_revision_id,object_epoch,acceptance_state,purge_state FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE").bind(subject.0).bind(schema_id.0).fetch_optional(&mut **mutation.tx()).await.map_err(db)?.ok_or_else(||Error::NotFound("CognitiveSchema not found".into()))?;
        if source
            .try_get::<String, _>("acceptance_state")
            .map_err(db)?
            != "accepted"
        {
            return Err(Error::FailedPrecondition(
                "source CognitiveSchema is withdrawn".into(),
            ));
        }
        if source.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
            return Err(Error::FailedPrecondition(
                "source CognitiveSchema is purging".into(),
            ));
        }
        let epoch: i64 = source.try_get("object_epoch").map_err(db)?;
        if epoch != expected_object_epoch {
            return Err(Error::Conflict(
                "expected schema object epoch is stale".into(),
            ));
        }
        let source_revision: Uuid = source.try_get("current_revision_id").map_err(db)?;
        let mut ids = Vec::new();
        for child in children {
            if matches!(child.formation_kind, SchemaFormationKind::Synthesized)
                && child.evidence_links.len() < 2
            {
                return Err(Error::Invalid(
                    "schema child needs two evidence links".into(),
                ));
            }
            let child_id = CognitiveSchemaId::new();
            let child_revision = CognitiveSchemaRevisionId::new();
            let now = self.cognition.now(subject);
            let (kind, start, end) = temporal_columns(&child.applicability_scope.valid_time);
            let aboutness = child
                .applicability_scope
                .aboutness
                .iter()
                .map(|v| v.as_str().to_owned())
                .collect::<Vec<_>>();
            let tags = child
                .applicability_scope
                .tags
                .iter()
                .map(|v| v.0)
                .collect::<Vec<_>>();
            let child_basis = child
                .evidence_links
                .iter()
                .map(|link| link.basis.clone())
                .collect::<Vec<_>>();
            if matches!(child.formation_kind, SchemaFormationKind::Synthesized) {
                let summary = self.provenance_summary(subject, &child_basis).await?;
                let independent_roots = summary
                    .roots
                    .iter()
                    .filter(|root| matches!(root.certainty, EvidenceRootCertainty::Known))
                    .count();
                if summary.normalized_inputs.len() < 2 || independent_roots < 2 {
                    return Err(Error::Invalid(
                        "synthesized schema child lacks independent provenance roots".into(),
                    ));
                }
            }
            self.validate_basis_in_tx(mutation.tx(), subject, &child_basis)
                .await?;
            sqlx::query("INSERT INTO cognitive_schemas(schema_id,subject_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,1,'accepted','valid','normal','normal',$4)").bind(child_id.0).bind(subject.0).bind(child_revision.0).bind(now).execute(&mut **mutation.tx()).await.map_err(db)?;
            sqlx::query("INSERT INTO cognitive_schema_revisions(schema_revision_id,schema_id,revision_no,title,structural_claim,applicability_description,aboutness,tags,boundary_definition,formation_kind,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) VALUES($1,$2,1,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)").bind(child_revision.0).bind(child_id.0).bind(&child.title).bind(&child.structural_claim).bind(&child.applicability_scope.description).bind(&aboutness).bind(&tags).bind(&child.boundary_definition).bind(child.formation_kind.as_str()).bind(kind).bind(start).bind(end).bind(now).bind(now).execute(&mut **mutation.tx()).await.map_err(db)?;
            for link in child.evidence_links {
                self.insert_schema_link(mutation.tx(), subject, child_revision, link)
                    .await?;
            }
            sqlx::query("INSERT INTO cognitive_schema_lineage(from_revision_id,to_revision_id,relation) VALUES($1,$2,'schema_split_from')").bind(child_revision.0).bind(source_revision).execute(&mut **mutation.tx()).await.map_err(db)?;
            ids.push(child_id);
        }
        sqlx::query("UPDATE cognitive_schemas SET acceptance_state='withdrawn',object_epoch=object_epoch+1 WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(schema_id.0).execute(&mut **mutation.tx()).await.map_err(db)?;
        let sequence = mutation.invalidate(ProjectionInvalidation::all()).await?;
        for child in &ids {
            self.enqueue_concept_in(
                mutation.tx(),
                subject,
                CognitiveRef::CognitiveSchema(*child),
                sequence,
            )
            .await?;
        }
        self.invalidate_object_dependents_in(
            mutation.tx(),
            subject,
            "schema",
            &[schema_id.0],
            sequence,
            "source_split",
        )
        .await?;
        mutation
            .commit(
                "schema_split",
                Some(&schema_id.0.to_string()),
                None,
                Some(epoch + 1),
            )
            .await?;
        let mut views = Vec::new();
        for id in ids {
            views.push(self.schema(subject, id).await?);
        }
        Ok(views)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "schema merge owns stable locking, union, lineage, and one commit"
    )]
    pub async fn merge_schemas(
        &self,
        subject: SubjectId,
        operation_id: OperationId,
        source_ids: Vec<CognitiveSchemaId>,
        expected_epochs: Vec<i64>,
        merged: CreateSchemaInput,
    ) -> Result<SchemaView> {
        if source_ids.len() < 2
            || source_ids.len() > 16
            || source_ids.len() != expected_epochs.len()
        {
            return Err(Error::Invalid(
                "schema merge needs 2..16 sources and matching epochs".into(),
            ));
        }
        let unique_sources = source_ids.iter().collect::<std::collections::BTreeSet<_>>();
        if unique_sources.len() != source_ids.len() {
            return Err(Error::Invalid(
                "schema merge sources must be distinct".into(),
            ));
        }
        self.store.require_subject(subject).await?;
        validate_schema_content(&merged)?;
        merged.applicability_scope.valid_time.validate()?;
        let digest = operation_digest(
            "merge_cognitive_schemas",
            subject,
            &serde_json::json!({"sources":source_ids,"epochs":expected_epochs,"merged":merged}),
        )?;
        let mut mutation = match self
            .start_mutation(subject, operation_id, "merge_cognitive_schemas", &digest)
            .await?
        {
            MutationStart::Replay(receipt) => {
                if receipt.state == "committed" {
                    return Err(Error::Unavailable(
                        "merge retry requires result lookup".into(),
                    ));
                }
                return Err(Error::Unavailable(
                    "schema merge operation is already in progress".into(),
                ));
            }
            MutationStart::Active(mutation) => mutation,
        };
        let mut sorted = source_ids.iter().copied().enumerate().collect::<Vec<_>>();
        sorted.sort_by_key(|(_, id)| *id);
        let mut source_revisions = Vec::with_capacity(sorted.len());
        for (index, id) in sorted {
            let row=sqlx::query("SELECT object_epoch,current_revision_id,acceptance_state,purge_state FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE").bind(subject.0).bind(id.0).fetch_one(&mut **mutation.tx()).await.map_err(db)?;
            if row.try_get::<String, _>("acceptance_state").map_err(db)? != "accepted" {
                return Err(Error::FailedPrecondition(
                    "schema merge source is withdrawn".into(),
                ));
            }
            if row.try_get::<String, _>("purge_state").map_err(db)? == "purging" {
                return Err(Error::FailedPrecondition(
                    "schema merge source is purging".into(),
                ));
            }
            let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
            fence_epoch(epoch, expected_epochs[index])?;
            source_revisions.push((
                id,
                CognitiveSchemaRevisionId(row.try_get("current_revision_id").map_err(db)?),
            ));
        }
        let mut merged_links = std::collections::BTreeMap::new();
        for (_, revision) in &source_revisions {
            for link in active_schema_link_inputs(mutation.tx(), subject, *revision).await? {
                let key = format!("{}|{}", link.role.as_str(), link.basis.canonical_key());
                merged_links.entry(key).or_insert(link);
            }
        }
        for link in merged.evidence_links.clone() {
            let key = format!("{}|{}", link.role.as_str(), link.basis.canonical_key());
            merged_links.entry(key).or_insert(link);
        }
        if merged_links.len() < 2 {
            return Err(Error::Invalid(
                "merged CognitiveSchema needs at least two active evidence links".into(),
            ));
        }
        for link in merged_links.values() {
            self.validate_basis_for_subject(subject, std::slice::from_ref(&link.basis))
                .await?;
        }
        let merged_basis = merged_links
            .values()
            .map(|link| link.basis.clone())
            .collect::<Vec<_>>();
        self.validate_basis_in_tx(mutation.tx(), subject, &merged_basis)
            .await?;
        let new_id = CognitiveSchemaId::new();
        let new_revision = CognitiveSchemaRevisionId::new();
        let now = self.cognition.now(subject);
        let (kind, start, end) = temporal_columns(&merged.applicability_scope.valid_time);
        let aboutness = merged
            .applicability_scope
            .aboutness
            .iter()
            .map(|v| v.as_str().to_owned())
            .collect::<Vec<_>>();
        let tags = merged
            .applicability_scope
            .tags
            .iter()
            .map(|v| v.0)
            .collect::<Vec<_>>();
        sqlx::query("INSERT INTO cognitive_schemas(schema_id,subject_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,1,'accepted','valid','normal','normal',$4)").bind(new_id.0).bind(subject.0).bind(new_revision.0).bind(now).execute(&mut **mutation.tx()).await.map_err(db)?;
        sqlx::query("INSERT INTO cognitive_schema_revisions(schema_revision_id,schema_id,revision_no,title,structural_claim,applicability_description,aboutness,tags,boundary_definition,formation_kind,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) VALUES($1,$2,1,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)").bind(new_revision.0).bind(new_id.0).bind(&merged.title).bind(&merged.structural_claim).bind(&merged.applicability_scope.description).bind(&aboutness).bind(&tags).bind(&merged.boundary_definition).bind(merged.formation_kind.as_str()).bind(kind).bind(start).bind(end).bind(now).bind(now).execute(&mut **mutation.tx()).await.map_err(db)?;
        for link in merged_links.into_values() {
            self.insert_schema_link(mutation.tx(), subject, new_revision, link)
                .await?;
        }
        for (id, source_revision) in source_revisions {
            sqlx::query("UPDATE cognitive_schemas SET acceptance_state='withdrawn',object_epoch=object_epoch+1 WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(id.0).execute(&mut **mutation.tx()).await.map_err(db)?;
            sqlx::query("INSERT INTO cognitive_schema_lineage(from_revision_id,to_revision_id,relation) VALUES($1,$2,'schema_merged_from')").bind(new_revision.0).bind(source_revision.0).execute(&mut **mutation.tx()).await.map_err(db)?;
        }
        let sequence = mutation.invalidate(ProjectionInvalidation::all()).await?;
        self.enqueue_concept_in(
            mutation.tx(),
            subject,
            CognitiveRef::CognitiveSchemaRevision(new_revision),
            sequence,
        )
        .await?;
        self.invalidate_object_dependents_in(
            mutation.tx(),
            subject,
            "schema",
            &source_ids.iter().map(|id| id.0).collect::<Vec<_>>(),
            sequence,
            "source_merged",
        )
        .await?;
        mutation
            .commit(
                "schema",
                Some(&new_id.0.to_string()),
                Some(new_revision.0),
                Some(1),
            )
            .await?;
        self.schema(subject, new_id).await
    }
}
