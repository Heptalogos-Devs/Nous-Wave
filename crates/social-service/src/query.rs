use super::*;
use std::collections::BTreeMap;

impl SocialService {
    #[expect(
        clippy::excessive_nesting,
        clippy::collapsible_if,
        reason = "Social direct-lane validation keeps endpoint and scope applicability checks adjacent to the Authority query"
    )]
    async fn legacy_contribute(
        &self,
        bound: &nous_cognitive_runtime::BoundQuery,
        plan: &nous_cognitive_runtime::QueryPlan,
    ) -> Result<CognitiveQueryResult> {
        let query = &bound.source_query;
        let social_target = query
            .targets
            .iter()
            .any(|target| matches!(target, QueryTarget::Social));
        let relation_cue = query.cues.iter().find_map(|cue| match cue {
            Cue::SocialRelation(value) => Some(value),
            _ => None,
        });
        let convention_cue = query.cues.iter().find_map(|cue| match cue {
            Cue::LanguageConvention(value) => Some(value),
            _ => None,
        });
        let mut results = Vec::new();
        if social_target || relation_cue.is_some() {
            let rows = sqlx::query("SELECT a.relationship_id,a.current_revision_id,t.key,t.temporal_kind,a.from_kind,a.from_entity_ref,a.to_kind,a.to_entity_ref,r.degree_value,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM relationship_assertions a JOIN relation_type_definitions t USING(relation_type_id) JOIN relationship_revisions r ON r.relationship_revision_id=a.current_revision_id WHERE a.subject_id=$1 AND a.acceptance_state='accepted' AND a.integrity_state='valid' AND a.suppression_state='normal' AND a.purge_state='normal' ORDER BY r.recorded_at DESC,r.relationship_revision_id LIMIT $2")
                .bind(query.subject.0)
                .bind(plan.lane_budget(EvidenceFamily::SocialRelationDirect) as i64)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                let type_key: String = row.try_get("key").map_err(db)?;
                if let Some(cue) = relation_cue
                    && cue
                        .relation_type_key
                        .as_deref()
                        .is_some_and(|key| key != type_key)
                {
                    continue;
                }
                let from_kind: String = row.try_get("from_kind").map_err(db)?;
                let from_ref: Option<String> = row.try_get("from_entity_ref").map_err(db)?;
                let to_kind: String = row.try_get("to_kind").map_err(db)?;
                let to_ref: Option<String> = row.try_get("to_entity_ref").map_err(db)?;
                if let Some(cue) = relation_cue {
                    if cue
                        .from
                        .as_ref()
                        .is_some_and(|value| from_ref.as_deref() != Some(value.as_str()))
                        || cue
                            .to
                            .as_ref()
                            .is_some_and(|value| to_ref.as_deref() != Some(value.as_str()))
                    {
                        continue;
                    }
                }
                let revision =
                    RelationshipRevisionId(row.try_get("current_revision_id").map_err(db)?);
                let reference = CognitiveRef::RelationshipRevision(revision);
                let degree: Option<serde_json::Value> = row.try_get("degree_value").map_err(db)?;
                let from = format_party(from_kind, from_ref);
                let to = format_party(to_kind, to_ref);
                results.push(social_hit(
                    reference,
                    format!(
                        "relationship\ntype={type_key}\nfrom={from}\nto={to}\ndegree={}",
                        degree.map_or_else(|| "none".into(), |value| value.to_string())
                    ),
                    EvidenceFamily::SocialRelationDirect,
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                )?);
            }
        }
        if social_target || convention_cue.is_some() {
            let rows = sqlx::query("SELECT c.current_revision_id,c.expression,c.scope_kind,c.scope_refs,c.context_scope,c.topic_scope,r.meaning,r.pragmatic_role,r.valid_time_kind,r.valid_time_start,r.valid_time_end,r.formed_at,r.recorded_at FROM language_conventions c JOIN language_convention_revisions r ON r.convention_revision_id=c.current_revision_id WHERE c.subject_id=$1 AND c.acceptance_state='accepted' AND c.integrity_state='valid' AND c.suppression_state='normal' AND c.purge_state='normal' ORDER BY r.recorded_at DESC,r.convention_revision_id LIMIT $2")
                .bind(query.subject.0)
                .bind(plan.lane_budget(EvidenceFamily::LanguageConventionDirect) as i64)
                .fetch_all(self.store.pool())
                .await
                .map_err(db)?;
            for row in rows {
                let expression: String = row.try_get("expression").map_err(db)?;
                if let Some(cue) = convention_cue {
                    if cue.expression != expression
                        || cue.scope.as_deref().is_some_and(|scope| {
                            row.try_get::<String, _>("scope_kind").ok().as_deref() != Some(scope)
                        })
                    {
                        continue;
                    }
                }
                let reference = CognitiveRef::LanguageConventionRevision(
                    LanguageConventionRevisionId(row.try_get("current_revision_id").map_err(db)?),
                );
                let scope_kind: String = row.try_get("scope_kind").map_err(db)?;
                let scope_refs: Vec<String> = row.try_get("scope_refs").map_err(db)?;
                results.push(social_hit(
                    reference,
                    format!("language_convention\nexpression={expression}\nmeaning={}\nrole={}\nscope={}:{}", row.try_get::<String, _>("meaning").map_err(db)?, row.try_get::<Option<String>, _>("pragmatic_role").map_err(db)?.unwrap_or_default(), scope_kind, scope_refs.join("|")),
                    EvidenceFamily::LanguageConventionDirect,
                    row.try_get("valid_time_kind").map_err(db)?,
                    row.try_get("valid_time_start").map_err(db)?,
                    row.try_get("valid_time_end").map_err(db)?,
                    row.try_get("formed_at").map_err(db)?,
                    row.try_get("recorded_at").map_err(db)?,
                )?);
            }
        }
        results.truncate(query.result_need.limit);
        Ok(CognitiveQueryResult {
            query_id: bound.query_id,
            generation: QueryGenerationTrace::default(),
            status: QueryStatus::Complete,
            results,
            resource_actions: Vec::new(),
            degradation: Vec::new(),
            diagnostics: None,
        })
    }
}

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

    async fn direct_lanes(
        &self,
        bound: &nous_cognitive_runtime::BoundQuery,
        plan: &nous_cognitive_runtime::QueryPlan,
    ) -> Result<Vec<nous_cognitive_runtime::LaneOutput>> {
        let result = self.legacy_contribute(bound, plan).await?;
        let mut outputs = BTreeMap::<EvidenceFamily, nous_cognitive_runtime::LaneOutput>::new();
        for hit in result.results {
            for family in &hit.match_evidence.families {
                outputs
                    .entry(*family)
                    .or_insert_with(|| {
                        nous_cognitive_runtime::LaneOutput::empty(
                            *family,
                            nous_cognitive_runtime::LaneStatus::Ready,
                        )
                    })
                    .candidates
                    .push(nous_cognitive_runtime::LaneCandidate {
                        reference: hit.reference.clone(),
                        rank: hit.match_evidence.best_lane_rank,
                        variants: hit.match_evidence.variants.clone(),
                        provider_metadata: serde_json::Value::Null,
                    });
            }
        }
        Ok(outputs.into_values().collect())
    }

    async fn validate_and_materialize(
        &self,
        _subject: SubjectId,
        references: &[CognitiveRef],
        bound: &nous_cognitive_runtime::BoundQuery,
    ) -> Result<(Vec<CognitiveHit>, std::collections::BTreeMap<String, usize>)> {
        let result = self
            .legacy_contribute(
                bound,
                &nous_cognitive_runtime::QueryPlan::for_bound_query(bound),
            )
            .await?;
        let hits = result
            .results
            .into_iter()
            .filter(|hit| references.contains(&hit.reference))
            .map(|mut hit| {
                hit.match_evidence = MatchEvidence::default();
                hit
            })
            .collect();
        Ok((hits, std::collections::BTreeMap::new()))
    }
}

fn social_hit(
    reference: CognitiveRef,
    representation: String,
    family: EvidenceFamily,
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
        match_evidence: MatchEvidence {
            families: vec![family],
            base_rank_score: 1.0,
            best_lane_rank: 1,
            enabled_lane_count: 1,
            final_score: 1.0,
            variants: Vec::new(),
            explanation: None,
        },
        materialization: Vec::new(),
    })
}

fn format_party(kind: String, reference: Option<String>) -> String {
    match reference {
        Some(value) => format!("{kind}:{value}"),
        None => kind,
    }
}
