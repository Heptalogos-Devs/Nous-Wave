use super::*;

pub(super) async fn insert_relationship_revision(
    tx: &mut Transaction<'_, Postgres>,
    input: &CreateRelationship,
    relationship_id: RelationshipAssertionId,
    revision_id: RelationshipRevisionId,
    revision_no: i32,
    parent: Option<RelationshipRevisionId>,
    intent: Option<&str>,
    recorded_at: DateTime<Utc>,
) -> Result<()> {
    let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
    sqlx::query("INSERT INTO relationship_revisions(relationship_revision_id,relationship_id,subject_id,revision_no,parent_revision_id,revision_intent,degree_kind,degree_value,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,NULL,$7,$8,$9,$10,$11,$12,$13,$14)")
        .bind(revision_id.0).bind(relationship_id.0).bind(input.subject.0).bind(revision_no).bind(parent.map(|value| value.0)).bind(intent).bind(&input.degree).bind(enum_name(input.epistemic_class)).bind(valid_kind).bind(valid_start).bind(valid_end).bind(input.formed_at).bind(recorded_at).bind(input.producer_signature_id).execute(&mut **tx).await.map_err(db)?;
    insert_supports(
        tx,
        "relationship_revision_supports",
        "relationship_revision_id",
        revision_id.0,
        &input.supports,
    )
    .await
}

pub(super) async fn insert_convention_revision(
    tx: &mut Transaction<'_, Postgres>,
    input: &CreateLanguageConvention,
    convention_id: LanguageConventionId,
    revision_id: LanguageConventionRevisionId,
    formation_digest: String,
    recorded_at: DateTime<Utc>,
) -> Result<()> {
    let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
    sqlx::query("INSERT INTO language_convention_revisions(convention_revision_id,convention_id,subject_id,revision_no,parent_revision_id,revision_intent,meaning,pragmatic_role,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id,formation_policy_digest) VALUES($1,$2,$3,1,NULL,NULL,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(revision_id.0).bind(convention_id.0).bind(input.subject.0).bind(&input.meaning).bind(&input.pragmatic_role).bind(enum_name(input.epistemic_class)).bind(valid_kind).bind(valid_start).bind(valid_end).bind(input.formed_at).bind(recorded_at).bind(input.producer_signature_id).bind(formation_digest).execute(&mut **tx).await.map_err(db)?;
    insert_supports(
        tx,
        "language_convention_revision_supports",
        "convention_revision_id",
        revision_id.0,
        &input.supports,
    )
    .await
}

pub(super) async fn insert_supports(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    column: &str,
    revision: Uuid,
    supports: &[RevisionSupport],
) -> Result<()> {
    for support in supports {
        let (
            kind,
            reference,
            role,
            occurrence_id,
            source_region_id,
            derived_representation_id,
            derived_region_id,
            seed_path,
        ) = match support {
            RevisionSupport::Evidence(value) => {
                let (source_region_id, derived_representation_id, derived_region_id) =
                    match value.locator {
                        EvidenceLocator::WholeOccurrence => (None, None, None),
                        EvidenceLocator::SourceRegion(id) => (Some(id.0), None, None),
                        EvidenceLocator::DerivedRepresentation(id) => (None, Some(id.0), None),
                        EvidenceLocator::DerivedRegion(id) => (None, None, Some(id.0)),
                    };
                (
                    "evidence".into(),
                    value.canonical_key(),
                    value.support_role.as_str().to_owned(),
                    Some(value.occurrence_id.0),
                    source_region_id,
                    derived_representation_id,
                    derived_region_id,
                    None,
                )
            }
            RevisionSupport::CognitionDependency(value) => (
                reference_parts(&value.target_revision).0,
                reference_parts(&value.target_revision).1,
                value.support_role.as_str().to_owned(),
                None,
                None,
                None,
                None,
                None,
            ),
            RevisionSupport::Seed(value) => (
                "cognitive_seed_version".into(),
                value.seed_version_id.0.to_string(),
                "direct".into(),
                None,
                None,
                None,
                None,
                Some(value.semantic_path.clone()),
            ),
        };
        let query = format!(
            "INSERT INTO {table}({column},support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id,seed_path) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)"
        );
        sqlx::query(sqlx::AssertSqlSafe(query))
            .bind(revision)
            .bind(kind)
            .bind(reference)
            .bind(role)
            .bind(occurrence_id)
            .bind(source_region_id)
            .bind(derived_representation_id)
            .bind(derived_region_id)
            .bind(seed_path)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
    }
    Ok(())
}

pub(super) async fn validate_social_supports(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    supports: &[RevisionSupport],
) -> Result<()> {
    for support in supports {
        match support {
            RevisionSupport::Evidence(value) => {
                let row = sqlx::query(
                    "SELECT artifact_id FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=$2 FOR SHARE",
                )
                .bind(subject.0)
                .bind(value.occurrence_id.0)
                .fetch_optional(&mut **tx)
                .await
                .map_err(db)?
                .ok_or_else(|| Error::FailedPrecondition("Social evidence occurrence is not owned by Subject".into()))?;
                let occurrence_artifact: Option<Uuid> = row.try_get("artifact_id").map_err(db)?;
                match value.locator {
                    EvidenceLocator::WholeOccurrence => {}
                    EvidenceLocator::SourceRegion(id) => {
                        let artifact: Uuid = sqlx::query_scalar(
                            "SELECT artifact_id FROM source_regions WHERE subject_id=$1 AND source_region_id=$2",
                        )
                        .bind(subject.0)
                        .bind(id.0)
                        .fetch_one(&mut **tx)
                        .await
                        .map_err(db)?;
                        if Some(artifact) != occurrence_artifact {
                            return Err(Error::FailedPrecondition(
                                "Social source region does not match occurrence".into(),
                            ));
                        }
                    }
                    EvidenceLocator::DerivedRepresentation(id) => {
                        let artifact: Option<Uuid> = sqlx::query_scalar(
                            "SELECT sr.artifact_id FROM derived_representations d JOIN source_regions sr USING(source_region_id) WHERE d.subject_id=$1 AND d.derived_representation_id=$2",
                        )
                        .bind(subject.0)
                        .bind(id.0)
                        .fetch_one(&mut **tx)
                        .await
                        .map_err(db)?;
                        if artifact != occurrence_artifact {
                            return Err(Error::FailedPrecondition(
                                "Social derived representation does not match occurrence".into(),
                            ));
                        }
                    }
                    EvidenceLocator::DerivedRegion(id) => {
                        let artifact: Option<Uuid> = sqlx::query_scalar(
                            "SELECT sr.artifact_id FROM derived_regions dr JOIN derived_representations d USING(derived_representation_id) JOIN source_regions sr USING(source_region_id) WHERE dr.subject_id=$1 AND dr.derived_region_id=$2",
                        )
                        .bind(subject.0)
                        .bind(id.0)
                        .fetch_one(&mut **tx)
                        .await
                        .map_err(db)?;
                        if artifact != occurrence_artifact {
                            return Err(Error::FailedPrecondition(
                                "Social derived region does not match occurrence".into(),
                            ));
                        }
                    }
                }
            }
            RevisionSupport::CognitionDependency(value) => {
                let (kind, reference) = reference_parts(&value.target_revision);
                let id = Uuid::parse_str(&reference)
                    .map_err(|_| Error::Invalid("invalid Social dependency reference".into()))?;
                let exists = match kind.as_str() {
                    "memory_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_revisions WHERE subject_id=$1 AND memory_revision_id=$2)").bind(subject.0).bind(id).fetch_one(&mut **tx).await.map_err(db)?,
                    "cognitive_schema_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cognitive_schema_revisions r JOIN cognitive_schemas s USING(schema_id) WHERE s.subject_id=$1 AND r.schema_revision_id=$2)").bind(subject.0).bind(id).fetch_one(&mut **tx).await.map_err(db)?,
                    "self_facet_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM self_facet_revisions WHERE subject_id=$1 AND self_facet_revision_id=$2)").bind(subject.0).bind(id).fetch_one(&mut **tx).await.map_err(db)?,
                    "narrative_identity_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM narrative_identity_revisions WHERE subject_id=$1 AND narrative_identity_revision_id=$2)").bind(subject.0).bind(id).fetch_one(&mut **tx).await.map_err(db)?,
                    "relationship_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM relationship_revisions WHERE subject_id=$1 AND relationship_revision_id=$2)").bind(subject.0).bind(id).fetch_one(&mut **tx).await.map_err(db)?,
                    "language_convention_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM language_convention_revisions WHERE subject_id=$1 AND convention_revision_id=$2)").bind(subject.0).bind(id).fetch_one(&mut **tx).await.map_err(db)?,
                    _ => false,
                };
                if !exists {
                    return Err(Error::FailedPrecondition(
                        "Social cognition dependency is not an owned exact revision".into(),
                    ));
                }
            }
            RevisionSupport::Seed(value) => {
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cognitive_seed_versions WHERE subject_id=$1 AND seed_version_id=$2)")
                    .bind(subject.0)
                    .bind(value.seed_version_id.0)
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(db)?;
                if !exists {
                    return Err(Error::FailedPrecondition(
                        "Cognitive Seed support is not owned by Subject".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "Formation acceptance validates support ownership, actor identity, and provenance roots together"
)]
pub(super) async fn accept_formation(
    store: &AuthorityStore,
    subject: SubjectId,
    evidence: &[ConventionFormationEvidence],
    supports: &[RevisionSupport],
    policy: &SocialPolicy,
) -> Result<()> {
    #[derive(Debug)]
    struct OccurrenceEvidence {
        actor: EntityRef,
        root: Option<String>,
        observed_at: DateTime<Utc>,
    }

    let mut occurrence_ids = BTreeSet::new();
    for item in evidence {
        let index = usize::try_from(item.support_index)
            .map_err(|_| Error::Invalid("formation support index must be non-negative".into()))?;
        let support = supports
            .get(index)
            .ok_or_else(|| Error::Invalid("formation support index is out of range".into()))?;
        match item.kind {
            FormationEvidenceKind::SeedDirect => {
                if !matches!(support, RevisionSupport::Seed(_)) || item.external_actor.is_some() {
                    return Err(Error::Invalid(
                        "seed_direct must reference a SeedSupportRef and no external actor".into(),
                    ));
                }
            }
            FormationEvidenceKind::Contextual => {
                if !matches!(
                    support,
                    RevisionSupport::Evidence(_) | RevisionSupport::CognitionDependency(_)
                ) {
                    return Err(Error::Invalid(
                        "contextual formation evidence must reference evidence or cognition dependency".into(),
                    ));
                }
            }
            _ => {
                let RevisionSupport::Evidence(value) = support else {
                    return Err(Error::Invalid(
                        "external formation evidence must reference an EvidenceRef".into(),
                    ));
                };
                if item.external_actor.is_none() {
                    return Err(Error::Invalid(
                        "external formation evidence requires external_actor".into(),
                    ));
                }
                occurrence_ids.insert(value.occurrence_id.0);
            }
        }
    }
    let rows = if occurrence_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query("SELECT occurrence_id,actor_entity_ref,artifact_id,external_object_ref,conversation_ref,observed_at FROM observation_occurrences WHERE subject_id=$1 AND occurrence_id=ANY($2::uuid[])")
            .bind(subject.0)
            .bind(occurrence_ids.iter().copied().collect::<Vec<_>>())
            .fetch_all(store.pool())
            .await
            .map_err(db)?
    };
    let mut occurrence_evidence = std::collections::HashMap::new();
    for row in rows {
        let occurrence_id: Uuid = row.try_get("occurrence_id").map_err(db)?;
        let actor = row
            .try_get::<Option<String>, _>("actor_entity_ref")
            .map_err(db)?
            .ok_or_else(|| Error::FailedPrecondition("formation occurrence has no actor".into()))?;
        let actor = EntityRef::new(actor)?;
        let external_object_ref: Option<String> = row.try_get("external_object_ref").map_err(db)?;
        let artifact_id: Option<Uuid> = row.try_get("artifact_id").map_err(db)?;
        let conversation_ref: Option<String> = row.try_get("conversation_ref").map_err(db)?;
        let root = external_object_ref
            .map(|value| format!("external_object:{value}"))
            .or_else(|| artifact_id.map(|value| format!("artifact:{value}")))
            .or_else(|| conversation_ref.map(|value| format!("conversation:{value}")));
        occurrence_evidence.insert(
            occurrence_id,
            OccurrenceEvidence {
                actor,
                root,
                observed_at: row.try_get("observed_at").map_err(db)?,
            },
        );
    }
    let mut resolved = Vec::new();
    for item in evidence {
        let index = usize::try_from(item.support_index)
            .map_err(|_| Error::Invalid("formation support index must be non-negative".into()))?;
        let Some(RevisionSupport::Evidence(value)) = supports.get(index) else {
            continue;
        };
        if item.kind == FormationEvidenceKind::Contextual && item.external_actor.is_none() {
            continue;
        }
        let occurrence = occurrence_evidence
            .get(&value.occurrence_id.0)
            .ok_or_else(|| {
                Error::FailedPrecondition("formation occurrence is not in Subject Authority".into())
            })?;
        let actor = item.external_actor.as_ref().ok_or_else(|| {
            Error::Invalid("external formation evidence requires external_actor".into())
        })?;
        if actor != &occurrence.actor {
            return Err(Error::FailedPrecondition(
                "external_actor does not match ObservationOccurrence actor".into(),
            ));
        }
        resolved.push((item, occurrence));
    }
    let has = |kind| evidence.iter().any(|value| value.kind == kind);
    if has(FormationEvidenceKind::SeedDirect)
        || has(FormationEvidenceKind::ExplicitExplanation)
        || has(FormationEvidenceKind::ExplicitConfirmation)
        || has(FormationEvidenceKind::RepairSequence)
    {
        return Ok(());
    }
    let uses = evidence
        .iter()
        .filter(|value| value.kind == FormationEvidenceKind::ExternalConsistentUse)
        .collect::<Vec<_>>();
    if uses.len() >= policy.repeated_external_use_min {
        let roots = uses
            .iter()
            .filter_map(|value| {
                let index = usize::try_from(value.support_index).ok()?;
                let RevisionSupport::Evidence(support) = supports.get(index)? else {
                    return None;
                };
                occurrence_evidence
                    .get(&support.occurrence_id.0)
                    .and_then(|item| item.root.clone())
            })
            .collect::<BTreeSet<_>>();
        if roots.len() >= policy.repeated_external_use_min && roots.len() == uses.len() {
            return Ok(());
        }
    }
    if has(FormationEvidenceKind::SuccessfulUnderstanding)
        && has(FormationEvidenceKind::ExternalConsistentUse)
    {
        let success = resolved
            .iter()
            .find(|(value, _)| value.kind == FormationEvidenceKind::SuccessfulUnderstanding);
        let continued = resolved
            .iter()
            .filter(|(value, _)| value.kind == FormationEvidenceKind::ExternalConsistentUse)
            .find(|(_, occurrence)| {
                success.is_some_and(|(_, success_occurrence)| {
                    occurrence.observed_at >= success_occurrence.observed_at
                        && occurrence.root.is_some()
                        && occurrence.root != success_occurrence.root
                })
            });
        if let (Some((_, success_occurrence)), Some((_, continued_occurrence))) =
            (success, continued)
            && (!policy.require_same_actor_after_successful_understanding
                || success_occurrence.actor == continued_occurrence.actor)
        {
            return Ok(());
        }
    }
    Err(Error::FailedPrecondition(
        "LanguageConvention formation evidence is insufficient".into(),
    ))
}

pub(super) async fn existing_receipt(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
    digest: &str,
) -> Result<Option<Option<Uuid>>> {
    let row = sqlx::query("SELECT request_digest,result_ref,state FROM mutation_receipts WHERE subject_id=$1 AND operation_id=$2 FOR UPDATE")
        .bind(subject.0).bind(operation.0).fetch_optional(&mut **tx).await.map_err(db)?;
    if let Some(row) = row {
        let existing: String = row.try_get("request_digest").map_err(db)?;
        if existing != digest {
            return Err(Error::Conflict(
                "Social operation_id was used with a different request".into(),
            ));
        }
        if row.try_get::<String, _>("state").map_err(db)? == "committed" {
            return Ok(Some(
                row.try_get::<Option<String>, _>("result_ref")
                    .map_err(db)?
                    .and_then(|value| Uuid::parse_str(&value).ok()),
            ));
        }
        return Err(Error::Unavailable(
            "Social operation is already in progress".into(),
        ));
    }
    sqlx::query("INSERT INTO mutation_receipts(subject_id,operation_id,operation_kind,request_digest,state,created_at) VALUES($1,$2,'social',$3,'in_progress',$4)")
        .bind(subject.0).bind(operation.0).bind(digest).bind(Utc::now()).execute(&mut **tx).await.map_err(db)?;
    Ok(None)
}

pub(super) async fn commit_receipt(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
    kind: &str,
    reference: &str,
    revision: Option<Uuid>,
    epoch: Option<i64>,
) -> Result<()> {
    sqlx::query("UPDATE mutation_receipts SET state='committed',result_kind=$3,result_ref=$4,result_revision=$5,result_epoch=$6,committed_at=$7 WHERE subject_id=$1 AND operation_id=$2")
        .bind(subject.0).bind(operation.0).bind(kind).bind(reference).bind(revision).bind(epoch).bind(Utc::now()).execute(&mut **tx).await.map_err(db)?;
    Ok(())
}

pub(super) async fn lock_operation(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
    operation: OperationId,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("{}:{}", subject.0, operation.0))
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}

pub(super) async fn bump_subject(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<()> {
    sqlx::query("UPDATE subjects SET authority_seq=authority_seq+1 WHERE subject_id=$1")
        .bind(subject.0)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}

pub(super) async fn invalidate(
    tx: &mut Transaction<'_, Postgres>,
    subject: SubjectId,
) -> Result<()> {
    AuthorityStore::invalidate_in(tx, subject, ProjectionInvalidation::text())
        .await
        .map(|_| ())
}

pub(super) fn parse_enum<T: for<'de> Deserialize<'de>>(value: String) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| Error::Infrastructure(error.to_string()))
}

pub(super) fn enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub(super) fn load_relation_type_row(
    row: &sqlx::postgres::PgRow,
    subject: SubjectId,
) -> Result<RelationTypeDefinition> {
    let view_kind: String = row.try_get("view_kind").map_err(db)?;
    let view_semantics = match view_kind.as_str() {
        "directed" => ViewSemantics::Directed,
        "symmetric" => ViewSemantics::SymmetricView,
        "inverse" => ViewSemantics::InverseView {
            inverse_key: row
                .try_get::<Option<String>, _>("inverse_key")
                .map_err(db)?
                .ok_or_else(|| {
                    Error::Infrastructure("inverse relation type has no inverse key".into())
                })?,
        },
        _ => return Err(Error::Infrastructure("invalid relation type view".into())),
    };
    let degree_kind: String = row.try_get("degree_kind").map_err(db)?;
    let config: serde_json::Value = row.try_get("degree_config").map_err(db)?;
    let degree_semantics = match degree_kind.as_str() {
        "none" => DegreeSemantics::None,
        "ordinal" => DegreeSemantics::Ordinal {
            levels: serde_json::from_value(config)
                .map_err(|error| Error::Infrastructure(error.to_string()))?,
        },
        "bounded_scalar" => DegreeSemantics::BoundedScalar {
            min: config
                .get("min")
                .and_then(|value| value.as_f64())
                .ok_or_else(|| Error::Infrastructure("bounded degree min missing".into()))?,
            max: config
                .get("max")
                .and_then(|value| value.as_f64())
                .ok_or_else(|| Error::Infrastructure("bounded degree max missing".into()))?,
        },
        "typed_state" => DegreeSemantics::TypedState {
            states: serde_json::from_value(config)
                .map_err(|error| Error::Infrastructure(error.to_string()))?,
        },
        _ => return Err(Error::Infrastructure("invalid relation type degree".into())),
    };
    Ok(RelationTypeDefinition {
        relation_type_id: RelationTypeId(row.try_get("relation_type_id").map_err(db)?),
        subject_id: subject,
        key: row.try_get("key").map_err(db)?,
        allowed_from_kinds: row.try_get("allowed_from_kinds").map_err(db)?,
        allowed_to_kinds: row.try_get("allowed_to_kinds").map_err(db)?,
        view_semantics,
        degree_semantics,
        temporal_semantics: parse_temporal(row.try_get("temporal_kind").map_err(db)?)?,
        source_seed_version_id: row
            .try_get::<Option<Uuid>, _>("source_seed_version_id")
            .map_err(db)?
            .map(CognitiveSeedVersionId),
        source_seed_path: row.try_get("source_seed_path").map_err(db)?,
        created_at: row.try_get("created_at").map_err(db)?,
    })
}

pub(super) fn degree_columns(value: &DegreeSemantics) -> Result<(&'static str, serde_json::Value)> {
    Ok(match value {
        DegreeSemantics::None => ("none", serde_json::json!({})),
        DegreeSemantics::Ordinal { levels } => (
            "ordinal",
            serde_json::to_value(levels).map_err(|error| Error::Internal(error.to_string()))?,
        ),
        DegreeSemantics::BoundedScalar { min, max } => (
            "bounded_scalar",
            serde_json::json!({"min": min, "max": max}),
        ),
        DegreeSemantics::TypedState { states } => (
            "typed_state",
            serde_json::to_value(states).map_err(|error| Error::Internal(error.to_string()))?,
        ),
    })
}

pub(super) fn view_columns(value: &ViewSemantics) -> (&'static str, Option<&str>) {
    match value {
        ViewSemantics::Directed => ("directed", None),
        ViewSemantics::SymmetricView => ("symmetric", None),
        ViewSemantics::InverseView { inverse_key } => ("inverse", Some(inverse_key.as_str())),
    }
}

pub(super) fn party_columns(value: &SocialParty) -> (&'static str, Option<&str>) {
    match value {
        SocialParty::SubjectSelf => ("subject", None),
        SocialParty::Entity {
            entity_kind,
            entity_ref,
        } => (entity_kind.as_str(), Some(entity_ref.as_str())),
    }
}

pub(super) fn scope_columns(value: &SocialScope) -> (&'static str, Vec<String>) {
    match value {
        SocialScope::Person(reference) => ("person", vec![reference.as_str().to_owned()]),
        SocialScope::Dyad(left, right) => ("dyad", vec![left.canonical(), right.canonical()]),
        SocialScope::Group(reference) => ("group", vec![reference.as_str().to_owned()]),
        SocialScope::Community(reference) => ("community", vec![reference.as_str().to_owned()]),
        SocialScope::Channel(reference) => ("channel", vec![reference.as_str().to_owned()]),
    }
}

pub(super) fn scope_from_columns(kind: String, refs: Vec<String>) -> Result<SocialScope> {
    let reference = |value: &String| EntityRef::new(value.clone());
    match kind.as_str() {
        "person" => Ok(SocialScope::Person(reference(refs.first().ok_or_else(
            || Error::Infrastructure("person scope reference missing".into()),
        )?)?)),
        "group" => Ok(SocialScope::Group(reference(refs.first().ok_or_else(
            || Error::Infrastructure("group scope reference missing".into()),
        )?)?)),
        "community" => Ok(SocialScope::Community(reference(
            refs.first()
                .ok_or_else(|| Error::Infrastructure("community scope reference missing".into()))?,
        )?)),
        "channel" => Ok(SocialScope::Channel(reference(refs.first().ok_or_else(
            || Error::Infrastructure("channel scope reference missing".into()),
        )?)?)),
        "dyad" => {
            if refs.len() != 2 {
                return Err(Error::Infrastructure(
                    "dyad scope references are incomplete".into(),
                ));
            }
            Ok(SocialScope::Dyad(
                parse_canonical_party(&refs[0])?,
                parse_canonical_party(&refs[1])?,
            ))
        }
        _ => Err(Error::Infrastructure("invalid social scope kind".into())),
    }
}

pub(super) fn parse_canonical_party(value: &str) -> Result<SocialParty> {
    if value == "subject" {
        return Ok(SocialParty::SubjectSelf);
    }
    let (kind, reference) = value
        .split_once(':')
        .ok_or_else(|| Error::Infrastructure("invalid canonical social party".into()))?;
    let entity_kind = match kind {
        "person" => SocialEntityKind::Person,
        "group" => SocialEntityKind::Group,
        "community" => SocialEntityKind::Community,
        "channel" => SocialEntityKind::Channel,
        _ => {
            return Err(Error::Infrastructure(
                "invalid canonical social party kind".into(),
            ));
        }
    };
    Ok(SocialParty::Entity {
        entity_kind,
        entity_ref: EntityRef::new(reference.to_owned())?,
    })
}

pub(super) fn party_from_columns(kind: String, reference: Option<String>) -> Result<SocialParty> {
    if kind == "subject" {
        return Ok(SocialParty::SubjectSelf);
    }
    let entity_kind = match kind.as_str() {
        "person" => SocialEntityKind::Person,
        "group" => SocialEntityKind::Group,
        "community" => SocialEntityKind::Community,
        "channel" => SocialEntityKind::Channel,
        _ => return Err(Error::Infrastructure("invalid social party kind".into())),
    };
    Ok(SocialParty::Entity {
        entity_kind,
        entity_ref: EntityRef::new(
            reference.ok_or_else(|| Error::Infrastructure("social entity ref missing".into()))?,
        )?,
    })
}

pub(super) fn validate_party_against(allowed: &[String], party: &SocialParty) -> Result<()> {
    let kind = match party {
        SocialParty::SubjectSelf => "subject",
        SocialParty::Entity { entity_kind, .. } => entity_kind.as_str(),
    };
    if allowed.iter().any(|value| value == kind) {
        Ok(())
    } else {
        Err(Error::Invalid(
            "social party kind is not allowed by relation type".into(),
        ))
    }
}

pub(super) fn format_temporal(value: TemporalSemantics) -> &'static str {
    match value {
        TemporalSemantics::State => "state",
        TemporalSemantics::Interval => "interval",
        TemporalSemantics::Instant => "instant",
    }
}
pub(super) fn parse_temporal(value: String) -> Result<TemporalSemantics> {
    match value.as_str() {
        "state" => Ok(TemporalSemantics::State),
        "interval" => Ok(TemporalSemantics::Interval),
        "instant" => Ok(TemporalSemantics::Instant),
        _ => Err(Error::Infrastructure(
            "invalid social temporal semantics".into(),
        )),
    }
}
pub(super) fn validate_temporal_kind(
    kind: TemporalSemantics,
    value: &TemporalExtent,
) -> Result<()> {
    if matches!(
        (kind, value),
        (
            TemporalSemantics::State,
            TemporalExtent::Unknown | TemporalExtent::Interval { .. }
        ) | (TemporalSemantics::Interval, TemporalExtent::Interval { .. })
            | (TemporalSemantics::Instant, TemporalExtent::Instant { .. })
    ) {
        Ok(())
    } else {
        Err(Error::Invalid(
            "relationship valid_time conflicts with Relation Type temporal semantics".into(),
        ))
    }
}
pub(super) fn temporal_columns(
    value: &TemporalExtent,
) -> (&'static str, Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    match value {
        TemporalExtent::Unknown => ("unknown", None, None),
        TemporalExtent::Instant { at } => ("instant", Some(*at), None),
        TemporalExtent::Interval { start, end } => ("interval", *start, *end),
    }
}
pub(super) fn temporal_from_columns(
    kind: String,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<TemporalExtent> {
    match kind.as_str() {
        "unknown" => Ok(TemporalExtent::Unknown),
        "instant" => Ok(TemporalExtent::Instant {
            at: start.ok_or_else(|| Error::Infrastructure("instant timestamp missing".into()))?,
        }),
        "interval" => Ok(TemporalExtent::Interval { start, end }),
        _ => Err(Error::Infrastructure(
            "invalid social temporal value".into(),
        )),
    }
}
pub(super) fn format_evidence_kind(value: FormationEvidenceKind) -> &'static str {
    match value {
        FormationEvidenceKind::SeedDirect => "seed_direct",
        FormationEvidenceKind::ExplicitExplanation => "explicit_explanation",
        FormationEvidenceKind::ExplicitConfirmation => "explicit_confirmation",
        FormationEvidenceKind::ExternalConsistentUse => "external_consistent_use",
        FormationEvidenceKind::SuccessfulUnderstanding => "successful_understanding",
        FormationEvidenceKind::RepairSequence => "repair_sequence",
        FormationEvidenceKind::Contextual => "contextual",
    }
}

pub(super) fn seed_operation(
    subject: SubjectId,
    seed: CognitiveSeedVersionId,
    path: &str,
) -> OperationId {
    let name = format!("{}\n{}\n{}", subject.0, seed.0, path);
    OperationId(Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes()))
}

pub(super) fn seed_view(value: String, inverse_key: Option<String>) -> Result<ViewSemantics> {
    match value.as_str() {
        "directed" => Ok(ViewSemantics::Directed),
        "symmetric" => Ok(ViewSemantics::SymmetricView),
        "inverse" => Ok(ViewSemantics::InverseView {
            inverse_key: inverse_key
                .ok_or_else(|| Error::Invalid("inverse relation type needs inverse_key".into()))?,
        }),
        _ => Err(Error::Invalid("invalid relation type view".into())),
    }
}

pub(super) fn seed_degree(
    value: Option<nous_cognitive_seed::DegreeSeed>,
) -> Result<DegreeSemantics> {
    let Some(value) = value else {
        return Ok(DegreeSemantics::None);
    };
    match value.kind.as_str() {
        "none" => Ok(DegreeSemantics::None),
        "ordinal" => Ok(DegreeSemantics::Ordinal {
            levels: value.levels,
        }),
        "bounded_scalar" => Ok(DegreeSemantics::BoundedScalar {
            min: value
                .min
                .ok_or_else(|| Error::Invalid("degree min required".into()))?,
            max: value
                .max
                .ok_or_else(|| Error::Invalid("degree max required".into()))?,
        }),
        "typed_state" => Ok(DegreeSemantics::TypedState {
            states: value.states,
        }),
        _ => Err(Error::Invalid("invalid degree kind".into())),
    }
}

pub(super) fn seed_temporal(value: &str) -> Result<TemporalSemantics> {
    match value {
        "state" => Ok(TemporalSemantics::State),
        "interval" => Ok(TemporalSemantics::Interval),
        "instant" => Ok(TemporalSemantics::Instant),
        _ => Err(Error::Invalid("invalid temporal kind".into())),
    }
}

pub(super) fn seed_party(value: nous_cognitive_seed::SeedParty) -> Result<SocialParty> {
    if value.kind == "subject" {
        return Ok(SocialParty::SubjectSelf);
    }
    let reference = value
        .reference()
        .ok_or_else(|| Error::Invalid("social seed party reference required".into()))?;
    let entity_kind = match value.kind.as_str() {
        "person" => SocialEntityKind::Person,
        "group" => SocialEntityKind::Group,
        "community" => SocialEntityKind::Community,
        "channel" => SocialEntityKind::Channel,
        _ => return Err(Error::Invalid("invalid social seed party kind".into())),
    };
    Ok(SocialParty::Entity {
        entity_kind,
        entity_ref: EntityRef::new(reference)?,
    })
}

pub(super) fn seed_scope(value: nous_cognitive_seed::ConventionScopeSeed) -> Result<SocialScope> {
    match value.kind.as_str() {
        "person" => Ok(SocialScope::Person(EntityRef::new(
            value
                .reference()
                .ok_or_else(|| Error::Invalid("person scope reference required".into()))?,
        )?)),
        "group" => Ok(SocialScope::Group(EntityRef::new(
            value
                .reference()
                .ok_or_else(|| Error::Invalid("group scope reference required".into()))?,
        )?)),
        "community" => Ok(SocialScope::Community(EntityRef::new(
            value
                .reference()
                .ok_or_else(|| Error::Invalid("community scope reference required".into()))?,
        )?)),
        "channel" => Ok(SocialScope::Channel(EntityRef::new(
            value
                .reference()
                .ok_or_else(|| Error::Invalid("channel scope reference required".into()))?,
        )?)),
        "dyad" if value.parties.len() == 2 => Ok(SocialScope::Dyad(
            seed_party(value.parties[0].clone())?,
            seed_party(value.parties[1].clone())?,
        )),
        _ => Err(Error::Invalid("invalid convention scope".into())),
    }
}

pub(super) fn parse_epistemic(value: &str) -> Result<EpistemicClass> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| Error::Invalid("invalid epistemic class".into()))
}

pub(super) fn seed_time(value: Option<nous_cognitive_seed::SeedTime>) -> Result<TemporalExtent> {
    let Some(value) = value else {
        return Ok(TemporalExtent::Unknown);
    };
    match value.kind.as_deref().unwrap_or("unknown") {
        "unknown" => Ok(TemporalExtent::Unknown),
        "instant" => Ok(TemporalExtent::Instant {
            at: parse_time(
                value
                    .at
                    .ok_or_else(|| Error::Invalid("instant time required".into()))?,
            )?,
        }),
        "interval" => Ok(TemporalExtent::Interval {
            start: value.start.map(parse_time).transpose()?,
            end: value.end.map(parse_time).transpose()?,
        }),
        _ => Err(Error::Invalid("invalid seed valid_time kind".into())),
    }
}

pub(super) fn parse_time(value: String) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| Error::Invalid(format!("invalid seed timestamp: {error}")))
}
