use super::*;
mod mutation;
mod read;

use nous_persistence::database_error as db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaView {
    pub schema: CognitiveSchema,
    pub revision: CognitiveSchemaRevision,
    pub evidence_links: Vec<SchemaEvidenceLink>,
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
    pub formed_at: DateTime<Utc>,
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
        formed: DateTime<Utc>,
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
                formed_at: formed,
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
            .bind(write.revision_id.0).bind(write.schema_id.0).bind(write.number).bind(write.parent.map(|id|id.0)).bind(write.intent.map(|intent|intent.as_str())).bind(&input.title).bind(&input.structural_claim).bind(&input.applicability_scope.description).bind(aboutness).bind(tags).bind(&input.boundary_definition).bind(input.formation_kind.as_str()).bind(kind).bind(start).bind(end).bind(write.formed_at).bind(write.recorded_at).bind(write.producer).execute(&mut **tx).await.map_err(db)?;
        for link in &input.evidence_links {
            self.insert_schema_link(tx, input.subject, write.revision_id, link.clone())
                .await?;
        }
        Ok(())
    }
}

fn canonical_schema_producer(
    producer: Option<&ProducerSignature>,
) -> Result<Option<ProducerSignature>> {
    producer
        .map(|producer| {
            if producer.operation != CapabilityOperation::MemoryConsolidationText {
                return Err(Error::Invalid(
                    "Schema producer requires a cognition consolidation operation".into(),
                ));
            }
            AuthorityStore::canonical_producer(producer)
        })
        .transpose()
}
