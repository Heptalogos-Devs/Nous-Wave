use super::*;

impl SocialService {
    #[expect(
        clippy::too_many_lines,
        reason = "Seed adoption keeps deterministic ordering and unchanged/conflict outcomes together"
    )]
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
                source_seed_version_id: Some(seed_version_id),
                source_seed_path: Some(path.clone()),
            };
            if let Some(existing) = self.relation_type_by_key(subject, &definition.key).await? {
                if relation_type_matches(&existing, &definition) {
                    result.unchanged.push(path);
                } else {
                    result.conflicts.push(path);
                }
            } else {
                match self.register_relation_type(definition).await {
                    Ok(_) => result.created.push(path),
                    Err(error) => return Err(error),
                }
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
            if self.relationship_exists(&input).await? {
                if self.relationship_seed_matches(&input).await? {
                    result.unchanged.push(path);
                } else {
                    result.conflicts.push(path);
                }
            } else {
                match self.create_relationship(input).await {
                    Ok(_) => result.created.push(path),
                    Err(error) => return Err(error),
                }
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
            if let Some(existing) = self.convention_by_key(subject, &input.key).await? {
                let seed = input.supports.iter().find_map(|support| match support {
                    RevisionSupport::Seed(value) => Some(value),
                    _ => None,
                });
                let seed_matches = if let Some(value) = seed {
                    self.convention_seed_matches(subject, &input.key, value)
                        .await?
                } else {
                    false
                };
                if convention_matches(&existing, &input) && seed_matches {
                    result.unchanged.push(path);
                } else {
                    result.conflicts.push(path);
                }
            } else {
                match self.create_language_convention(input).await {
                    Ok(_) => result.created.push(path),
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(result)
    }
}
