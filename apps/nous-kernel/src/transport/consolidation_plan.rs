use super::*;
use nous_core::*;
use sqlx::Row;
use std::collections::BTreeSet;

impl KernelService {
    pub(super) async fn consolidation_context(&self, plan: &mut k::MaintenancePlan) -> Result<()> {
        let subject = SubjectId(id(&plan.subject_id)?);
        let snapshot = self.0.configuration.snapshot_for_subject(subject)?;
        let policy = snapshot.get(nous_memory::CONSOLIDATION_CONTEXT)?;
        let source = from_ref(required(
            required(plan.consolidation_source.clone(), "source")?.reference,
            "source reference",
        )?)?;
        let text = plan
            .journal
            .as_ref()
            .map(|journal| journal.narrative.as_str())
            .or_else(|| plan.members.first().map(|member| member.text.as_str()))
            .unwrap_or("");
        let cue: String = text.chars().take(policy.query_cue_chars).collect();
        let query = consolidation_query(subject, cue, policy.candidate_limit);
        let result = Box::pin(self.0.query(query)).await?;
        let schema_ids: Vec<uuid::Uuid> = result
            .results
            .iter()
            .filter_map(
                |hit| match hit.revision.as_ref().unwrap_or(&hit.reference) {
                    CognitiveRef::CognitiveSchemaRevision(id) => Some(id.0),
                    _ => None,
                },
            )
            .collect();
        let schemas: std::collections::BTreeMap<uuid::Uuid, sqlx::postgres::PgRow> = sqlx::query("SELECT schema_revision_id,formation_kind,title,applicability_description,boundary_definition,tags FROM cognitive_schema_revisions WHERE schema_revision_id=ANY($1::uuid[])")
            .bind(schema_ids).fetch_all(self.0.store.pool()).await.map_err(nous_persistence::database_error)?
            .into_iter().map(|row| (row.get("schema_revision_id"), row)).collect();
        let references: Vec<_> = result
            .results
            .iter()
            .map(|hit| hit.revision.as_ref().unwrap_or(&hit.reference).clone())
            .collect();
        let mut use_summaries = self
            .consolidation_use_summaries(subject, &references)
            .await?;
        let mut scopes = vec![source];
        let mut entities = BTreeSet::new();
        for member in &plan.members {
            if let Some(entity) = &member.actor_entity_ref {
                entities.insert(entity.clone());
            }
        }
        for hit in result.results {
            let reference = hit.revision.unwrap_or(hit.reference);
            if !matches!(
                reference,
                CognitiveRef::MemoryRevision(_) | CognitiveRef::CognitiveSchemaRevision(_)
            ) {
                continue;
            }
            let Some(epoch) = hit.authority_epoch else {
                continue;
            };
            entities.extend(
                hit.entity_refs
                    .iter()
                    .map(|entity| entity.as_str().to_owned()),
            );
            scopes.push(reference.clone());
            let mut text = hit.representation.unwrap_or_default();
            let formation = if let CognitiveRef::CognitiveSchemaRevision(id) = &reference {
                if let Some(schema) = schemas.get(&id.0) {
                    text = format!(
                        "Title: {}\nClaim: {}\nApplicability: {}\nBoundary: {}\nTags: {:?}",
                        schema.get::<Option<String>, _>("title").unwrap_or_default(),
                        text,
                        schema.get::<String, _>("applicability_description"),
                        schema.get::<String, _>("boundary_definition"),
                        schema.get::<Vec<uuid::Uuid>, _>("tags")
                    );
                    schema.get::<String, _>("formation_kind")
                } else {
                    String::new()
                }
            } else {
                hit.formation_mode.unwrap_or_default()
            };
            let text: String = text.chars().take(policy.candidate_text_chars).collect();
            let uses = use_summaries
                .remove(&reference.to_string())
                .unwrap_or_default();
            plan.candidates.push(k::ConsolidationCandidate {
                key: reference.to_string(),
                target: Some(k::ExpectedCognition {
                    reference: Some(to_ref(reference)),
                    expected_epoch: epoch,
                }),
                text,
                cognitive_role: hit.cognitive_role.unwrap_or_default(),
                formation_mode: formation,
                entity_refs: hit
                    .entity_refs
                    .into_iter()
                    .map(|entity| entity.as_str().to_owned())
                    .collect(),
                r#use: uses,
                valid_time: Some(temporal_proto(&hit.freshness.valid_time)),
                formed_at: hit.freshness.formed_at.map(timestamp),
                recorded_at: hit.freshness.recorded_at.map(timestamp),
            });
        }
        plan.max_consolidation_actions =
            snapshot.get(nous_memory::CONSOLIDATION_MAX_ACTIONS)? as u32;
        self.finish_consolidation_catalog(plan, subject, &scopes, entities, &policy)
            .await
    }

    async fn finish_consolidation_catalog(
        &self,
        plan: &mut k::MaintenancePlan,
        subject: SubjectId,
        scopes: &[CognitiveRef],
        entities: BTreeSet<String>,
        policy: &nous_memory::ConsolidationContextPolicy,
    ) -> Result<()> {
        let memory = self.require_memory()?;
        let supports = memory
            .consolidation_catalog_supports(subject, scopes)
            .await?;
        let provenance = memory.provenance_summary(subject, &supports).await?;
        plan.support_catalog_partial = supports.len() > policy.support_limit;
        plan.provenance_roots_partial = provenance.roots.len() > policy.provenance_root_limit;
        plan.provenance_roots = provenance
            .roots
            .into_iter()
            .take(policy.provenance_root_limit)
            .map(|root| k::ProvenanceRoot {
                key: root.root_key,
                certainty: enum_name(root.certainty),
            })
            .collect();
        let source_support = RevisionSupport::CognitionDependency(CognitionDependency {
            target_revision: scopes[0].clone(),
            support_role: SupportRole::Direct,
        });
        let source_key = source_support.canonical_key();
        let bounded = std::iter::once(source_support)
            .chain(
                supports
                    .into_iter()
                    .filter(|support| support.canonical_key() != source_key),
            )
            .take(policy.support_limit);
        plan.supports = bounded
            .map(|support| k::SupportCatalogEntry {
                key: support.canonical_key(),
                support: Some(support_proto(support)),
            })
            .collect();
        plan.entities = entities
            .into_iter()
            .take(policy.entity_limit)
            .enumerate()
            .map(|(index, entity_ref)| k::ConsolidationEntity {
                key: format!("entity{index}"),
                entity_ref,
            })
            .collect();
        Ok(())
    }
    async fn consolidation_use_summaries(
        &self,
        subject: SubjectId,
        references: &[CognitiveRef],
    ) -> Result<std::collections::BTreeMap<String, Vec<k::UseSummary>>> {
        let (kinds, values): (Vec<_>, Vec<_>) = references.iter().map(reference_parts).unzip();
        let rows = sqlx::query("SELECT e.ref_kind,e.ref_value,e.use_kind,COUNT(*) AS count,MAX(e.occurred_at) AS last_used FROM cognitive_use_events e JOIN unnest($2::text[],$3::text[]) refs(kind,value) ON e.ref_kind=refs.kind AND e.ref_value=refs.value WHERE e.subject_id=$1 AND e.use_kind<>'presented' GROUP BY e.ref_kind,e.ref_value,e.use_kind ORDER BY e.ref_kind,e.ref_value,e.use_kind")
            .bind(subject.0).bind(kinds).bind(values).fetch_all(self.0.store.pool()).await.map_err(nous_persistence::database_error)?;
        let mut result = std::collections::BTreeMap::<String, Vec<k::UseSummary>>::new();
        for row in rows {
            let key = format!(
                "{}:{}",
                row.get::<String, _>("ref_kind"),
                row.get::<String, _>("ref_value")
            );
            result.entry(key).or_default().push(k::UseSummary {
                kind: row.get("use_kind"),
                count: row.get::<i64, _>("count") as u64,
                last_used_at: Some(timestamp(row.get("last_used"))),
            });
        }
        Ok(result)
    }
}

fn consolidation_query(subject: SubjectId, cue: String, limit: usize) -> CognitiveQuery {
    let constraints = QueryConstraints::default();
    CognitiveQuery {
        text_only_compatibility: false,
        work_context: None,
        api_version: 1,
        subject,
        session: None,
        situation: Default::default(),
        expression: CognitiveQueryExpr {
            targets: vec![QueryTarget::Memory, QueryTarget::Schema],
            cues: vec![Cue::Text(TextCue { text: cue })],
            constraints,
            ..Default::default()
        },
        exploration: Default::default(),
        resources: Default::default(),
        result_need: ResultNeed {
            limit,
            ..Default::default()
        },
        effort: CognitiveEffort::Light,
        capabilities: Default::default(),
        diagnostics: Default::default(),
    }
}
