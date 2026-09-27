use crate::*;
use nous_authority_store::database_error as db;

impl MemoryService {
    pub async fn create_association(
        &self,
        input: CreateAssociationRequest,
        subject: SubjectId,
    ) -> Result<AssociationEvidence> {
        if input.relation_kind.is_empty()
            || input.relation_kind.len() > 128
            || !input
                .relation_kind
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.:-".contains(&b))
        {
            return Err(Error::Invalid("relation_kind is invalid".into()));
        }
        if input.supports.is_empty() {
            return Err(Error::Invalid("association needs support".into()));
        }
        input.valid_time.validate()?;
        self.store.validate_reference(subject, &input.from).await?;
        self.store.validate_reference(subject, &input.to).await?;
        self.validate_supports_for_subject(subject, &input.supports)
            .await?;
        let digest = operation_digest(
            "create_association",
            subject,
            &serde_json::json!({"from":input.from,"to":input.to,"relation_kind":input.relation_kind,"polarity":input.polarity,"support_class":input.support_class,"supports":input.supports,"valid_time":input.valid_time}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, input.operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            input.operation_id,
            "create_association",
            &digest,
        )
        .await?
        {
            let id = receipt.result_ref;
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return self
                    .association(
                        subject,
                        AssociationEvidenceId(
                            id.ok_or_else(|| {
                                Error::Infrastructure("association receipt missing result".into())
                            })?
                            .parse()
                            .map_err(|_| {
                                Error::Infrastructure("invalid association receipt".into())
                            })?,
                        ),
                    )
                    .await;
            }
            return Err(Error::Unavailable(
                "association operation is already in progress".into(),
            ));
        }
        let id = AssociationEvidenceId::new();
        let now = Utc::now();
        let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
        let (from_kind, from_ref) = reference_parts(&input.from);
        let (to_kind, to_ref) = reference_parts(&input.to);
        sqlx::query("INSERT INTO association_evidence(association_evidence_id,subject_id,from_ref_kind,from_ref,to_ref_kind,to_ref,relation_kind,polarity,support_class,valid_time_kind,valid_time_start,valid_time_end,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)").bind(id.0).bind(subject.0).bind(from_kind).bind(from_ref).bind(to_kind).bind(to_ref).bind(&input.relation_kind).bind(input.polarity.as_str()).bind(input.support_class.as_str()).bind(valid_kind).bind(valid_start).bind(valid_end).bind(now).execute(&mut *tx).await.map_err(db)?;
        for support in &input.supports {
            let (
                kind,
                value,
                role,
                occurrence,
                source_region,
                derived_representation,
                derived_region,
            ) = association_support_parts(support)?;
            sqlx::query("INSERT INTO association_evidence_supports(association_evidence_id,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
                .bind(id.0)
                .bind(kind)
                .bind(value)
                .bind(role)
                .bind(occurrence)
                .bind(source_region)
                .bind(derived_representation)
                .bind(derived_region)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::topology()).await?;
        commit_receipt(
            &mut tx,
            subject,
            input.operation_id,
            "association",
            Some(&id.0.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.association(subject, id).await
    }

    async fn association(
        &self,
        subject: SubjectId,
        id: AssociationEvidenceId,
    ) -> Result<AssociationEvidence> {
        let row = sqlx::query(
            "SELECT * FROM association_evidence WHERE subject_id=$1 AND association_evidence_id=$2",
        )
        .bind(subject.0)
        .bind(id.0)
        .fetch_optional(self.store.pool())
        .await
        .map_err(db)?
        .ok_or_else(|| Error::NotFound("association evidence not found".into()))?;
        let from = parse_reference(
            &row.try_get::<String, _>("from_ref_kind").map_err(db)?,
            &row.try_get::<String, _>("from_ref").map_err(db)?,
        )?;
        let to = parse_reference(
            &row.try_get::<String, _>("to_ref_kind").map_err(db)?,
            &row.try_get::<String, _>("to_ref").map_err(db)?,
        )?;
        let supports=sqlx::query("SELECT support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id FROM association_evidence_supports WHERE association_evidence_id=$1 ORDER BY support_kind,support_ref,support_role").bind(id.0).fetch_all(self.store.pool()).await.map_err(db)?.into_iter().map(|row| {
            let kind: String = row.try_get("support_kind").map_err(db)?;
            let support_role = parse_enum(row.try_get("support_role").map_err(db)?, "association support role")?;
            if kind == "evidence" {
                let locator = match (
                    row.try_get::<Option<Uuid>, _>("source_region_id").map_err(db)?,
                    row.try_get::<Option<Uuid>, _>("derived_representation_id").map_err(db)?,
                    row.try_get::<Option<Uuid>, _>("derived_region_id").map_err(db)?,
                ) {
                    (Some(value), None, None) => EvidenceLocator::SourceRegion(nous_core::SourceRegionId(value)),
                    (None, Some(value), None) => EvidenceLocator::DerivedRepresentation(nous_core::DerivedRepresentationId(value)),
                    (None, None, Some(value)) => EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(value)),
                    (None, None, None) => EvidenceLocator::WholeOccurrence,
                    _ => return Err(Error::Infrastructure("association evidence locator is invalid".into())),
                };
                Ok(RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: OccurrenceId(row.try_get("occurrence_id").map_err(db)?),
                    locator,
                    support_role,
                }))
            } else {
                Ok(RevisionSupport::CognitionDependency(CognitionDependency {
                    target_revision: parse_reference(&kind, &row.try_get::<String, _>("support_ref").map_err(db)?)?,
                    support_role,
                }))
            }
        }).collect::<Result<Vec<_>>>()?;
        Ok(AssociationEvidence {
            association_evidence_id: id,
            subject_id: subject,
            from,
            to,
            relation_kind: row.try_get("relation_kind").map_err(db)?,
            polarity: parse_enum(row.try_get("polarity").map_err(db)?, "association polarity")?,
            support_class: parse_enum(
                row.try_get("support_class").map_err(db)?,
                "association support class",
            )?,
            supports,
            producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
            valid_time: temporal_from_columns(
                row.try_get("valid_time_kind").map_err(db)?,
                row.try_get("valid_time_start").map_err(db)?,
                row.try_get("valid_time_end").map_err(db)?,
            )?,
            created_at: row.try_get("created_at").map_err(db)?,
            revoked_at: row.try_get("revoked_at").map_err(db)?,
        })
    }

    pub async fn revoke_association(
        &self,
        subject: SubjectId,
        association: AssociationEvidenceId,
        operation_id: OperationId,
    ) -> Result<()> {
        let digest = operation_digest(
            "revoke_association",
            subject,
            &serde_json::json!({"association_id": association}),
        )?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, subject, operation_id).await?;
        if let Some(receipt) = check_receipt(
            &mut tx,
            subject,
            operation_id,
            "revoke_association",
            &digest,
        )
        .await?
        {
            tx.commit().await.map_err(db)?;
            if receipt.state == "committed" {
                return Ok(());
            }
            return Err(Error::Unavailable(
                "association revoke operation is already in progress".into(),
            ));
        }
        let changed = sqlx::query("UPDATE association_evidence SET revoked_at=COALESCE(revoked_at,$3) WHERE subject_id=$1 AND association_evidence_id=$2")
            .bind(subject.0)
            .bind(association.0)
            .bind(Utc::now())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if changed.rows_affected() == 0 {
            return Err(Error::NotFound("association evidence not found".into()));
        }
        AuthorityStore::invalidate_in(&mut tx, subject, ProjectionInvalidation::topology()).await?;
        commit_receipt(
            &mut tx,
            subject,
            operation_id,
            "association",
            Some(&association.0.to_string()),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)
    }

    pub async fn accessibility_level(
        &self,
        subject: SubjectId,
        memory: MemoryId,
        now: DateTime<Utc>,
    ) -> Result<AccessibilityLevel> {
        self.accessibility_policy
            .level_for_memory(&self.store, subject, memory, now)
            .await
    }

    pub async fn temporal_evidence_public(
        &self,
        revision: MemoryRevisionId,
    ) -> Result<TemporalEvidence> {
        self.temporal_evidence(revision).await
    }

    pub async fn require_exact_revision(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<()> {
        if !matches!(
            reference,
            CognitiveRef::MemoryRevision(_) | CognitiveRef::CognitiveSchemaRevision(_)
        ) {
            return Err(Error::Invalid(
                "durable cognition reference must be an exact revision".into(),
            ));
        }
        self.store.validate_reference(subject, reference).await
    }

    pub async fn consolidate(
        &self,
        subject: SubjectId,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationResult> {
        if request.subject != subject || request.source_memories.len() < 2 {
            return Err(Error::Invalid(
                "consolidation needs at least two source revisions".into(),
            ));
        }
        if matches!(request.target, ConsolidationTarget::TopologyOnly) {
            return Ok(ConsolidationResult {
                memory: None,
                topology_changes: 0,
            });
        }
        let supports = request
            .source_memories
            .iter()
            .map(|revision| {
                RevisionSupport::CognitionDependency(CognitionDependency {
                    target_revision: CognitiveRef::MemoryRevision(*revision),
                    support_role: SupportRole::Direct,
                })
            })
            .collect();
        let input = ExplicitMemoryInput {
            operation_id: request.operation_id,
            subject,
            cognitive_role: CognitiveRole::Declarative,
            formation_mode: FormationMode::Synthesized,
            grounding_occurrence_id: None,
            semantic_role: request
                .semantic_role
                .unwrap_or_else(|| "synthesized".into()),
            representation_text: request
                .representation_text
                .unwrap_or_else(|| "synthesized cognition".into()),
            title: None,
            supports,
            aboutness: Vec::new(),
            tags: Vec::new(),
            valid_time: TemporalExtent::Unknown,
            formed_at: request.formed_at,
            epistemic_class: EpistemicClass::Inferred,
        };
        Ok(ConsolidationResult {
            memory: Some(self.form_memory(input).await?),
            topology_changes: 0,
        })
    }
}

#[expect(
    clippy::type_complexity,
    reason = "the tuple mirrors the normalized support columns written atomically"
)]
fn association_support_parts(
    support: &RevisionSupport,
) -> Result<(
    String,
    String,
    String,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
)> {
    match support {
        RevisionSupport::Evidence(value) => {
            let (source_region, derived_representation, derived_region) = match value.locator {
                EvidenceLocator::WholeOccurrence => (None, None, None),
                EvidenceLocator::SourceRegion(id) => (Some(id.0), None, None),
                EvidenceLocator::DerivedRepresentation(id) => (None, Some(id.0), None),
                EvidenceLocator::DerivedRegion(id) => (None, None, Some(id.0)),
            };
            Ok((
                "evidence".into(),
                value.canonical_key(),
                value.support_role.as_str().into(),
                Some(value.occurrence_id.0),
                source_region,
                derived_representation,
                derived_region,
            ))
        }
        RevisionSupport::CognitionDependency(value) => {
            let (kind, reference) = reference_parts(&value.target_revision);
            Ok((
                kind,
                reference,
                value.support_role.as_str().into(),
                None,
                None,
                None,
                None,
            ))
        }
    }
}
