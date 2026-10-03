use super::*;
use nous_persistence::database_error as db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaView {
    pub schema: CognitiveSchema,
    pub revision: CognitiveSchemaRevision,
    pub evidence_links: Vec<SchemaEvidenceLink>,
}

impl MemoryService {
    pub async fn schema(
        &self,
        subject: SubjectId,
        schema_id: CognitiveSchemaId,
    ) -> Result<SchemaView> {
        let row = sqlx::query("SELECT s.schema_id,s.subject_id,s.current_revision_id,s.object_epoch,s.acceptance_state,s.integrity_state,s.suppression_state,s.purge_state,s.created_at,r.schema_revision_id,r.schema_id AS revision_schema_id,r.revision_no,r.parent_revision_id,r.revision_intent,r.title,r.structural_claim,r.applicability_description,r.aboutness,r.tags,r.boundary_definition,r.formation_kind,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id FROM cognitive_schemas s JOIN cognitive_schema_revisions r ON r.schema_revision_id=s.current_revision_id WHERE s.subject_id=$1 AND s.schema_id=$2")
            .bind(subject.0).bind(schema_id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(||Error::NotFound("CognitiveSchema not found".into()))?;
        let revision_id = CognitiveSchemaRevisionId(row.try_get("schema_revision_id").map_err(db)?);
        let revision = CognitiveSchemaRevision {
            schema_revision_id: revision_id,
            schema_id,
            revision_no: row.try_get("revision_no").map_err(db)?,
            parent_revision_id: row
                .try_get::<Option<Uuid>, _>("parent_revision_id")
                .map_err(db)?
                .map(CognitiveSchemaRevisionId),
            revision_intent: row
                .try_get::<Option<String>, _>("revision_intent")
                .map_err(db)?
                .map(|v| parse_enum(v, "schema revision intent"))
                .transpose()?,
            title: row.try_get("title").map_err(db)?,
            structural_claim: row.try_get("structural_claim").map_err(db)?,
            applicability_scope: SchemaScope {
                description: row.try_get("applicability_description").map_err(db)?,
                aboutness: row
                    .try_get::<Vec<String>, _>("aboutness")
                    .map_err(db)?
                    .into_iter()
                    .map(EntityRef::new)
                    .collect::<Result<Vec<_>>>()?,
                tags: row
                    .try_get::<Vec<Uuid>, _>("tags")
                    .map_err(db)?
                    .into_iter()
                    .map(TagId)
                    .collect(),
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
            },
            boundary_definition: row.try_get("boundary_definition").map_err(db)?,
            formation_kind: parse_enum(
                row.try_get("formation_kind").map_err(db)?,
                "schema formation kind",
            )?,
            formed_at: row.try_get("formed_at").map_err(db)?,
            recorded_at: row.try_get("recorded_at").map_err(db)?,
            producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
        };
        let links = self.schema_links(subject, revision_id).await?;
        Ok(SchemaView {
            schema: CognitiveSchema {
                schema_id,
                subject_id: subject,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: parse_enum(
                    row.try_get("acceptance_state").map_err(db)?,
                    "schema acceptance state",
                )?,
                integrity_state: parse_enum(
                    row.try_get("integrity_state").map_err(db)?,
                    "schema integrity state",
                )?,
                suppression_state: parse_enum(
                    row.try_get("suppression_state").map_err(db)?,
                    "schema suppression state",
                )?,
                purge_state: parse_enum(
                    row.try_get("purge_state").map_err(db)?,
                    "schema purge state",
                )?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision,
            evidence_links: links,
        })
    }

    pub(crate) async fn schema_links(
        &self,
        subject: SubjectId,
        revision: CognitiveSchemaRevisionId,
    ) -> Result<Vec<SchemaEvidenceLink>> {
        let rows=sqlx::query("SELECT link_id,subject_id,schema_revision_id,role,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,producer_signature_id,created_at,revoked_at FROM cognitive_schema_evidence_links WHERE subject_id=$1 AND schema_revision_id=$2 AND revoked_at IS NULL ORDER BY link_id").bind(subject.0).bind(revision.0).fetch_all(self.store.pool()).await.map_err(db)?;
        rows.into_iter()
            .map(|row| {
                let support = if row.try_get::<String, _>("support_kind").map_err(db)? == "evidence"
                {
                    RevisionSupport::Evidence(EvidenceRef {
                        occurrence_id: OccurrenceId(row.try_get("occurrence_id").map_err(db)?),
                        locator: match (
                            row.try_get::<Option<Uuid>, _>("source_region_id")
                                .map_err(db)?,
                            row.try_get::<Option<Uuid>, _>("derived_representation_id")
                                .map_err(db)?,
                            row.try_get::<Option<Uuid>, _>("derived_region_id")
                                .map_err(db)?,
                        ) {
                            (Some(id), None, None) => {
                                EvidenceLocator::SourceRegion(nous_core::SourceRegionId(id))
                            }
                            (None, Some(id), None) => EvidenceLocator::DerivedRepresentation(
                                nous_core::DerivedRepresentationId(id),
                            ),
                            (None, None, Some(id)) => {
                                EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(id))
                            }
                            (None, None, None) => EvidenceLocator::WholeOccurrence,
                            _ => {
                                return Err(Error::Infrastructure(
                                    "schema evidence locator is invalid".into(),
                                ));
                            }
                        },
                        support_role: parse_enum(
                            row.try_get("support_role").map_err(db)?,
                            "schema evidence support role",
                        )?,
                    })
                } else {
                    RevisionSupport::CognitionDependency(CognitionDependency {
                        target_revision: parse_reference(
                            &row.try_get::<String, _>("support_kind").map_err(db)?,
                            &row.try_get::<String, _>("support_ref").map_err(db)?,
                        )?,
                        support_role: parse_enum(
                            row.try_get("support_role").map_err(db)?,
                            "schema dependency support role",
                        )?,
                    })
                };
                Ok(SchemaEvidenceLink {
                    link_id: SchemaEvidenceLinkId(row.try_get("link_id").map_err(db)?),
                    subject_id: SubjectId(row.try_get("subject_id").map_err(db)?),
                    schema_revision_id: revision,
                    role: parse_enum(row.try_get("role").map_err(db)?, "schema evidence role")?,
                    support,
                    producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                    created_at: row.try_get("created_at").map_err(db)?,
                    revoked_at: row.try_get("revoked_at").map_err(db)?,
                })
            })
            .collect()
    }

    pub async fn create_schema(&self, input: CreateSchemaInput) -> Result<SchemaView> {
        match input.formation_kind {
            SchemaFormationKind::ExplicitImport if input.evidence_links.is_empty() => {
                return Err(Error::Invalid(
                    "explicit_import CognitiveSchema requires at least one evidence link".into(),
                ));
            }
            SchemaFormationKind::Synthesized if input.evidence_links.len() < 2 => {
                return Err(Error::Invalid(
                    "synthesized CognitiveSchema requires at least two evidence links".into(),
                ));
            }
            _ => {}
        }
        self.store.require_subject(input.subject).await?;
        validate_schema_content(&input)?;
        input.applicability_scope.valid_time.validate()?;
        if input.formed_at > Utc::now() + chrono::Duration::minutes(5) {
            return Err(Error::Invalid("formed_at is too far in the future".into()));
        }
        for link in &input.evidence_links {
            self.validate_supports_for_subject(input.subject, std::slice::from_ref(&link.support))
                .await?;
        }
        if matches!(input.formation_kind, SchemaFormationKind::Synthesized) {
            let supports = input
                .evidence_links
                .iter()
                .map(|link| link.support.clone())
                .collect::<Vec<_>>();
            let summary = self.provenance_summary(input.subject, &supports).await?;
            let independent_roots = summary
                .roots
                .iter()
                .filter(|root| matches!(root.certainty, EvidenceRootCertainty::Known))
                .count();
            if summary.normalized_inputs.len() < 2 || independent_roots < 2 {
                return Err(Error::Invalid(
                    "synthesized CognitiveSchema requires two normalized inputs and two known independent provenance roots".into(),
                ));
            }
        }
        let digest = operation_digest(
            "create_cognitive_schema",
            input.subject,
            &serde_json::json!({"title":input.title,"structural_claim":input.structural_claim,"scope":input.applicability_scope,"boundary_definition":input.boundary_definition,"formed_at":input.formed_at,"formation_kind":input.formation_kind,"evidence_links":input.evidence_links}),
        )?;
        let mut tx = self.begin_mutation(input.subject).await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "create_cognitive_schema",
            &digest,
        )
        .await?
        {
            let id = receipt.result_ref;
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self
                    .schema(
                        input.subject,
                        CognitiveSchemaId(
                            id.ok_or_else(|| {
                                Error::Infrastructure("schema receipt missing result".into())
                            })?
                            .parse()
                            .map_err(|_| Error::Infrastructure("invalid schema receipt".into()))?,
                        ),
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "schema operation is already in progress".into(),
            ));
        }
        let (schema_id, revision_id) = self.create_schema_in(&mut tx, &input, None).await?;
        AuthorityStore::invalidate_in(&mut tx, input.subject, ProjectionInvalidation::all())
            .await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "schema",
            Some(&schema_id.0.to_string()),
            Some(revision_id.0),
            Some(1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.schema(input.subject, schema_id).await
    }

    async fn insert_schema_link(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        subject: SubjectId,
        revision: CognitiveSchemaRevisionId,
        input: SchemaEvidenceLinkInput,
    ) -> Result<()> {
        let id = SchemaEvidenceLinkId::new();
        let (
            kind,
            value,
            support_role,
            occurrence,
            source_region,
            derived_representation,
            derived_region,
        ) = match input.support {
            RevisionSupport::Evidence(e) => {
                let (sr, dr, drr) = match e.locator {
                    EvidenceLocator::WholeOccurrence => (None, None, None),
                    EvidenceLocator::SourceRegion(id) => (Some(id.0), None, None),
                    EvidenceLocator::DerivedRepresentation(id) => (None, Some(id.0), None),
                    EvidenceLocator::DerivedRegion(id) => (None, None, Some(id.0)),
                };
                (
                    "evidence".to_owned(),
                    e.canonical_key(),
                    e.support_role.as_str().to_owned(),
                    Some(e.occurrence_id.0),
                    sr,
                    dr,
                    drr,
                )
            }
            RevisionSupport::CognitionDependency(d) => {
                let (k, v) = reference_parts(&d.target_revision);
                (
                    k,
                    v,
                    d.support_role.as_str().to_owned(),
                    None,
                    None,
                    None,
                    None,
                )
            }
            RevisionSupport::Seed(_) => {
                return Err(Error::Invalid(
                    "CognitiveSchema cannot use Cognitive Seed support".into(),
                ));
            }
        };
        sqlx::query("INSERT INTO cognitive_schema_evidence_links(link_id,subject_id,schema_revision_id,role,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)").bind(id.0).bind(subject.0).bind(revision.0).bind(input.role.as_str()).bind(kind).bind(value).bind(support_role).bind(occurrence).bind(source_region).bind(derived_representation).bind(derived_region).bind(self.cognition.now(subject)).execute(&mut **tx).await.map_err(db)?;
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
        self.validate_supports_for_subject(subject, std::slice::from_ref(&link.support))
            .await?;
        let digest = operation_digest(
            "add_schema_evidence",
            subject,
            &serde_json::json!({"schema_id":schema_id,"expected_object_epoch":expected_object_epoch,"link":link}),
        )?;
        let mut tx = self.begin_mutation(subject).await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "add_schema_evidence",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self.schema(subject, schema_id).await;
            }
            return Err(Error::Unavailable(
                "schema evidence operation is already in progress".into(),
            ));
        }
        let row=sqlx::query("SELECT current_revision_id,object_epoch FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE").bind(subject.0).bind(schema_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||Error::NotFound("CognitiveSchema not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != expected_object_epoch {
            return Err(Error::Conflict(
                "expected schema object epoch is stale".into(),
            ));
        }
        self.validate_supports_in_tx(&mut tx, subject, std::slice::from_ref(&link.support))
            .await?;
        self.insert_schema_link(
            &mut tx,
            subject,
            CognitiveSchemaRevisionId(row.try_get("current_revision_id").map_err(db)?),
            link,
        )
        .await?;
        sqlx::query("UPDATE cognitive_schemas SET object_epoch=object_epoch+1 WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(schema_id.0).execute(&mut *tx).await.map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::topology()).await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "schema",
            Some(&schema_id.0.to_string()),
            None,
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.schema(subject, schema_id).await
    }

    #[expect(
        clippy::too_many_lines,
        reason = "schema content revision owns one atomic Authority transaction"
    )]
    pub async fn revise_schema(&self, input: ReviseSchemaInput) -> Result<SchemaView> {
        self.store.require_subject(input.subject).await?;
        validate_schema_revision_content(&input)?;
        if input.formed_at > Utc::now() + chrono::Duration::minutes(5) {
            return Err(Error::Invalid("formed_at is too far in the future".into()));
        }
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
                "formed_at": input.formed_at,
                "copy_link_ids": input.copy_link_ids,
            }),
        )?;
        let mut tx = self.begin_mutation(input.subject).await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "revise_cognitive_schema",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self.schema(input.subject, input.schema_id).await;
            }
            return Err(Error::Unavailable(
                "schema revision operation is already in progress".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT current_revision_id,object_epoch FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE",
        )
        .bind(input.subject.0)
        .bind(input.schema_id.0)
        .fetch_optional(&mut *tx)
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
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut copied_links = Vec::new();
        for link_id in &input.copy_link_ids {
            let link = sqlx::query("SELECT schema_revision_id,role,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id FROM cognitive_schema_evidence_links WHERE link_id=$1 AND subject_id=$2 AND revoked_at IS NULL")
                .bind(link_id.0)
                .bind(input.subject.0)
                .fetch_optional(&mut *tx)
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
        let copied_supports = copied_links
            .iter()
            .map(|link| link.support.clone())
            .collect::<Vec<_>>();
        self.validate_object_dependency_cycle(
            input.subject,
            &format!("schema:{}", input.schema_id.0),
            &copied_supports,
        )
        .await?;
        self.validate_supports_in_tx(&mut tx, input.subject, &copied_supports)
            .await?;
        let revision_id = CognitiveSchemaRevisionId::new();
        let formation_kind: String = sqlx::query_scalar(
            "SELECT formation_kind FROM cognitive_schema_revisions WHERE schema_revision_id=$1",
        )
        .bind(parent.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let content = CreateSchemaInput {
            operation_id: input.operation_id,
            subject: input.subject,
            title: input.title.clone(),
            structural_claim: input.structural_claim.clone(),
            applicability_scope: input.applicability_scope.clone(),
            boundary_definition: input.boundary_definition.clone(),
            formed_at: input.formed_at,
            formation_kind: parse_enum(formation_kind, "Schema formation kind")?,
            evidence_links: copied_links,
        };
        self.validate_schema_formation(&content).await?;
        self.write_schema_revision_in(
            &mut tx,
            &content,
            SchemaRevisionWrite {
                schema_id: input.schema_id,
                revision_id,
                parent: Some(parent),
                intent: Some(input.intent),
                number: revision_no,
                recorded_at: self.cognition.now(input.subject),
                producer: None,
            },
        )
        .await?;
        sqlx::query("UPDATE cognitive_schemas SET current_revision_id=$3,object_epoch=object_epoch+1,integrity_state='valid' WHERE subject_id=$1 AND schema_id=$2")
            .bind(input.subject.0)
            .bind(input.schema_id.0)
            .bind(revision_id.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, input.subject, ProjectionInvalidation::all())
            .await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "schema_revision",
            Some(&input.schema_id.0.to_string()),
            Some(revision_id.0),
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
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
                self.validate_supports_for_subject(subject, std::slice::from_ref(&link.support))
                    .await?;
            }
        }
        let digest = operation_digest(
            "split_cognitive_schema",
            subject,
            &serde_json::json!({"schema_id":schema_id,"expected_object_epoch":expected_object_epoch,"children":children}),
        )?;
        let mut tx = self.begin_mutation(subject).await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "split_cognitive_schema",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Err(Error::Unavailable(
                    "split retry requires child lookup".into(),
                ));
            }
            return Err(Error::Unavailable(
                "schema split operation is already in progress".into(),
            ));
        }
        let source=sqlx::query("SELECT current_revision_id,object_epoch,acceptance_state,purge_state FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE").bind(subject.0).bind(schema_id.0).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(||Error::NotFound("CognitiveSchema not found".into()))?;
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
            let now = Utc::now();
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
            let child_supports = child
                .evidence_links
                .iter()
                .map(|link| link.support.clone())
                .collect::<Vec<_>>();
            if matches!(child.formation_kind, SchemaFormationKind::Synthesized) {
                let summary = self.provenance_summary(subject, &child_supports).await?;
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
            self.validate_supports_in_tx(&mut tx, subject, &child_supports)
                .await?;
            sqlx::query("INSERT INTO cognitive_schemas(schema_id,subject_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,1,'accepted','valid','normal','normal',$4)").bind(child_id.0).bind(subject.0).bind(child_revision.0).bind(now).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("INSERT INTO cognitive_schema_revisions(schema_revision_id,schema_id,revision_no,title,structural_claim,applicability_description,aboutness,tags,boundary_definition,formation_kind,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) VALUES($1,$2,1,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)").bind(child_revision.0).bind(child_id.0).bind(&child.title).bind(&child.structural_claim).bind(&child.applicability_scope.description).bind(&aboutness).bind(&tags).bind(&child.boundary_definition).bind(child.formation_kind.as_str()).bind(kind).bind(start).bind(end).bind(child.formed_at).bind(now).execute(&mut *tx).await.map_err(db)?;
            for link in child.evidence_links {
                self.insert_schema_link(&mut tx, subject, child_revision, link)
                    .await?;
            }
            sqlx::query("INSERT INTO cognitive_schema_lineage(from_revision_id,to_revision_id,relation) VALUES($1,$2,'schema_split_from')").bind(child_revision.0).bind(source_revision).execute(&mut *tx).await.map_err(db)?;
            ids.push(child_id);
        }
        sqlx::query("UPDATE cognitive_schemas SET acceptance_state='withdrawn',object_epoch=object_epoch+1 WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(schema_id.0).execute(&mut *tx).await.map_err(db)?;
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "schema_split",
            Some(&schema_id.0.to_string()),
            None,
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
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
        let mut tx = self.begin_mutation(subject).await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "merge_cognitive_schemas",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Err(Error::Unavailable(
                    "merge retry requires result lookup".into(),
                ));
            }
            return Err(Error::Unavailable(
                "schema merge operation is already in progress".into(),
            ));
        }
        let mut sorted = source_ids.iter().copied().enumerate().collect::<Vec<_>>();
        sorted.sort_by_key(|(_, id)| *id);
        let mut source_revisions = Vec::with_capacity(sorted.len());
        for (index, id) in sorted {
            let row=sqlx::query("SELECT object_epoch,current_revision_id,acceptance_state,purge_state FROM cognitive_schemas WHERE subject_id=$1 AND schema_id=$2 FOR UPDATE").bind(subject.0).bind(id.0).fetch_one(&mut *tx).await.map_err(db)?;
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
            if epoch != expected_epochs[index] {
                return Err(Error::Conflict("schema merge epoch is stale".into()));
            }
            source_revisions.push((
                id,
                CognitiveSchemaRevisionId(row.try_get("current_revision_id").map_err(db)?),
            ));
        }
        let mut merged_links = std::collections::BTreeMap::new();
        for (_, revision) in &source_revisions {
            for link in active_schema_link_inputs(&mut tx, subject, *revision).await? {
                let key = format!("{}|{}", link.role.as_str(), link.support.canonical_key());
                merged_links.entry(key).or_insert(link);
            }
        }
        for link in merged.evidence_links.clone() {
            let key = format!("{}|{}", link.role.as_str(), link.support.canonical_key());
            merged_links.entry(key).or_insert(link);
        }
        if merged_links.len() < 2 {
            return Err(Error::Invalid(
                "merged CognitiveSchema needs at least two active evidence links".into(),
            ));
        }
        for link in merged_links.values() {
            self.validate_supports_for_subject(subject, std::slice::from_ref(&link.support))
                .await?;
        }
        let merged_supports = merged_links
            .values()
            .map(|link| link.support.clone())
            .collect::<Vec<_>>();
        self.validate_supports_in_tx(&mut tx, subject, &merged_supports)
            .await?;
        let new_id = CognitiveSchemaId::new();
        let new_revision = CognitiveSchemaRevisionId::new();
        let now = Utc::now();
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
        sqlx::query("INSERT INTO cognitive_schemas(schema_id,subject_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,1,'accepted','valid','normal','normal',$4)").bind(new_id.0).bind(subject.0).bind(new_revision.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO cognitive_schema_revisions(schema_revision_id,schema_id,revision_no,title,structural_claim,applicability_description,aboutness,tags,boundary_definition,formation_kind,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) VALUES($1,$2,1,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)").bind(new_revision.0).bind(new_id.0).bind(&merged.title).bind(&merged.structural_claim).bind(&merged.applicability_scope.description).bind(&aboutness).bind(&tags).bind(&merged.boundary_definition).bind(merged.formation_kind.as_str()).bind(kind).bind(start).bind(end).bind(merged.formed_at).bind(now).execute(&mut *tx).await.map_err(db)?;
        for link in merged_links.into_values() {
            self.insert_schema_link(&mut tx, subject, new_revision, link)
                .await?;
        }
        for (id, source_revision) in source_revisions {
            sqlx::query("UPDATE cognitive_schemas SET acceptance_state='withdrawn',object_epoch=object_epoch+1 WHERE subject_id=$1 AND schema_id=$2").bind(subject.0).bind(id.0).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("INSERT INTO cognitive_schema_lineage(from_revision_id,to_revision_id,relation) VALUES($1,$2,'schema_merged_from')").bind(new_revision.0).bind(source_revision.0).execute(&mut *tx).await.map_err(db)?;
        }
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::all()).await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "schema",
            Some(&new_id.0.to_string()),
            Some(new_revision.0),
            Some(1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.schema(subject, new_id).await
    }
}

fn validate_schema_content(input: &CreateSchemaInput) -> Result<()> {
    if input.structural_claim.trim().is_empty()
        || input.structural_claim.len() > 16 * 1024
        || input.applicability_scope.description.trim().is_empty()
        || input.applicability_scope.description.len() > 16 * 1024
        || input.boundary_definition.trim().is_empty()
        || input.boundary_definition.len() > 16 * 1024
    {
        return Err(Error::Invalid(
            "schema content is required and bounded".into(),
        ));
    }
    Ok(())
}

fn validate_schema_revision_content(input: &ReviseSchemaInput) -> Result<()> {
    if input.operation_id.0.is_nil() {
        return Err(Error::Invalid("operation_id is required".into()));
    }
    if input.structural_claim.trim().is_empty()
        || input.structural_claim.len() > 16 * 1024
        || input.applicability_scope.description.trim().is_empty()
        || input.applicability_scope.description.len() > 16 * 1024
        || input.boundary_definition.trim().is_empty()
        || input.boundary_definition.len() > 16 * 1024
    {
        return Err(Error::Invalid(
            "schema content is required and bounded".into(),
        ));
    }
    input.applicability_scope.valid_time.validate()
}

async fn active_schema_link_inputs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subject: SubjectId,
    revision: CognitiveSchemaRevisionId,
) -> Result<Vec<SchemaEvidenceLinkInput>> {
    let rows = sqlx::query("SELECT role,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id FROM cognitive_schema_evidence_links WHERE subject_id=$1 AND schema_revision_id=$2 AND revoked_at IS NULL ORDER BY link_id")
        .bind(subject.0)
        .bind(revision.0)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
    rows.into_iter().map(decode_schema_link_input).collect()
}

fn decode_schema_link_input(row: sqlx::postgres::PgRow) -> Result<SchemaEvidenceLinkInput> {
    let support_kind: String = row.try_get("support_kind").map_err(db)?;
    let support_role = parse_enum(
        row.try_get("support_role").map_err(db)?,
        "schema support role",
    )?;
    let support = if support_kind == "evidence" {
        let locator = match (
            row.try_get::<Option<Uuid>, _>("source_region_id")
                .map_err(db)?,
            row.try_get::<Option<Uuid>, _>("derived_representation_id")
                .map_err(db)?,
            row.try_get::<Option<Uuid>, _>("derived_region_id")
                .map_err(db)?,
        ) {
            (Some(id), None, None) => EvidenceLocator::SourceRegion(nous_core::SourceRegionId(id)),
            (None, Some(id), None) => {
                EvidenceLocator::DerivedRepresentation(nous_core::DerivedRepresentationId(id))
            }
            (None, None, Some(id)) => {
                EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(id))
            }
            (None, None, None) => EvidenceLocator::WholeOccurrence,
            _ => {
                return Err(Error::Infrastructure(
                    "schema evidence locator is invalid".into(),
                ));
            }
        };
        RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: OccurrenceId(row.try_get("occurrence_id").map_err(db)?),
            locator,
            support_role,
        })
    } else {
        RevisionSupport::CognitionDependency(CognitionDependency {
            target_revision: parse_reference(
                &support_kind,
                &row.try_get::<String, _>("support_ref").map_err(db)?,
            )?,
            support_role,
        })
    };
    Ok(SchemaEvidenceLinkInput {
        role: parse_enum(row.try_get("role").map_err(db)?, "schema evidence role")?,
        support,
    })
}

pub(super) struct SchemaRevisionWrite {
    pub schema_id: CognitiveSchemaId,
    pub revision_id: CognitiveSchemaRevisionId,
    pub parent: Option<CognitiveSchemaRevisionId>,
    pub intent: Option<RevisionIntent>,
    pub number: i32,
    pub recorded_at: DateTime<Utc>,
    pub producer: Option<Uuid>,
}
impl MemoryService {
    pub(super) async fn validate_schema_formation(&self, input: &CreateSchemaInput) -> Result<()> {
        validate_schema_content(input)?;
        input.applicability_scope.valid_time.validate()?;
        let supports: Vec<_> = input
            .evidence_links
            .iter()
            .map(|link| link.support.clone())
            .collect();
        if supports.is_empty() {
            return Err(Error::Invalid(
                "Schema revision requires evidence links".into(),
            ));
        }
        for support in &supports {
            validate_exact_supports(std::slice::from_ref(support))?;
        }
        self.validate_supports_for_subject(input.subject, &supports)
            .await?;
        if input.formation_kind == SchemaFormationKind::Synthesized {
            self.validate_formation_semantics(input.subject, FormationMode::Synthesized, &supports)
                .await?;
        }
        Ok(())
    }
    pub(super) async fn create_schema_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        input: &CreateSchemaInput,
        producer: Option<Uuid>,
    ) -> Result<(CognitiveSchemaId, CognitiveSchemaRevisionId)> {
        let schema_id = CognitiveSchemaId::new();
        let revision_id = CognitiveSchemaRevisionId::new();
        let now = self.cognition.now(input.subject);
        let supports: Vec<_> = input
            .evidence_links
            .iter()
            .map(|link| link.support.clone())
            .collect();
        self.validate_supports_in_tx(tx, input.subject, &supports)
            .await?;
        sqlx::query("INSERT INTO cognitive_schemas(schema_id,subject_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,1,'accepted','valid','normal','normal',$4)")
            .bind(schema_id.0).bind(input.subject.0).bind(revision_id.0).bind(now).execute(&mut **tx).await.map_err(db)?;
        self.write_schema_revision_in(
            tx,
            input,
            SchemaRevisionWrite {
                schema_id,
                revision_id,
                parent: None,
                intent: None,
                number: 1,
                recorded_at: now,
                producer,
            },
        )
        .await?;
        Ok((schema_id, revision_id))
    }
    pub(super) async fn write_schema_revision_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        input: &CreateSchemaInput,
        write: SchemaRevisionWrite,
    ) -> Result<()> {
        let (kind, start, end) = temporal_columns(&input.applicability_scope.valid_time);
        let aboutness: Vec<String> = input
            .applicability_scope
            .aboutness
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect();
        let tags: Vec<Uuid> = input
            .applicability_scope
            .tags
            .iter()
            .map(|id| id.0)
            .collect();
        sqlx::query("INSERT INTO cognitive_schema_revisions(schema_revision_id,schema_id,revision_no,parent_revision_id,revision_intent,title,structural_claim,applicability_description,aboutness,tags,boundary_definition,formation_kind,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)")
            .bind(write.revision_id.0).bind(write.schema_id.0).bind(write.number).bind(write.parent.map(|id|id.0)).bind(write.intent.map(|intent|intent.as_str())).bind(&input.title).bind(&input.structural_claim).bind(&input.applicability_scope.description).bind(aboutness).bind(tags).bind(&input.boundary_definition).bind(input.formation_kind.as_str()).bind(kind).bind(start).bind(end).bind(input.formed_at).bind(write.recorded_at).bind(write.producer).execute(&mut **tx).await.map_err(db)?;
        for link in &input.evidence_links {
            self.insert_schema_link(tx, input.subject, write.revision_id, link.clone())
                .await?;
        }
        Ok(())
    }
}
