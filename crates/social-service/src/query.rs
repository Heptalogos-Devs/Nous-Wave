use super::*;
use std::collections::HashMap;

#[async_trait::async_trait]
impl nous_cognitive_runtime::CognitiveContributor for SocialService {
    fn owns(&self, reference: &CognitiveRef) -> bool {
        matches!(
            reference,
            CognitiveRef::RelationshipAssertion(_)
                | CognitiveRef::RelationshipRevision(_)
                | CognitiveRef::LanguageConvention(_)
                | CognitiveRef::LanguageConventionRevision(_)
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Social direct lanes keep relation view and convention scope ordering in one owner boundary"
    )]
    async fn direct_lanes(
        &self,
        bound: &nous_cognitive_runtime::BoundQuery,
        plan: &nous_cognitive_runtime::QueryPlan,
    ) -> Result<Vec<nous_cognitive_runtime::LaneOutput>> {
        let query = &bound.source_query;
        if !plan
            .enabled_lanes
            .contains(&EvidenceFamily::SocialRelationDirect)
            && !plan
                .enabled_lanes
                .contains(&EvidenceFamily::LanguageConventionDirect)
        {
            return Ok(Vec::new());
        }
        let relation_cue = query.cues.iter().find_map(|cue| match cue {
            Cue::SocialRelation(value) => Some(value),
            _ => None,
        });
        let convention_cue = query.cues.iter().find_map(|cue| match cue {
            Cue::LanguageConvention(value) => Some(value),
            _ => None,
        });
        let entity_cues = query
            .cues
            .iter()
            .filter_map(|cue| match cue {
                Cue::Entity(value) => Some(value.entity_ref.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let social_target = query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::Social));
        let mut outputs = Vec::new();
        if plan
            .enabled_lanes
            .contains(&EvidenceFamily::SocialRelationDirect)
            && (social_target || relation_cue.is_some() || !entity_cues.is_empty())
        {
            let rows = sqlx::query("SELECT a.current_revision_id,t.key,t.view_kind,t.inverse_key,a.from_kind,a.from_entity_ref,a.to_kind,a.to_entity_ref FROM relationship_assertions a JOIN relation_type_definitions t USING(relation_type_id) JOIN relationship_revisions r ON r.relationship_revision_id=a.current_revision_id WHERE a.subject_id=$1 AND a.acceptance_state='accepted' AND a.integrity_state='valid' AND a.suppression_state='normal' AND a.purge_state='normal' ORDER BY r.recorded_at DESC,r.relationship_revision_id LIMIT $2")
                .bind(query.subject.0)
                .bind(plan.lane_budget(EvidenceFamily::SocialRelationDirect) as i64)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            let mut output = nous_cognitive_runtime::LaneOutput::empty(
                EvidenceFamily::SocialRelationDirect,
                nous_cognitive_runtime::LaneStatus::Ready,
            );
            for (index, row) in rows.into_iter().enumerate() {
                let type_key: String = row.try_get("key").map_err(db)?;
                let from_ref: Option<String> = row.try_get("from_entity_ref").map_err(db)?;
                let to_ref: Option<String> = row.try_get("to_entity_ref").map_err(db)?;
                let view_kind: String = row.try_get("view_kind").map_err(db)?;
                let inverse_key: Option<String> = row.try_get("inverse_key").map_err(db)?;
                let direct_type_match = relation_cue
                    .and_then(|cue| cue.relation_type_key.as_deref())
                    .is_none_or(|key| key == type_key);
                let direct_endpoint_match = relation_cue.is_none_or(|cue| {
                    cue.from
                        .as_ref()
                        .is_none_or(|value| from_ref.as_deref() == Some(value.as_str()))
                        && cue
                            .to
                            .as_ref()
                            .is_none_or(|value| to_ref.as_deref() == Some(value.as_str()))
                });
                let direct = direct_type_match && direct_endpoint_match;
                let reverse_type_match = relation_cue
                    .and_then(|cue| cue.relation_type_key.as_deref())
                    .is_none_or(|key| {
                        (view_kind == "inverse" && inverse_key.as_deref() == Some(key))
                            || key == type_key
                    });
                let reverse_endpoint_match = relation_cue.is_some_and(|cue| {
                    cue.from
                        .as_ref()
                        .is_some_and(|value| to_ref.as_deref() == Some(value.as_str()))
                        && cue
                            .to
                            .as_ref()
                            .is_some_and(|value| from_ref.as_deref() == Some(value.as_str()))
                });
                let view = relation_cue.is_some_and(|cue| {
                    cue.include_views
                        && reverse_type_match
                        && reverse_endpoint_match
                        && (view_kind == "symmetric" || view_kind == "inverse")
                });
                if !direct && !view {
                    continue;
                }
                if !entity_cues.is_empty()
                    && !entity_cues.iter().any(|entity| {
                        from_ref.as_deref() == Some(*entity) || to_ref.as_deref() == Some(*entity)
                    })
                {
                    continue;
                }
                let variants = if view_kind == "symmetric" && view {
                    vec!["social:symmetric-view".into()]
                } else if view {
                    vec![format!(
                        "social:inverse-view:{}",
                        relation_cue
                            .and_then(|cue| cue.relation_type_key.clone())
                            .or(inverse_key)
                            .unwrap_or(type_key.clone())
                    )]
                } else {
                    vec!["social:relationship".into()]
                };
                output
                    .candidates
                    .push(nous_cognitive_runtime::LaneCandidate {
                        reference: CognitiveRef::RelationshipRevision(RelationshipRevisionId(
                            row.try_get("current_revision_id").map_err(db)?,
                        )),
                        rank: (index + 1) as u32,
                        variants,
                        provider_metadata: serde_json::json!({"relation_type": type_key}),
                    });
            }
            outputs.push(output);
        }
        if plan
            .enabled_lanes
            .contains(&EvidenceFamily::LanguageConventionDirect)
            && (social_target || convention_cue.is_some())
        {
            let policy = resolve_social_policy(&bound.config_snapshot)?;
            let rows = sqlx::query("SELECT c.current_revision_id,c.expression,c.scope_kind,c.scope_refs,c.context_scope,c.topic_scope,r.meaning,r.pragmatic_role,r.recorded_at FROM language_conventions c JOIN language_convention_revisions r ON r.convention_revision_id=c.current_revision_id WHERE c.subject_id=$1 AND c.acceptance_state='accepted' AND c.integrity_state='valid' AND c.suppression_state='normal' AND c.purge_state='normal'")
                .bind(query.subject.0)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            let mut rows = rows;
            rows.sort_by(|left, right| {
                let left_scope: String = left.try_get("scope_kind").unwrap_or_default();
                let right_scope: String = right.try_get("scope_kind").unwrap_or_default();
                scope_rank(&policy.scope_preference, &left_scope)
                    .cmp(&scope_rank(&policy.scope_preference, &right_scope))
                    .then_with(|| {
                        let left_context: Option<String> = left.try_get("context_scope").ok();
                        let right_context: Option<String> = right.try_get("context_scope").ok();
                        right_context.is_some().cmp(&left_context.is_some())
                    })
                    .then_with(|| {
                        let left_topic: Option<String> = left.try_get("topic_scope").ok();
                        let right_topic: Option<String> = right.try_get("topic_scope").ok();
                        right_topic.is_some().cmp(&left_topic.is_some())
                    })
                    .then_with(|| {
                        let left_recorded: Option<chrono::DateTime<chrono::Utc>> =
                            left.try_get("recorded_at").ok();
                        let right_recorded: Option<chrono::DateTime<chrono::Utc>> =
                            right.try_get("recorded_at").ok();
                        right_recorded.cmp(&left_recorded)
                    })
            });
            let mut output = nous_cognitive_runtime::LaneOutput::empty(
                EvidenceFamily::LanguageConventionDirect,
                nous_cognitive_runtime::LaneStatus::Ready,
            );
            for (index, row) in rows.into_iter().enumerate() {
                let expression: String = row.try_get("expression").map_err(db)?;
                let scope_kind: String = row.try_get("scope_kind").map_err(db)?;
                let scope_refs: Vec<String> = row.try_get("scope_refs").map_err(db)?;
                let context_scope: Option<String> = row.try_get("context_scope").map_err(db)?;
                let topic_scope: Option<String> = row.try_get("topic_scope").map_err(db)?;
                if !convention_scope_applicable(
                    &scope_kind,
                    &scope_refs,
                    context_scope.as_deref(),
                    topic_scope.as_deref(),
                    query.situation.social.as_ref(),
                    convention_cue,
                ) {
                    continue;
                }
                if let Some(cue) = convention_cue
                    && (cue.expression != expression
                        || cue
                            .scope
                            .as_deref()
                            .is_some_and(|scope| scope != scope_kind))
                {
                    continue;
                }
                output
                    .candidates
                    .push(nous_cognitive_runtime::LaneCandidate {
                        reference: CognitiveRef::LanguageConventionRevision(
                            LanguageConventionRevisionId(
                                row.try_get("current_revision_id").map_err(db)?,
                            ),
                        ),
                        rank: (index + 1) as u32,
                        variants: vec!["social:language_convention".into()],
                        provider_metadata: serde_json::json!({
                            "scope": scope_kind,
                            "scope_refs": scope_refs,
                        }),
                    });
                if output.candidates.len()
                    >= plan.lane_budget(EvidenceFamily::LanguageConventionDirect)
                {
                    break;
                }
            }
            outputs.push(output);
        }
        Ok(outputs)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Social owner batch materialization keeps relationship, convention, and support reads bounded"
    )]
    async fn validate_and_materialize(
        &self,
        _subject: SubjectId,
        references: &[CognitiveRef],
        bound: &nous_cognitive_runtime::BoundQuery,
    ) -> Result<(Vec<CognitiveHit>, std::collections::BTreeMap<String, usize>)> {
        let subject = bound.source_query.subject;
        let relationship_refs = references
            .iter()
            .filter_map(|reference| match reference {
                CognitiveRef::RelationshipRevision(value) => Some(value.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let convention_refs = references
            .iter()
            .filter_map(|reference| match reference {
                CognitiveRef::LanguageConventionRevision(value) => Some(value.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut drops = std::collections::BTreeMap::new();
        let mut hits = Vec::new();
        if !relationship_refs.is_empty() {
            let rows = sqlx::query("SELECT a.current_revision_id,a.object_epoch,a.acceptance_state,a.integrity_state,a.suppression_state,a.purge_state,a.from_kind,a.from_entity_ref,a.to_kind,a.to_entity_ref,t.key,r.relationship_revision_id,r.degree_value,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at,r.epistemic_class FROM relationship_assertions a JOIN relation_type_definitions t USING(relation_type_id) JOIN relationship_revisions r ON r.relationship_id=a.relationship_id WHERE a.subject_id=$1 AND r.relationship_revision_id=ANY($2::uuid[])")
                .bind(subject.0)
                .bind(&relationship_refs)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                let revision =
                    RelationshipRevisionId(row.try_get("relationship_revision_id").map_err(db)?);
                let reference = CognitiveRef::RelationshipRevision(revision);
                if !social_reference_is_available(&row, &reference, bound, &mut drops)? {
                    continue;
                }
                let type_key: String = row.try_get("key").map_err(db)?;
                let from = format_party(
                    row.try_get("from_kind").map_err(db)?,
                    row.try_get("from_entity_ref").map_err(db)?,
                );
                let to = format_party(
                    row.try_get("to_kind").map_err(db)?,
                    row.try_get("to_entity_ref").map_err(db)?,
                );
                let degree: Option<serde_json::Value> = row.try_get("degree_value").map_err(db)?;
                let mut hit = social_hit(
                    reference,
                    format!(
                        "relationship\ntype={type_key}\nfrom={from}\nto={to}\ndegree={}",
                        degree.map_or_else(|| "none".into(), |value| value.to_string())
                    ),
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                )?;
                hit.semantic_role = Some(format!("social:relationship:{type_key}"));
                hit.entity_refs = [
                    row.try_get::<Option<String>, _>("from_entity_ref")
                        .map_err(db)?,
                    row.try_get::<Option<String>, _>("to_entity_ref")
                        .map_err(db)?,
                ]
                .into_iter()
                .flatten()
                .map(EntityRef::new)
                .collect::<Result<Vec<_>>>()?;
                hits.push(hit);
            }
        }
        if !convention_refs.is_empty() {
            let rows = sqlx::query("SELECT c.current_revision_id,c.object_epoch,c.acceptance_state,c.integrity_state,c.suppression_state,c.purge_state,c.expression,c.scope_kind,c.scope_refs,c.context_scope,c.topic_scope,r.convention_revision_id,r.meaning,r.pragmatic_role,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM language_conventions c JOIN language_convention_revisions r ON r.convention_id=c.convention_id WHERE c.subject_id=$1 AND r.convention_revision_id=ANY($2::uuid[])")
                .bind(subject.0)
                .bind(&convention_refs)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                let revision = LanguageConventionRevisionId(
                    row.try_get("convention_revision_id").map_err(db)?,
                );
                let reference = CognitiveRef::LanguageConventionRevision(revision);
                if !social_reference_is_available(&row, &reference, bound, &mut drops)? {
                    continue;
                }
                let expression: String = row.try_get("expression").map_err(db)?;
                let scope_kind: String = row.try_get("scope_kind").map_err(db)?;
                let scope_refs: Vec<String> = row.try_get("scope_refs").map_err(db)?;
                let mut hit = social_hit(
                    reference,
                    format!(
                        "language_convention\nexpression={expression}\nmeaning={}\nrole={}\nscope={}:{}",
                        row.try_get::<String, _>("meaning").map_err(db)?,
                        row.try_get::<Option<String>, _>("pragmatic_role")
                            .map_err(db)?
                            .unwrap_or_default(),
                        scope_kind,
                        scope_refs.join("|")
                    ),
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                )?;
                hit.semantic_role = Some("social:language-convention".into());
                hits.push(hit);
            }
        }
        if bound.source_query.result_need.need_evidence {
            let relationship_supports = load_support_handles(
                &self.store,
                "relationship_revision_supports",
                "relationship_revision_id",
                &relationship_refs,
            )
            .await?;
            let convention_supports = load_support_handles(
                &self.store,
                "language_convention_revision_supports",
                "convention_revision_id",
                &convention_refs,
            )
            .await?;
            for hit in &mut hits {
                let supports = match hit.reference {
                    CognitiveRef::RelationshipRevision(id) => relationship_supports.get(&id.0),
                    CognitiveRef::LanguageConventionRevision(id) => convention_supports.get(&id.0),
                    _ => None,
                };
                if let Some(supports) = supports {
                    hit.evidence = supports.clone();
                }
            }
        }
        Ok((hits, drops))
    }
}

async fn load_support_handles(
    store: &AuthorityStore,
    table: &str,
    revision_column: &str,
    revisions: &[Uuid],
) -> Result<HashMap<Uuid, Vec<EvidenceHandle>>> {
    if revisions.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT {revision_column} AS revision_id,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id FROM {table} WHERE {revision_column}=ANY($1::uuid[]) ORDER BY {revision_column},support_kind,support_ref,support_role"
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(revisions)
        .fetch_all(store.pool())
        .await
        .map_err(db)?;
    let mut result = HashMap::new();
    for row in rows {
        let revision: Uuid = row.try_get("revision_id").map_err(db)?;
        let kind: String = row.try_get("support_kind").map_err(db)?;
        let support_ref: String = row.try_get("support_ref").map_err(db)?;
        let reference = if kind == "evidence" {
            if let Some(value) = row
                .try_get::<Option<Uuid>, _>("source_region_id")
                .map_err(db)?
            {
                CognitiveRef::SourceRegion(SourceRegionId(value))
            } else if let Some(value) = row
                .try_get::<Option<Uuid>, _>("derived_representation_id")
                .map_err(db)?
            {
                CognitiveRef::DerivedRepresentation(DerivedRepresentationId(value))
            } else if let Some(value) = row
                .try_get::<Option<Uuid>, _>("derived_region_id")
                .map_err(db)?
            {
                CognitiveRef::DerivedRegion(DerivedRegionId(value))
            } else if let Some(value) = row
                .try_get::<Option<Uuid>, _>("occurrence_id")
                .map_err(db)?
            {
                CognitiveRef::Occurrence(OccurrenceId(value))
            } else {
                return Err(Error::Infrastructure(
                    "social support has no evidence locator".into(),
                ));
            }
        } else if kind == "cognitive_seed_version" {
            CognitiveRef::CognitiveSeedVersion(CognitiveSeedVersionId(
                support_ref
                    .parse()
                    .map_err(|_| Error::Infrastructure("invalid Seed support reference".into()))?,
            ))
        } else {
            parse_reference(&kind, &support_ref)?
        };
        result
            .entry(revision)
            .or_insert_with(Vec::new)
            .push(EvidenceHandle {
                reference,
                support_role: row.try_get("support_role").map_err(db)?,
            });
    }
    Ok(result)
}

fn scope_rank(preference: &[String], value: &str) -> usize {
    preference
        .iter()
        .position(|item| item == value)
        .unwrap_or(usize::MAX)
}

fn convention_scope_applicable(
    scope_kind: &str,
    scope_refs: &[String],
    context_scope: Option<&str>,
    topic_scope: Option<&str>,
    situation: Option<&SocialSituation>,
    cue: Option<&LanguageConventionCue>,
) -> bool {
    if let Some(cue) = cue
        && cue
            .scope
            .as_deref()
            .is_some_and(|scope| scope != scope_kind)
    {
        return false;
    }
    let Some(situation) = situation else {
        return cue.is_none_or(|value| value.scope.is_some());
    };
    let entity = |value: &str| {
        situation
            .participants
            .iter()
            .any(|item| item.as_str() == value)
    };
    let scope_match = match scope_kind {
        "person" => scope_refs.iter().any(|value| entity(value)),
        "dyad" => {
            let subject_present = scope_refs.iter().any(|value| value == "subject");
            let external_count = scope_refs.iter().filter(|value| entity(value)).count();
            (subject_present && external_count >= 1) || external_count >= 2
        }
        "group" => scope_refs
            .iter()
            .any(|value| situation.groups.iter().any(|item| item.as_str() == value)),
        "community" => scope_refs.iter().any(|value| {
            situation
                .communities
                .iter()
                .any(|item| item.as_str() == value)
        }),
        "channel" => situation
            .channel
            .as_ref()
            .is_some_and(|value| scope_refs.iter().any(|item| item == value.as_str())),
        _ => false,
    };
    scope_match
        && context_scope
            .is_none_or(|value| situation.context_tokens.iter().any(|token| token == value))
        && topic_scope.is_none_or(|value| situation.topic_tokens.iter().any(|token| token == value))
}

fn social_reference_is_available(
    row: &sqlx::postgres::PgRow,
    reference: &CognitiveRef,
    bound: &nous_cognitive_runtime::BoundQuery,
    drops: &mut std::collections::BTreeMap<String, usize>,
) -> Result<bool> {
    let historical = bound.revision_policy.allows_historical(reference);
    let current: Uuid = row.try_get("current_revision_id").map_err(db)?;
    let revision = match reference {
        CognitiveRef::RelationshipRevision(value) => value.0,
        CognitiveRef::LanguageConventionRevision(value) => value.0,
        _ => return Ok(false),
    };
    if !historical && current != revision {
        *drops.entry("not_current".into()).or_default() += 1;
        return Ok(false);
    }
    let state_ok = row.try_get::<String, _>("acceptance_state").map_err(db)? == "accepted"
        && row.try_get::<String, _>("integrity_state").map_err(db)? == "valid"
        && row.try_get::<String, _>("suppression_state").map_err(db)? == "normal"
        && row.try_get::<String, _>("purge_state").map_err(db)? == "normal";
    if !state_ok {
        *drops.entry("unavailable_lifecycle".into()).or_default() += 1;
        return Ok(false);
    }
    if let Some(binding) = bound
        .exact_bindings
        .iter()
        .find(|binding| binding.bound_ref == *reference)
        && binding.mutable_object
        && binding.bound_object_epoch != Some(row.try_get::<i64, _>("object_epoch").map_err(db)?)
    {
        *drops.entry("stale_exact_binding".into()).or_default() += 1;
        return Ok(false);
    }
    Ok(true)
}

fn social_hit(
    reference: CognitiveRef,
    representation: String,
    valid_kind: String,
    valid_start: Option<DateTime<Utc>>,
    valid_end: Option<DateTime<Utc>>,
    formed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
) -> Result<CognitiveHit> {
    Ok(CognitiveHit {
        revision: Some(reference.clone()),
        reference,
        semantic_role: Some("social:cognition".into()),
        cognitive_role: None,
        formation_mode: None,
        representation: Some(representation),
        authority: AuthorityClass::SubjectCognition,
        freshness: FreshnessDescriptor {
            observed_at: None,
            valid_time: temporal_from_columns(valid_kind, valid_start, valid_end)?,
            formed_at: Some(formed_at),
            recorded_at: Some(recorded_at),
        },
        entity_refs: Vec::new(),
        evidence: Vec::new(),
        match_evidence: MatchEvidence::default(),
        materialization: Vec::new(),
    })
}

fn format_party(kind: String, reference: Option<String>) -> String {
    match reference {
        Some(value) => format!("{kind}:{value}"),
        None => kind,
    }
}
