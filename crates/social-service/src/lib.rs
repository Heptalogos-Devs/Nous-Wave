//! Social Cognition Authority and direct query owner.

mod persistence;
mod query;
use persistence::*;

use chrono::{DateTime, Utc};
use nous_authority_store::{AuthorityStore, ProjectionInvalidation, database_error as db};
use nous_configuration_service::{
    ConfigApplyMode, ConfigExposure, ConfigKey, ConfigRegistryBuilder, ConfigScopePolicy,
    ConfigSemanticEffect, ConfigSnapshot,
};
use nous_core::*;
use nous_social_domain::*;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const REPEATED_EXTERNAL_USE_MIN: ConfigKey<usize> =
    ConfigKey::new("social.language_convention.repeated_external_use_min");
pub const REQUIRE_SAME_ACTOR: ConfigKey<bool> =
    ConfigKey::new("social.language_convention.require_same_actor_after_successful_understanding");
pub const SCOPE_PREFERENCE: ConfigKey<Vec<String>> =
    ConfigKey::new("social.query.scope_preference");

#[derive(Debug, Clone)]
pub struct SocialPolicy {
    pub repeated_external_use_min: usize,
    pub require_same_actor_after_successful_understanding: bool,
    pub scope_preference: Vec<String>,
}

impl SocialPolicy {
    fn validate(&self) -> Result<()> {
        if self.repeated_external_use_min < 2
            || self.scope_preference.len() != 5
            || self.scope_preference.iter().collect::<BTreeSet<_>>().len() != 5
            || self.scope_preference.iter().any(|value| {
                !matches!(
                    value.as_str(),
                    "dyad" | "person" | "channel" | "group" | "community"
                )
            })
        {
            return Err(Error::Invalid("invalid Social policy".into()));
        }
        Ok(())
    }
}

pub fn register_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        REPEATED_EXTERNAL_USE_MIN,
        "social-service",
        "Independent external uses required for a language convention.",
        2,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |value| {
            if *value >= 2 {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "repeated_external_use_min must be at least 2".into(),
                ))
            }
        },
    )?;
    registry.register(
        REQUIRE_SAME_ACTOR,
        "social-service",
        "Require the same external actor after successful understanding.",
        true,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::AuthorityFormation,
        |_: &bool| Ok(()),
    )?;
    registry.register(
        SCOPE_PREFERENCE,
        "social-service",
        "Scope preference for applicable language conventions.",
        vec!["dyad", "person", "channel", "group", "community"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        ConfigExposure::Advanced,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::QueryPolicy,
        |value: &Vec<String>| {
            if value.len() == 5
                && value.iter().collect::<BTreeSet<_>>().len() == 5
                && value.iter().all(|item| {
                    matches!(
                        item.as_str(),
                        "dyad" | "person" | "channel" | "group" | "community"
                    )
                })
            {
                Ok(())
            } else {
                Err(Error::Invalid(
                    "scope_preference must be a permutation of SocialScope kinds".into(),
                ))
            }
        },
    )?;
    Ok(())
}

pub fn resolve_social_policy(snapshot: &ConfigSnapshot) -> Result<SocialPolicy> {
    let policy = SocialPolicy {
        repeated_external_use_min: snapshot.get(REPEATED_EXTERNAL_USE_MIN)?,
        require_same_actor_after_successful_understanding: snapshot.get(REQUIRE_SAME_ACTOR)?,
        scope_preference: snapshot.get(SCOPE_PREFERENCE)?,
    };
    policy.validate()?;
    Ok(policy)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRelationType {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub key: String,
    pub allowed_from_kinds: Vec<String>,
    pub allowed_to_kinds: Vec<String>,
    pub view_semantics: ViewSemantics,
    pub degree_semantics: DegreeSemantics,
    pub temporal_semantics: TemporalSemantics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationship {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub relation_type_key: String,
    pub from: SocialParty,
    pub to: SocialParty,
    pub degree: Option<serde_json::Value>,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseRelationship {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub relationship_id: RelationshipAssertionId,
    pub expected_object_epoch: i64,
    pub parent_revision_id: RelationshipRevisionId,
    pub revision_intent: String,
    pub degree: Option<serde_json::Value>,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLanguageConvention {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub key: String,
    pub expression: String,
    pub scope: SocialScope,
    pub context_scope: Option<String>,
    pub topic_scope: Option<String>,
    pub meaning: String,
    pub pragmatic_role: Option<String>,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub formation_evidence: Vec<ConventionFormationEvidence>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseLanguageConvention {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub convention_id: LanguageConventionId,
    pub expected_object_epoch: i64,
    pub parent_revision_id: LanguageConventionRevisionId,
    pub revision_intent: String,
    pub meaning: String,
    pub pragmatic_role: Option<String>,
    pub epistemic_class: EpistemicClass,
    pub valid_time: TemporalExtent,
    pub formed_at: DateTime<Utc>,
    pub supports: Vec<RevisionSupport>,
    pub formation_evidence: Vec<ConventionFormationEvidence>,
    pub producer_signature_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateSocialLifecycle {
    pub operation_id: OperationId,
    pub subject: SubjectId,
    pub reference: CognitiveRef,
    pub expected_object_epoch: i64,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialRelationshipView {
    pub assertion: RelationshipAssertion,
    pub revision: RelationshipRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialConventionView {
    pub convention: LanguageConvention,
    pub revision: LanguageConventionRevision,
    pub formation_evidence: Vec<ConventionFormationEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocialSeedImportResult {
    pub created: Vec<String>,
    pub unchanged: Vec<String>,
    pub conflicts: Vec<String>,
}

#[derive(Clone)]
pub struct SocialService {
    pub store: AuthorityStore,
    pub configuration: nous_configuration_service::ConfigurationService,
}

impl SocialService {
    pub async fn import_seed(
        &self,
        subject: SubjectId,
        seed_version_id: CognitiveSeedVersionId,
        text: &str,
    ) -> Result<SocialSeedImportResult> {
        let document = nous_cognitive_seed::parse(text)?;
        let mut result = SocialSeedImportResult::default();
        let Some(social) = document.social else {
            return Ok(result);
        };
        for relation in social.relation_types {
            let path = format!("social.relation-types/{}", relation.key);
            let operation_id = seed_operation(subject, seed_version_id, &path);
            let definition = RegisterRelationType {
                operation_id,
                subject,
                key: relation.key,
                allowed_from_kinds: relation.allowed_from,
                allowed_to_kinds: relation.allowed_to,
                view_semantics: seed_view(relation.view, relation.inverse_key)?,
                degree_semantics: seed_degree(relation.degree)?,
                temporal_semantics: seed_temporal(&relation.temporal)?,
            };
            match self.register_relation_type(definition).await {
                Ok(_) => result.created.push(path),
                Err(Error::Conflict(_)) => result.conflicts.push(path),
                Err(error) => return Err(error),
            }
        }
        for relationship in social.relationships {
            let path = format!("social.relationships/{}", relationship.key);
            let operation_id = seed_operation(subject, seed_version_id, &path);
            let input = CreateRelationship {
                operation_id,
                subject,
                relation_type_key: relationship.relation_type,
                from: seed_party(relationship.from)?,
                to: seed_party(relationship.to)?,
                degree: relationship
                    .degree
                    .map(|value| {
                        serde_json::to_value(value)
                            .map_err(|error| Error::Invalid(error.to_string()))
                    })
                    .transpose()?,
                epistemic_class: parse_epistemic(&relationship.epistemic)?,
                valid_time: seed_time(relationship.valid_time)?,
                formed_at: Utc::now(),
                supports: vec![RevisionSupport::Seed(SeedSupportRef::new(
                    seed_version_id,
                    path.clone(),
                )?)],
                producer_signature_id: None,
            };
            match self.create_relationship(input).await {
                Ok(_) => result.created.push(path),
                Err(Error::Conflict(_)) => result.conflicts.push(path),
                Err(error) => return Err(error),
            }
        }
        for convention in social.conventions {
            let path = format!("social.conventions/{}", convention.key);
            let operation_id = seed_operation(subject, seed_version_id, &path);
            let input = CreateLanguageConvention {
                operation_id,
                subject,
                key: convention.key,
                expression: convention.expression,
                scope: seed_scope(convention.scope)?,
                context_scope: convention.context,
                topic_scope: convention.topic,
                meaning: convention.meaning,
                pragmatic_role: convention.pragmatic_role,
                epistemic_class: parse_epistemic(&convention.epistemic)?,
                valid_time: TemporalExtent::Unknown,
                formed_at: Utc::now(),
                supports: vec![RevisionSupport::Seed(SeedSupportRef::new(
                    seed_version_id,
                    path.clone(),
                )?)],
                formation_evidence: vec![ConventionFormationEvidence {
                    support_index: 0,
                    kind: FormationEvidenceKind::SeedDirect,
                    external_actor: None,
                }],
                producer_signature_id: None,
            };
            match self.create_language_convention(input).await {
                Ok(_) => result.created.push(path),
                Err(Error::Conflict(_)) => result.conflicts.push(path),
                Err(error) => return Err(error),
            }
        }
        Ok(result)
    }

    pub fn new(
        store: AuthorityStore,
        configuration: nous_configuration_service::ConfigurationService,
    ) -> Self {
        Self {
            store,
            configuration,
        }
    }

    pub async fn register_relation_type(
        &self,
        input: RegisterRelationType,
    ) -> Result<RelationTypeDefinition> {
        validate_social_key(&input.key, "relation type key")?;
        let definition = RelationTypeDefinition {
            relation_type_id: RelationTypeId::new(),
            subject_id: input.subject,
            key: input.key,
            allowed_from_kinds: input.allowed_from_kinds,
            allowed_to_kinds: input.allowed_to_kinds,
            view_semantics: input.view_semantics,
            degree_semantics: input.degree_semantics,
            temporal_semantics: input.temporal_semantics,
            created_at: Utc::now(),
        };
        definition.validate()?;
        self.store.require_subject(input.subject).await?;
        let digest =
            canonical_request_digest("social.relation_type.register", input.subject, &definition)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(result) =
            existing_receipt(&mut tx, input.subject, input.operation_id, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            let id = result
                .ok_or_else(|| Error::Infrastructure("Social receipt has no result".into()))?;
            return self.relation_type(input.subject, RelationTypeId(id)).await;
        }
        let existing = sqlx::query("SELECT relation_type_id,key,allowed_from_kinds,allowed_to_kinds,view_kind,inverse_key,degree_kind,degree_config,temporal_kind,created_at FROM relation_type_definitions WHERE subject_id=$1 AND key=$2")
            .bind(input.subject.0).bind(&definition.key).fetch_optional(&mut *tx).await.map_err(db)?;
        if let Some(row) = existing {
            let existing_def = load_relation_type_row(&row, input.subject)?;
            if serde_json::to_value(&existing_def)
                .map_err(|error| Error::Internal(error.to_string()))?
                != serde_json::to_value(&definition)
                    .map_err(|error| Error::Internal(error.to_string()))?
            {
                return Err(Error::Conflict(
                    "relation type definition conflicts with existing key".into(),
                ));
            }
            commit_receipt(
                &mut tx,
                input.subject,
                input.operation_id,
                "relation_type",
                &existing_def.relation_type_id.0.to_string(),
                None,
                None,
            )
            .await?;
            tx.commit().await.map_err(db)?;
            return Ok(existing_def);
        }
        let (view_kind, inverse_key) = view_columns(&definition.view_semantics);
        let (degree_kind, degree_config) = degree_columns(&definition.degree_semantics)?;
        sqlx::query("INSERT INTO relation_type_definitions(relation_type_id,subject_id,key,allowed_from_kinds,allowed_to_kinds,view_kind,inverse_key,degree_kind,degree_config,temporal_kind,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(definition.relation_type_id.0).bind(input.subject.0).bind(&definition.key).bind(&definition.allowed_from_kinds).bind(&definition.allowed_to_kinds).bind(view_kind).bind(inverse_key).bind(degree_kind).bind(degree_config).bind(format_temporal(definition.temporal_semantics)).bind(definition.created_at).execute(&mut *tx).await.map_err(db)?;
        bump_subject(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "relation_type",
            &definition.relation_type_id.0.to_string(),
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(definition)
    }

    pub async fn create_relationship(
        &self,
        input: CreateRelationship,
    ) -> Result<SocialRelationshipView> {
        validate_relationship_identity(&input.from, &input.to)?;
        validate_exact_supports(&input.supports)?;
        let policy_snapshot = self.configuration.snapshot_for_subject(input.subject)?;
        let _ = resolve_social_policy(&policy_snapshot)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("social.relationship.create", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(result) =
            existing_receipt(&mut tx, input.subject, input.operation_id, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            let id = result
                .ok_or_else(|| Error::Infrastructure("Social receipt has no result".into()))?;
            return self
                .relationship(input.subject, RelationshipAssertionId(id))
                .await;
        }
        let type_row = sqlx::query("SELECT relation_type_id,key,allowed_from_kinds,allowed_to_kinds,view_kind,inverse_key,degree_kind,degree_config,temporal_kind,created_at FROM relation_type_definitions WHERE subject_id=$1 AND key=$2")
            .bind(input.subject.0).bind(&input.relation_type_key).fetch_optional(&mut *tx).await.map_err(db)?.ok_or_else(|| Error::NotFound("relation type not found".into()))?;
        let relation_type = load_relation_type_row(&type_row, input.subject)?;
        validate_party_against(&relation_type.allowed_from_kinds, &input.from)?;
        validate_party_against(&relation_type.allowed_to_kinds, &input.to)?;
        validate_temporal_kind(relation_type.temporal_semantics, &input.valid_time)?;
        let (from_kind, from_ref) = party_columns(&input.from);
        let (to_kind, to_ref) = party_columns(&input.to);
        let duplicate: Option<Uuid> = sqlx::query_scalar("SELECT relationship_id FROM relationship_assertions WHERE subject_id=$1 AND relation_type_id=$2 AND from_kind=$3 AND COALESCE(from_entity_ref,'')=COALESCE($4,'') AND to_kind=$5 AND COALESCE(to_entity_ref,'')=COALESCE($6,'')")
            .bind(input.subject.0).bind(relation_type.relation_type_id.0).bind(from_kind).bind(from_ref).bind(to_kind).bind(to_ref).fetch_optional(&mut *tx).await.map_err(db)?;
        if duplicate.is_some() {
            return Err(Error::Conflict(
                "relationship assertion identity already exists".into(),
            ));
        }
        let relationship_id = RelationshipAssertionId::new();
        let revision_id = RelationshipRevisionId::new();
        let now = Utc::now();
        sqlx::query("INSERT INTO relationship_assertions(relationship_id,subject_id,relation_type_id,from_kind,from_entity_ref,to_kind,to_entity_ref,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,1,'accepted','valid','normal','normal',$9)")
            .bind(relationship_id.0).bind(input.subject.0).bind(relation_type.relation_type_id.0).bind(from_kind).bind(from_ref).bind(to_kind).bind(to_ref).bind(revision_id.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        insert_relationship_revision(
            &mut tx,
            &input,
            relationship_id,
            revision_id,
            1,
            None,
            None,
            now,
        )
        .await?;
        invalidate(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "relationship",
            &relationship_id.0.to_string(),
            Some(revision_id.0),
            Some(1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.relationship(input.subject, relationship_id).await
    }

    pub async fn create_language_convention(
        &self,
        input: CreateLanguageConvention,
    ) -> Result<SocialConventionView> {
        validate_social_key(&input.key, "language convention key")?;
        if input.expression.trim().is_empty() || input.meaning.trim().is_empty() {
            return Err(Error::Invalid(
                "language convention expression and meaning are required".into(),
            ));
        }
        validate_exact_supports(&input.supports)?;
        let scope = input.scope.clone().canonicalize()?;
        let snapshot = self.configuration.snapshot_for_subject(input.subject)?;
        let policy = resolve_social_policy(&snapshot)?;
        accept_formation(&input.formation_evidence, &input.supports, &policy)?;
        let formation_digest =
            snapshot.digest_for(&[REPEATED_EXTERNAL_USE_MIN.path(), REQUIRE_SAME_ACTOR.path()])?;
        let digest =
            canonical_request_digest("social.language_convention.create", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(result) =
            existing_receipt(&mut tx, input.subject, input.operation_id, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            let id = result
                .ok_or_else(|| Error::Infrastructure("Social receipt has no result".into()))?;
            return self
                .convention(input.subject, LanguageConventionId(id))
                .await;
        }
        let duplicate: Option<Uuid> = sqlx::query_scalar(
            "SELECT convention_id FROM language_conventions WHERE subject_id=$1 AND key=$2",
        )
        .bind(input.subject.0)
        .bind(&input.key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if duplicate.is_some() {
            return Err(Error::Conflict(
                "LanguageConvention key already exists".into(),
            ));
        }
        let convention_id = LanguageConventionId::new();
        let revision_id = LanguageConventionRevisionId::new();
        let now = Utc::now();
        let (scope_kind, scope_refs) = scope_columns(&scope);
        sqlx::query("INSERT INTO language_conventions(convention_id,subject_id,key,expression,scope_kind,scope_refs,context_scope,topic_scope,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,1,'accepted','valid','normal','normal',$10)")
            .bind(convention_id.0).bind(input.subject.0).bind(&input.key).bind(&input.expression).bind(scope_kind).bind(scope_refs).bind(&input.context_scope).bind(&input.topic_scope).bind(revision_id.0).bind(now).execute(&mut *tx).await.map_err(db)?;
        insert_convention_revision(
            &mut tx,
            &input,
            convention_id,
            revision_id,
            formation_digest,
            now,
        )
        .await?;
        for evidence in &input.formation_evidence {
            sqlx::query("INSERT INTO language_convention_formation_evidence(convention_revision_id,support_ordinal,evidence_kind,external_actor_ref) VALUES($1,$2,$3,$4)")
                .bind(revision_id.0).bind(evidence.support_index).bind(format_evidence_kind(evidence.kind)).bind(evidence.external_actor.as_ref().map(|value| value.as_str())).execute(&mut *tx).await.map_err(db)?;
        }
        invalidate(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "language_convention",
            &convention_id.0.to_string(),
            Some(revision_id.0),
            Some(1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.convention(input.subject, convention_id).await
    }

    pub async fn revise_relationship(
        &self,
        input: ReviseRelationship,
    ) -> Result<SocialRelationshipView> {
        validate_exact_supports(&input.supports)?;
        self.store.require_subject(input.subject).await?;
        let digest = canonical_request_digest("social.relationship.revise", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(result) =
            existing_receipt(&mut tx, input.subject, input.operation_id, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            let id = result
                .ok_or_else(|| Error::Infrastructure("Social receipt has no result".into()))?;
            return self
                .relationship(input.subject, RelationshipAssertionId(id))
                .await;
        }
        let row = sqlx::query("SELECT relation_type_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state FROM relationship_assertions WHERE subject_id=$1 AND relationship_id=$2 FOR UPDATE")
            .bind(input.subject.0)
            .bind(input.relationship_id.0)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("relationship not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
        if epoch != input.expected_object_epoch || current != input.parent_revision_id.0 {
            return Err(Error::Conflict(
                "relationship revision fence is stale".into(),
            ));
        }
        if row.try_get::<String, _>("purge_state").map_err(db)? != "normal"
            || row.try_get::<String, _>("suppression_state").map_err(db)? != "normal"
        {
            return Err(Error::FailedPrecondition(
                "relationship is not active".into(),
            ));
        }
        let type_id = RelationTypeId(row.try_get("relation_type_id").map_err(db)?);
        let type_row = sqlx::query("SELECT relation_type_id,key,allowed_from_kinds,allowed_to_kinds,view_kind,inverse_key,degree_kind,degree_config,temporal_kind,created_at FROM relation_type_definitions WHERE subject_id=$1 AND relation_type_id=$2")
            .bind(input.subject.0)
            .bind(type_id.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let relation_type = load_relation_type_row(&type_row, input.subject)?;
        validate_temporal_kind(relation_type.temporal_semantics, &input.valid_time)?;
        let revision_id = RelationshipRevisionId::new();
        let next: i32 = sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM relationship_revisions WHERE relationship_id=$1")
            .bind(input.relationship_id.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO relationship_revisions(relationship_revision_id,relationship_id,subject_id,revision_no,parent_revision_id,revision_intent,degree_kind,degree_value,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id) VALUES($1,$2,$3,$4,$5,$6,NULL,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(revision_id.0)
            .bind(input.relationship_id.0)
            .bind(input.subject.0)
            .bind(next)
            .bind(input.parent_revision_id.0)
            .bind(&input.revision_intent)
            .bind(&input.degree)
            .bind(enum_name(input.epistemic_class))
            .bind(valid_kind)
            .bind(valid_start)
            .bind(valid_end)
            .bind(input.formed_at)
            .bind(Utc::now())
            .bind(input.producer_signature_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        insert_supports(
            &mut tx,
            "relationship_revision_supports",
            "relationship_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        sqlx::query("UPDATE relationship_assertions SET current_revision_id=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND relationship_id=$2")
            .bind(input.subject.0)
            .bind(input.relationship_id.0)
            .bind(revision_id.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        invalidate(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "relationship",
            &input.relationship_id.0.to_string(),
            Some(revision_id.0),
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.relationship(input.subject, input.relationship_id)
            .await
    }

    pub async fn revise_language_convention(
        &self,
        input: ReviseLanguageConvention,
    ) -> Result<SocialConventionView> {
        validate_social_key(&input.revision_intent, "revision intent")?;
        validate_exact_supports(&input.supports)?;
        let snapshot = self.configuration.snapshot_for_subject(input.subject)?;
        let policy = resolve_social_policy(&snapshot)?;
        accept_formation(&input.formation_evidence, &input.supports, &policy)?;
        let digest =
            canonical_request_digest("social.language_convention.revise", input.subject, &input)?;
        let formation_digest =
            snapshot.digest_for(&[REPEATED_EXTERNAL_USE_MIN.path(), REQUIRE_SAME_ACTOR.path()])?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if let Some(result) =
            existing_receipt(&mut tx, input.subject, input.operation_id, &digest).await?
        {
            tx.commit().await.map_err(db)?;
            let id = result
                .ok_or_else(|| Error::Infrastructure("Social receipt has no result".into()))?;
            return self
                .convention(input.subject, LanguageConventionId(id))
                .await;
        }
        let row = sqlx::query("SELECT current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state FROM language_conventions WHERE subject_id=$1 AND convention_id=$2 FOR UPDATE")
            .bind(input.subject.0)
            .bind(input.convention_id.0)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("LanguageConvention not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
        if epoch != input.expected_object_epoch || current != input.parent_revision_id.0 {
            return Err(Error::Conflict(
                "LanguageConvention revision fence is stale".into(),
            ));
        }
        let revision_id = LanguageConventionRevisionId::new();
        let next: i32 = sqlx::query_scalar("SELECT COALESCE(max(revision_no),0)+1 FROM language_convention_revisions WHERE convention_id=$1")
            .bind(input.convention_id.0)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let (valid_kind, valid_start, valid_end) = temporal_columns(&input.valid_time);
        sqlx::query("INSERT INTO language_convention_revisions(convention_revision_id,convention_id,subject_id,revision_no,parent_revision_id,revision_intent,meaning,pragmatic_role,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at,producer_signature_id,formation_policy_digest) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
            .bind(revision_id.0)
            .bind(input.convention_id.0)
            .bind(input.subject.0)
            .bind(next)
            .bind(input.parent_revision_id.0)
            .bind(&input.revision_intent)
            .bind(&input.meaning)
            .bind(&input.pragmatic_role)
            .bind(enum_name(input.epistemic_class))
            .bind(valid_kind)
            .bind(valid_start)
            .bind(valid_end)
            .bind(input.formed_at)
            .bind(Utc::now())
            .bind(input.producer_signature_id)
            .bind(formation_digest)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        insert_supports(
            &mut tx,
            "language_convention_revision_supports",
            "convention_revision_id",
            revision_id.0,
            &input.supports,
        )
        .await?;
        for evidence in &input.formation_evidence {
            sqlx::query("INSERT INTO language_convention_formation_evidence(convention_revision_id,support_ordinal,evidence_kind,external_actor_ref) VALUES($1,$2,$3,$4)")
                .bind(revision_id.0)
                .bind(evidence.support_index)
                .bind(format_evidence_kind(evidence.kind))
                .bind(evidence.external_actor.as_ref().map(|value| value.as_str()))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        sqlx::query("UPDATE language_conventions SET current_revision_id=$3,object_epoch=object_epoch+1 WHERE subject_id=$1 AND convention_id=$2")
            .bind(input.subject.0)
            .bind(input.convention_id.0)
            .bind(revision_id.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        invalidate(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            "language_convention",
            &input.convention_id.0.to_string(),
            Some(revision_id.0),
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.convention(input.subject, input.convention_id).await
    }

    pub async fn mutate_lifecycle(&self, input: MutateSocialLifecycle) -> Result<()> {
        let digest = canonical_request_digest("social.lifecycle", input.subject, &input)?;
        let mut tx = self.store.begin().await?;
        lock_operation(&mut tx, input.subject, input.operation_id).await?;
        if existing_receipt(&mut tx, input.subject, input.operation_id, &digest)
            .await?
            .is_some()
        {
            tx.commit().await.map_err(db)?;
            return Ok(());
        }
        let (kind, id) = match input.reference {
            CognitiveRef::RelationshipAssertion(id) => ("relationship", id.0),
            CognitiveRef::RelationshipRevision(id) => ("relationship", sqlx::query_scalar("SELECT relationship_id FROM relationship_revisions WHERE subject_id=$1 AND relationship_revision_id=$2").bind(input.subject.0).bind(id.0).fetch_one(&mut *tx).await.map_err(db)?),
            CognitiveRef::LanguageConvention(id) => ("convention", id.0),
            CognitiveRef::LanguageConventionRevision(id) => ("convention", sqlx::query_scalar("SELECT convention_id FROM language_convention_revisions WHERE subject_id=$1 AND convention_revision_id=$2").bind(input.subject.0).bind(id.0).fetch_one(&mut *tx).await.map_err(db)?),
            _ => return Err(Error::Invalid("Social lifecycle requires a Social reference".into())),
        };
        let table = if kind == "relationship" {
            "relationship_assertions"
        } else {
            "language_conventions"
        };
        let id_column = if kind == "relationship" {
            "relationship_id"
        } else {
            "convention_id"
        };
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT object_epoch FROM {table} WHERE subject_id=$1 AND {id_column}=$2 FOR UPDATE"
        )))
        .bind(input.subject.0)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| Error::NotFound("Social object not found".into()))?;
        let epoch: i64 = row.try_get("object_epoch").map_err(db)?;
        if epoch != input.expected_object_epoch {
            return Err(Error::Conflict("Social object epoch is stale".into()));
        }
        if input.operation == "purge" {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE subject_id=$1 AND {id_column}=$2"
            )))
            .bind(input.subject.0)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            let assignments = match input.operation.as_str() {
                "withdraw" => "acceptance_state='withdrawn'",
                "reaccept" => "acceptance_state='accepted'",
                "suppress" => "suppression_state='suppressed'",
                "restore" => "suppression_state='normal'",
                "mark_revalidation_required" => "integrity_state='revalidation_required'",
                "restore_valid" => "integrity_state='valid'",
                _ => {
                    return Err(Error::Invalid(
                        "unsupported Social lifecycle operation".into(),
                    ));
                }
            };
            sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {table} SET {assignments},object_epoch=object_epoch+1 WHERE subject_id=$1 AND {id_column}=$2")))
                .bind(input.subject.0).bind(id).execute(&mut *tx).await.map_err(db)?;
        }
        invalidate(&mut tx, input.subject).await?;
        commit_receipt(
            &mut tx,
            input.subject,
            input.operation_id,
            kind,
            &id.to_string(),
            None,
            Some(epoch + 1),
        )
        .await?;
        tx.commit().await.map_err(db)
    }

    pub async fn relationship(
        &self,
        subject: SubjectId,
        id: RelationshipAssertionId,
    ) -> Result<SocialRelationshipView> {
        let row = sqlx::query("SELECT a.relation_type_id,a.from_kind,a.from_entity_ref,a.to_kind,a.to_entity_ref,a.current_revision_id,a.object_epoch,a.acceptance_state,a.integrity_state,a.suppression_state,a.purge_state,a.created_at,r.revision_no,r.parent_revision_id,r.revision_intent,r.degree_kind,r.degree_value,r.epistemic_class,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id FROM relationship_assertions a JOIN relationship_revisions r ON r.relationship_revision_id=a.current_revision_id WHERE a.subject_id=$1 AND a.relationship_id=$2")
            .bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(|| Error::NotFound("relationship not found".into()))?;
        let revision_id = RelationshipRevisionId(row.try_get("current_revision_id").map_err(db)?);
        let from = party_from_columns(
            row.try_get("from_kind").map_err(db)?,
            row.try_get("from_entity_ref").map_err(db)?,
        )?;
        let to = party_from_columns(
            row.try_get("to_kind").map_err(db)?,
            row.try_get("to_entity_ref").map_err(db)?,
        )?;
        Ok(SocialRelationshipView {
            assertion: RelationshipAssertion {
                relationship_id: id,
                subject_id: subject,
                relation_type_id: RelationTypeId(row.try_get("relation_type_id").map_err(db)?),
                from,
                to,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: row.try_get("acceptance_state").map_err(db)?,
                integrity_state: row.try_get("integrity_state").map_err(db)?,
                suppression_state: row.try_get("suppression_state").map_err(db)?,
                purge_state: row.try_get("purge_state").map_err(db)?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision: RelationshipRevision {
                relationship_revision_id: revision_id,
                relationship_id: id,
                subject_id: subject,
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(RelationshipRevisionId),
                revision_intent: row.try_get("revision_intent").map_err(db)?,
                degree: row.try_get("degree_value").map_err(db)?,
                epistemic_class: parse_enum(row.try_get("epistemic_class").map_err(db)?)?,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                supports: Vec::new(),
            },
        })
    }

    pub async fn relation_type(
        &self,
        subject: SubjectId,
        id: RelationTypeId,
    ) -> Result<RelationTypeDefinition> {
        let row = sqlx::query("SELECT relation_type_id,key,allowed_from_kinds,allowed_to_kinds,view_kind,inverse_key,degree_kind,degree_config,temporal_kind,created_at FROM relation_type_definitions WHERE subject_id=$1 AND relation_type_id=$2")
            .bind(subject.0)
            .bind(id.0)
            .fetch_optional(self.store.pool())
            .await
            .map_err(db)?
            .ok_or_else(|| Error::NotFound("relation type not found".into()))?;
        load_relation_type_row(&row, subject)
    }

    pub async fn convention(
        &self,
        subject: SubjectId,
        id: LanguageConventionId,
    ) -> Result<SocialConventionView> {
        let row = sqlx::query("SELECT c.key,c.expression,c.scope_kind,c.scope_refs,c.context_scope,c.topic_scope,c.current_revision_id,c.object_epoch,c.acceptance_state,c.integrity_state,c.suppression_state,c.purge_state,c.created_at,r.revision_no,r.parent_revision_id,r.revision_intent,r.meaning,r.pragmatic_role,r.epistemic_class,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.producer_signature_id,r.formation_policy_digest FROM language_conventions c JOIN language_convention_revisions r ON r.convention_revision_id=c.current_revision_id WHERE c.subject_id=$1 AND c.convention_id=$2")
            .bind(subject.0).bind(id.0).fetch_optional(self.store.pool()).await.map_err(db)?.ok_or_else(|| Error::NotFound("LanguageConvention not found".into()))?;
        let revision_id =
            LanguageConventionRevisionId(row.try_get("current_revision_id").map_err(db)?);
        let scope = scope_from_columns(
            row.try_get("scope_kind").map_err(db)?,
            row.try_get("scope_refs").map_err(db)?,
        )?;
        Ok(SocialConventionView {
            convention: LanguageConvention {
                convention_id: id,
                subject_id: subject,
                key: row.try_get("key").map_err(db)?,
                expression: row.try_get("expression").map_err(db)?,
                scope,
                context_scope: row.try_get("context_scope").map_err(db)?,
                topic_scope: row.try_get("topic_scope").map_err(db)?,
                current_revision_id: revision_id,
                object_epoch: row.try_get("object_epoch").map_err(db)?,
                acceptance_state: row.try_get("acceptance_state").map_err(db)?,
                integrity_state: row.try_get("integrity_state").map_err(db)?,
                suppression_state: row.try_get("suppression_state").map_err(db)?,
                purge_state: row.try_get("purge_state").map_err(db)?,
                created_at: row.try_get("created_at").map_err(db)?,
            },
            revision: LanguageConventionRevision {
                convention_revision_id: revision_id,
                convention_id: id,
                subject_id: subject,
                revision_no: row.try_get("revision_no").map_err(db)?,
                parent_revision_id: row
                    .try_get::<Option<Uuid>, _>("parent_revision_id")
                    .map_err(db)?
                    .map(LanguageConventionRevisionId),
                revision_intent: row.try_get("revision_intent").map_err(db)?,
                meaning: row.try_get("meaning").map_err(db)?,
                pragmatic_role: row.try_get("pragmatic_role").map_err(db)?,
                epistemic_class: parse_enum(row.try_get("epistemic_class").map_err(db)?)?,
                valid_time: temporal_from_columns(
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                )?,
                formed_at: row.try_get("formed_at").map_err(db)?,
                recorded_at: row.try_get("recorded_at").map_err(db)?,
                producer_signature_id: row.try_get("producer_signature_id").map_err(db)?,
                formation_policy_digest: row.try_get("formation_policy_digest").map_err(db)?,
                supports: Vec::new(),
            },
            formation_evidence: Vec::new(),
        })
    }
}
