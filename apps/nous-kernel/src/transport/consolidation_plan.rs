use super::*;
use nous_core::*;
use sqlx::Row;
use std::collections::BTreeSet;

impl KernelService {
    pub(super) async fn consolidation_context(&self, plan: &mut k::MaintenancePlan) -> Result<()> {
        let subject = SubjectId(id(&plan.subject_id)?);
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
        let cue: String = text.chars().take(2048).collect();
        let query = consolidation_query(subject, cue);
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
        let kinds:std::collections::BTreeMap<uuid::Uuid,String>=sqlx::query("SELECT schema_revision_id,formation_kind FROM cognitive_schema_revisions WHERE schema_revision_id=ANY($1::uuid[])").bind(schema_ids).fetch_all(self.0.store.pool()).await.map_err(nous_persistence::database_error)?.into_iter().map(|row|(row.get("schema_revision_id"),row.get("formation_kind"))).collect();
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
            let text: String = hit
                .representation
                .unwrap_or_default()
                .chars()
                .take(4096)
                .collect();
            let formation = if let CognitiveRef::CognitiveSchemaRevision(id) = &reference {
                kinds.get(&id.0).cloned().unwrap_or_default()
            } else {
                hit.formation_mode.unwrap_or_default()
            };
            let uses = self.consolidation_use_summary(subject, &reference).await?;
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
        self.finish_consolidation_catalog(plan, subject, &scopes, entities)
            .await
    }

    async fn finish_consolidation_catalog(
        &self,
        plan: &mut k::MaintenancePlan,
        subject: SubjectId,
        scopes: &[CognitiveRef],
        entities: BTreeSet<String>,
    ) -> Result<()> {
        let memory = self.require_memory()?;
        let supports = memory
            .consolidation_catalog_supports(subject, scopes)
            .await?;
        let provenance = memory.provenance_summary(subject, &supports).await?;
        plan.support_catalog_partial = supports.len() > 512;
        plan.provenance_roots_partial = provenance.roots.len() > 512;
        plan.provenance_roots = provenance
            .roots
            .into_iter()
            .take(512)
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
            .take(512);
        plan.supports = bounded
            .map(|support| k::SupportCatalogEntry {
                key: support.canonical_key(),
                support: Some(support_proto(support)),
            })
            .collect();
        plan.entities = entities
            .into_iter()
            .take(128)
            .enumerate()
            .map(|(index, entity_ref)| k::ConsolidationEntity {
                key: format!("entity{index}"),
                entity_ref,
            })
            .collect();
        plan.max_consolidation_actions =
            self.0
                .configuration
                .snapshot_for_subject(subject)?
                .get(nous_memory::CONSOLIDATION_MAX_ACTIONS)? as u32;
        Ok(())
    }
    async fn consolidation_use_summary(
        &self,
        subject: SubjectId,
        reference: &CognitiveRef,
    ) -> Result<Vec<k::UseSummary>> {
        let (kind, value) = reference_parts(reference);
        let rows=sqlx::query("SELECT use_kind,COUNT(*) AS count,MAX(occurred_at) AS last_used FROM cognitive_use_events WHERE subject_id=$1 AND ref_kind=$2 AND ref_value=$3 AND use_kind<>'presented' GROUP BY use_kind ORDER BY use_kind")
            .bind(subject.0).bind(kind).bind(value).fetch_all(self.0.store.pool()).await.map_err(nous_persistence::database_error)?;
        Ok(rows
            .into_iter()
            .map(|row| k::UseSummary {
                kind: row.get("use_kind"),
                count: row.get::<i64, _>("count") as u64,
                last_used_at: Some(timestamp(row.get("last_used"))),
            })
            .collect())
    }
}

fn consolidation_query(subject: SubjectId, cue: String) -> CognitiveQuery {
    let constraints = QueryConstraints::default();
    CognitiveQuery {
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
            limit: 16,
            ..Default::default()
        },
        effort: CognitiveEffort::Light,
        capabilities: Default::default(),
        diagnostics: Default::default(),
    }
}
