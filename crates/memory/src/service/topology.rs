use super::*;
use nous_persistence::database_error as db;

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
        validate_association_endpoint(&input.from)?;
        validate_association_endpoint(&input.to)?;
        self.store.validate_reference(subject, &input.from).await?;
        self.store.validate_reference(subject, &input.to).await?;
        self.validate_association_supports(subject, &input).await?;
        let digest = operation_digest(
            "create_association",
            subject,
            &serde_json::json!({"from":input.from,"to":input.to,"relation_kind":input.relation_kind,"polarity":input.polarity,"support_class":input.support_class,"supports":input.supports,"producer_signature_id":input.producer_signature_id,"valid_time":input.valid_time}),
        )?;
        let mut tx = self.begin_mutation(subject).await?;
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
        sqlx::query("INSERT INTO association_evidence(association_evidence_id,subject_id,from_ref_kind,from_ref,to_ref_kind,to_ref,relation_kind,polarity,support_class,valid_time_kind,valid_time_start,valid_time_end,producer_signature_id,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)").bind(id.0).bind(subject.0).bind(from_kind).bind(from_ref).bind(to_kind).bind(to_ref).bind(&input.relation_kind).bind(input.polarity.as_str()).bind(input.support_class.as_str()).bind(valid_kind).bind(valid_start).bind(valid_end).bind(input.producer_signature_id).bind(now).execute(&mut *tx).await.map_err(db)?;
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
            if kind == "use_event" {
                let value = row.try_get::<String, _>("support_ref").map_err(db)?;
                let (consumer_ref, event_id) = parse_use_event_key(&value)?;
                Ok(AssociationSupport::UseEvent(UseEventRef {
                    subject_id: subject,
                    consumer_ref,
                    event_id: UseEventId(event_id),
                }))
            } else if kind == "evidence" {
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
                Ok(AssociationSupport::Revision(RevisionSupport::Evidence(EvidenceRef {
                    occurrence_id: OccurrenceId(row.try_get("occurrence_id").map_err(db)?),
                    locator,
                    support_role,
                })))
            } else {
                Ok(AssociationSupport::Revision(RevisionSupport::CognitionDependency(CognitionDependency {
                    target_revision: parse_reference(&kind, &row.try_get::<String, _>("support_ref").map_err(db)?)?,
                    support_role,
                })))
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

    async fn validate_association_supports(
        &self,
        subject: SubjectId,
        input: &CreateAssociationRequest,
    ) -> Result<()> {
        let mut has_source_evidence = false;
        let mut use_events = Vec::new();
        for support in &input.supports {
            match support {
                AssociationSupport::Revision(value) => {
                    if matches!(value, RevisionSupport::Evidence(_)) {
                        has_source_evidence = true;
                    }
                    self.validate_supports_for_subject(subject, std::slice::from_ref(value))
                        .await?;
                }
                AssociationSupport::UseEvent(value) => {
                    if value.subject_id != subject {
                        return Err(Error::Invalid(
                            "association UseEvent support belongs to another Subject".into(),
                        ));
                    }
                    let active = sqlx::query_scalar::<_, String>(
                        "SELECT use_kind FROM cognitive_use_events WHERE subject_id=$1 AND consumer_ref=$2 AND event_id=$3",
                    )
                    .bind(subject.0)
                    .bind(&value.consumer_ref)
                    .bind(value.event_id.0)
                    .fetch_optional(self.store.pool())
                    .await
                    .map_err(db)?;
                    let purged = sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS(SELECT 1 FROM purged_use_receipts WHERE subject_id=$1 AND consumer_ref=$2 AND event_id=$3)",
                    )
                    .bind(subject.0)
                    .bind(&value.consumer_ref)
                    .bind(value.event_id.0)
                    .fetch_one(self.store.pool())
                    .await
                    .map_err(db)?;
                    if active.is_none() && !purged {
                        return Err(Error::Invalid(
                            "association UseEvent support was not recorded".into(),
                        ));
                    }
                    use_events.push(active);
                }
            }
        }
        match input.support_class {
            AssociationSupportClass::HostExplicit => {}
            AssociationSupportClass::SourceEvidence if !has_source_evidence => {
                return Err(Error::Invalid(
                    "source_evidence association needs an EvidenceRef support".into(),
                ));
            }
            AssociationSupportClass::CognitiveDerivation
            | AssociationSupportClass::DerivedStructure => {
                let producer = input.producer_signature_id.ok_or_else(|| {
                    Error::Invalid("derived association requires producer_signature_id".into())
                })?;
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM producer_signatures WHERE producer_signature_id=$1)",
                )
                .bind(producer)
                .fetch_one(self.store.pool())
                .await
                .map_err(db)?;
                if !exists {
                    return Err(Error::Invalid(
                        "producer signature is not registered".into(),
                    ));
                }
                if input
                    .supports
                    .iter()
                    .all(|support| matches!(support, AssociationSupport::UseEvent(_)))
                {
                    return Err(Error::Invalid(
                        "derived association needs revision provenance support".into(),
                    ));
                }
            }
            AssociationSupportClass::MeaningfulUse if use_events.is_empty() => {
                return Err(Error::Invalid(
                    "meaningful_use association needs a UseEventRef support".into(),
                ));
            }
            AssociationSupportClass::MeaningfulUse => {
                if use_events.iter().any(Option::is_none) {
                    return Err(Error::Invalid(
                        "purged UseEvent receipts cannot prove meaningful-use kind".into(),
                    ));
                }
                for use_kind in use_events.into_iter().flatten() {
                    let allowed = matches!(
                        use_kind.as_str(),
                        "referenced" | "acted_on" | "result_supported" | "corrected" | "pinned"
                    ) || (input.polarity == AssociationPolarity::Negative
                        && use_kind == "result_refuted");
                    if !allowed {
                        return Err(Error::Invalid(
                            "UseEvent kind cannot support a positive meaningful-use association"
                                .into(),
                        ));
                    }
                }
            }
            AssociationSupportClass::SourceEvidence => {}
        }
        Ok(())
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
        let mut tx = self.begin_mutation(subject).await?;
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
        let policy = super::resolve_accessibility_policy(
            &self.configuration.snapshot_for_subject(subject)?,
        )?;
        policy
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
            producer: None,
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
    support: &AssociationSupport,
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
        AssociationSupport::UseEvent(value) => Ok((
            "use_event".into(),
            value.canonical_key(),
            SupportRole::Contextual.as_str().into(),
            None,
            None,
            None,
            None,
        )),
        AssociationSupport::Revision(RevisionSupport::Evidence(value)) => {
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
        AssociationSupport::Revision(RevisionSupport::CognitionDependency(value)) => {
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
        AssociationSupport::Revision(RevisionSupport::Seed(_)) => Err(Error::Invalid(
            "Association support cannot use Cognitive Seed support".into(),
        )),
    }
}

fn validate_association_endpoint(reference: &CognitiveRef) -> Result<()> {
    match reference {
        CognitiveRef::Memory(_) | CognitiveRef::CognitiveSchema(_) => Err(Error::Invalid(
            "association cognition endpoints must be exact revisions".into(),
        )),
        CognitiveRef::MemoryRevision(_)
        | CognitiveRef::CognitiveSchemaRevision(_)
        | CognitiveRef::Entity(_)
        | CognitiveRef::Tag(_)
        | CognitiveRef::Resource(_) => Ok(()),
        _ => Err(Error::Invalid(
            "association endpoint must be cognition revision, Entity, Tag, or Resource".into(),
        )),
    }
}

fn parse_use_event_key(value: &str) -> Result<(String, Uuid)> {
    let (_, rest) = value
        .split_once(':')
        .ok_or_else(|| Error::Infrastructure("invalid association UseEvent key".into()))?;
    let (consumer, event) = rest
        .rsplit_once(':')
        .ok_or_else(|| Error::Infrastructure("invalid association UseEvent key".into()))?;
    let event = event
        .parse()
        .map_err(|_| Error::Infrastructure("invalid association UseEvent id".into()))?;
    Ok((consumer.into(), event))
}
